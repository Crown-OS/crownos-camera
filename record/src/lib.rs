//! Video recording: processed frames and microphone samples, encoded and muxed
//! as they arrive.

mod audio;
mod clock;
mod error;
mod mux;
mod video;

use std::path::{Path, PathBuf};

use crownos_config::schema::VideoContainer;
use ffmpeg_next::{Packet, codec};

use self::audio::AudioTrack;
pub use self::audio::{Channels, SAMPLE_RATE};
use self::clock::{FramePacer, RecordingClock};
pub use self::error::RecordError;
pub use self::mux::extension;
use self::mux::{Muxer, PendingPackets};
pub use self::video::VideoSettings;
use self::video::{BACKENDS, VideoEncoder};
use crownos_camera_frame::VideoFrame;

/// An encoder whose stream settings the muxer copies.
pub trait EncodedStream {
    fn context(&self) -> &codec::Context;
}

/// Where encoded packets go.
pub trait PacketSink {
    fn video(&mut self, packet: Packet) -> Result<(), RecordError>;
    fn audio(&mut self, packet: Packet) -> Result<(), RecordError>;
}

/// Everything about a recording known before its first frame.
#[derive(Debug, Clone)]
pub struct Recording {
    pub path: PathBuf,
    pub container: VideoContainer,
    pub video: VideoSettings,
    /// The microphone's layout, or `None` for a silent recording.
    pub audio: Option<Channels>,
}

/// A recording in progress.
pub struct Recorder {
    encoder: Box<dyn VideoEncoder>,
    audio: Option<AudioTrack>,
    muxer: Muxer,
    clock: RecordingClock,
    pacer: FramePacer,
    path: PathBuf,
    encoder_name: &'static str,
}

impl Recorder {
    /// Opens the best encoder that takes `first` and encodes it.
    pub fn start(recording: &Recording, first: &VideoFrame<'_>) -> Result<Self, RecordError> {
        ffmpeg_next::init()?;
        let clock = RecordingClock::starting_at(first.captured_at_ns);
        let mut pending = PendingPackets::default();
        let (encoder, encoder_name) = open_first(first, &recording.video, &mut pending)?;
        let audio = recording.audio.map(AudioTrack::open).transpose()?;
        let mut muxer = Muxer::create(
            &recording.path,
            recording.container,
            encoder.as_ref(),
            audio.as_ref().map(|track| track as &dyn EncodedStream),
        )?;
        pending.replay(&mut muxer)?;
        let mut pacer = FramePacer::new(recording.video.frames_per_second);
        pacer.accept(0);
        Ok(Self {
            encoder,
            audio,
            muxer,
            clock,
            pacer,
            path: recording.path.clone(),
            encoder_name,
        })
    }

    pub fn video(&mut self, frame: &VideoFrame<'_>) -> Result<(), RecordError> {
        let timestamp = self.clock.microseconds_at(frame.captured_at_ns);
        if self.pacer.accept(timestamp) {
            self.encoder.encode(frame, timestamp, &mut self.muxer)?;
        }
        Ok(())
    }

    /// Adds interleaved microphone samples captured at `captured_at_ns`.
    pub fn audio(&mut self, samples: &[f32], captured_at_ns: u64) -> Result<(), RecordError> {
        let offset = self.clock.offset_ns(captured_at_ns);
        match &mut self.audio {
            Some(track) => track.push(samples, offset, &mut self.muxer),
            None => Ok(()),
        }
    }

    /// Flushes both encoders and closes the file.
    pub fn finish(mut self) -> Result<PathBuf, RecordError> {
        self.encoder.finish(&mut self.muxer)?;
        if let Some(track) = &mut self.audio {
            track.finish(&mut self.muxer)?;
        }
        self.muxer.finish()?;
        Ok(self.path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Which encoder is recording: "VA-API", "NVENC" or "software".
    pub fn encoder_name(&self) -> &'static str {
        self.encoder_name
    }
}

fn open_first(
    first: &VideoFrame<'_>,
    settings: &VideoSettings,
    pending: &mut PendingPackets,
) -> Result<(Box<dyn VideoEncoder>, &'static str), RecordError> {
    let mut last_failure = RecordError::NoUsableEncoder;
    for backend in BACKENDS
        .into_iter()
        .filter(|backend| backend.accepts(&first.pixels))
    {
        let mut attempt = PendingPackets::default();
        let opened = backend.open(first, settings).and_then(|mut encoder| {
            encoder.encode(first, 0, &mut attempt)?;
            Ok(encoder)
        });
        match opened {
            Ok(encoder) => {
                *pending = attempt;
                return Ok((encoder, backend.name()));
            }
            Err(error) => last_failure = error,
        }
    }
    Err(last_failure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crownos_camera_frame::{FramePixels, MemoryFrame, PixelFormat};
    use crownos_config::schema::{RecordingQuality, VideoCodec};

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 48;
    const FRAME_NS: u64 = 33_333_333;

    fn frame(pixels: &[u8], index: u64) -> VideoFrame<'_> {
        VideoFrame {
            pixels: FramePixels::Memory(MemoryFrame {
                bytes: pixels,
                width: WIDTH,
                height: HEIGHT,
                stride: WIDTH * 4,
                format: PixelFormat::Rgba,
            }),
            slot: 0,
            captured_at_ns: 1_000_000_000 + index * FRAME_NS,
        }
    }

    #[test]
    fn records_frames_and_sound_into_an_mp4() -> Result<(), RecordError> {
        let path = std::env::temp_dir().join(format!("crownos-camera-{}.mp4", std::process::id()));
        let recording = Recording {
            path: path.clone(),
            container: VideoContainer::Mp4,
            video: VideoSettings {
                codec: VideoCodec::H264,
                quality: RecordingQuality::Low,
                frames_per_second: 30,
            },
            audio: Some(Channels::Mono),
        };
        let pixels = vec![200u8; (WIDTH * HEIGHT * 4) as usize];
        let tone: Vec<f32> = (0..1_600).map(|n| (n as f32 * 0.05).sin() * 0.2).collect();

        let mut recorder = Recorder::start(&recording, &frame(&pixels, 0))?;
        for index in 1..30 {
            recorder.video(&frame(&pixels, index))?;
            recorder.audio(&tone, 1_000_000_000 + index * FRAME_NS)?;
        }
        let written = recorder.finish()?;

        let streams: Vec<_> = ffmpeg_next::format::input(&written)?
            .streams()
            .map(|stream| stream.parameters().medium())
            .collect();
        std::fs::remove_file(&written)?;
        assert_eq!(
            streams,
            [
                ffmpeg_next::media::Type::Video,
                ffmpeg_next::media::Type::Audio
            ]
        );
        Ok(())
    }
}
