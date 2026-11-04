use std::path::Path;

use crownos_config::schema::VideoContainer;
use ffmpeg_next::{Packet, Rational, format};

use super::{EncodedStream, PacketSink, RecordError};

pub fn format_name(container: VideoContainer) -> &'static str {
    match container {
        VideoContainer::Mp4 => "mp4",
        VideoContainer::Mkv => "matroska",
    }
}

pub fn extension(container: VideoContainer) -> &'static str {
    match container {
        VideoContainer::Mp4 => "mp4",
        VideoContainer::Mkv => "mkv",
    }
}

#[derive(Debug, Clone, Copy)]
struct Track {
    index: usize,
    encoder_time_base: Rational,
}

/// Interleaves encoded packets into a container file.
pub struct Muxer {
    output: format::context::Output,
    video: Track,
    audio: Option<Track>,
}

// SAFETY: the output context is owned here and used from one thread at a time.
unsafe impl Send for Muxer {}

impl Muxer {
    pub fn create(
        path: &Path,
        container: VideoContainer,
        video: &dyn EncodedStream,
        audio: Option<&dyn EncodedStream>,
    ) -> Result<Self, RecordError> {
        let mut output = format::output_as(path, format_name(container))?;
        let mut add = |encoded: &dyn EncodedStream| -> Result<Track, RecordError> {
            let mut stream = output.add_stream_with(encoded.context())?;
            stream.set_time_base(encoded.context().time_base());
            Ok(Track {
                index: stream.index(),
                encoder_time_base: encoded.context().time_base(),
            })
        };
        let video = add(video)?;
        let audio = audio.map(&mut add).transpose()?;
        output.write_header()?;
        Ok(Self {
            output,
            video,
            audio,
        })
    }

    pub fn finish(mut self) -> Result<(), RecordError> {
        self.output.write_trailer()?;
        Ok(())
    }

    fn write(&mut self, track: Track, mut packet: Packet) -> Result<(), RecordError> {
        let stream_time_base = self
            .output
            .stream(track.index)
            .map_or(track.encoder_time_base, |stream| stream.time_base());
        packet.set_stream(track.index);
        packet.rescale_ts(track.encoder_time_base, stream_time_base);
        packet.write_interleaved(&mut self.output)?;
        Ok(())
    }
}

impl PacketSink for Muxer {
    fn video(&mut self, packet: Packet) -> Result<(), RecordError> {
        self.write(self.video, packet)
    }

    fn audio(&mut self, packet: Packet) -> Result<(), RecordError> {
        match self.audio {
            Some(track) => self.write(track, packet),
            None => Ok(()),
        }
    }
}

/// Holds the first frame's packets until the muxer exists.
#[derive(Default)]
pub struct PendingPackets {
    video: Vec<Packet>,
}

impl PendingPackets {
    pub fn replay(self, sink: &mut dyn PacketSink) -> Result<(), RecordError> {
        self.video
            .into_iter()
            .try_for_each(|packet| sink.video(packet))
    }
}

impl PacketSink for PendingPackets {
    fn video(&mut self, packet: Packet) -> Result<(), RecordError> {
        self.video.push(packet);
        Ok(())
    }

    fn audio(&mut self, _: Packet) -> Result<(), RecordError> {
        Ok(())
    }
}
