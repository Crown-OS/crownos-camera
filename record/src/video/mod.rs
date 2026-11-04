//! Encoding processed camera frames to compressed video.
//!
//! [`VaapiBackend`] imports GPU frames without a copy, [`NvencBackend`] hands
//! memory frames to NVIDIA's encoder, which converts them on the GPU, and
//! [`SoftwareBackend`] is the fallback for everything else.

mod filter;
mod hardware;
mod nvenc;
mod software;
mod vaapi;

use crownos_config::schema::{RecordingQuality, VideoCodec};
use ffmpeg_next::{Dictionary, Packet, Rational, codec, encoder, format, frame};

pub use self::nvenc::NvencBackend;
pub use self::software::SoftwareBackend;
pub use self::vaapi::VaapiBackend;
use super::clock::RecordingClock;
use super::{EncodedStream, PacketSink, RecordError};
use crownos_camera_frame::{FramePixels, MemoryFrame, PixelFormat, VideoFrame};

/// How the encoder should compress the video.
#[derive(Debug, Clone, Copy)]
pub struct VideoSettings {
    pub codec: VideoCodec,
    pub quality: RecordingQuality,
    pub frames_per_second: u32,
}

/// A family of encoders.
pub trait VideoBackend {
    fn name(&self) -> &'static str;

    /// Whether frames in `pixels`' kind of buffer can be encoded at all.
    fn accepts(&self, pixels: &FramePixels<'_>) -> bool;

    /// Opens an encoder sized for `first`, which it has not encoded yet.
    fn open(
        &self,
        first: &VideoFrame<'_>,
        settings: &VideoSettings,
    ) -> Result<Box<dyn VideoEncoder>, RecordError>;
}

/// A running video encoder.
pub trait VideoEncoder: EncodedStream + Send {
    /// Encodes `frame`, stamped `timestamp` microseconds into the recording.
    fn encode(
        &mut self,
        frame: &VideoFrame<'_>,
        timestamp: i64,
        sink: &mut dyn PacketSink,
    ) -> Result<(), RecordError>;

    /// Flushes the frames the encoder still holds.
    fn finish(&mut self, sink: &mut dyn PacketSink) -> Result<(), RecordError>;
}

/// Every backend, best first.
pub const BACKENDS: [&dyn VideoBackend; 3] = [&VaapiBackend, &NvencBackend, &SoftwareBackend];

/// Bits per pixel per frame for each quality step.
fn bits_per_pixel(quality: RecordingQuality) -> f64 {
    match quality {
        RecordingQuality::Low => 0.04,
        RecordingQuality::Medium => 0.08,
        RecordingQuality::High => 0.15,
        RecordingQuality::Lossless => 0.4,
    }
}

fn bit_rate(width: u32, height: u32, settings: &VideoSettings) -> usize {
    let pixels_per_second =
        f64::from(width) * f64::from(height) * f64::from(settings.frames_per_second);
    (pixels_per_second * bits_per_pixel(settings.quality)) as usize
}

/// Encoders need even dimensions.
fn even(size: u32) -> u32 {
    size & !1
}

/// Opens `name` for `width`×`height` frames of `pixel` with the shared rate control.
fn open_encoder(
    name: &'static str,
    pixel: format::Pixel,
    (width, height): (u32, u32),
    settings: &VideoSettings,
    configure: impl FnOnce(&mut encoder::video::Video),
    mut options: Dictionary<'_>,
) -> Result<encoder::Video, RecordError> {
    let codec = encoder::find_by_name(name).ok_or(RecordError::NoEncoder(name))?;
    let mut video = codec::Context::new_with_codec(codec).encoder().video()?;
    let rate = bit_rate(width, height, settings);
    video.set_width(width);
    video.set_height(height);
    video.set_format(pixel);
    video.set_time_base(Rational(1, RecordingClock::MICROSECONDS));
    video.set_frame_rate(Some(Rational(settings.frames_per_second as i32, 1)));
    video.set_gop(settings.frames_per_second * 2);
    video.set_max_b_frames(0);
    video.set_bit_rate(rate);
    video.set_max_bit_rate(rate * 3 / 2);
    options.set("bufsize", &(rate * 2).to_string());
    video.set_flags(codec::Flags::GLOBAL_HEADER);
    configure(&mut video);
    Ok(video.open_with(options)?)
}

/// Hands every packet `encoder` has ready to `sink`. Packets without a duration get one frame
/// interval, or containers would trim the last frame off the end.
fn drain(encoder: &mut encoder::Video, sink: &mut dyn PacketSink) -> Result<(), RecordError> {
    let rate = encoder.frame_rate();
    let frame_interval = match rate.numerator() {
        0 => 0,
        frames => {
            i64::from(RecordingClock::MICROSECONDS) * i64::from(rate.denominator())
                / i64::from(frames)
        }
    };
    loop {
        let mut packet = Packet::empty();
        match encoder.receive_packet(&mut packet) {
            Ok(()) => {
                if packet.duration() == 0 {
                    packet.set_duration(frame_interval);
                }
                sink.video(packet)?;
            }
            Err(
                ffmpeg_next::Error::Other {
                    errno: ffmpeg_next::util::error::EAGAIN,
                }
                | ffmpeg_next::Error::Eof,
            ) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
}

fn flush(encoder: &mut encoder::Video, sink: &mut dyn PacketSink) -> Result<(), RecordError> {
    encoder.send_eof()?;
    drain(encoder, sink)
}

/// The FFmpeg name of a packed 32-bit pixel layout.
fn ffmpeg_pixel(format: PixelFormat) -> format::Pixel {
    match format {
        PixelFormat::Bgrx => format::Pixel::BGRZ,
        PixelFormat::Bgra => format::Pixel::BGRA,
        PixelFormat::Rgbx => format::Pixel::RGBZ,
        PixelFormat::Rgba => format::Pixel::RGBA,
    }
}

/// `memory` copied into an FFmpeg frame of its own size and format.
fn video_frame(memory: &MemoryFrame<'_>, timestamp: i64) -> frame::Video {
    let mut pixels = frame::Video::new(ffmpeg_pixel(memory.format), memory.width, memory.height);
    let stride = pixels.stride(0);
    for (target, row) in pixels
        .data_mut(0)
        .chunks_exact_mut(stride)
        .zip(memory.rows())
    {
        target[..row.len()].copy_from_slice(row);
    }
    pixels.set_pts(Some(timestamp));
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_bit_rate_with_quality() {
        let settings = |quality| VideoSettings {
            codec: VideoCodec::H264,
            quality,
            frames_per_second: 30,
        };
        let low = bit_rate(1920, 1080, &settings(RecordingQuality::Low));
        let high = bit_rate(1920, 1080, &settings(RecordingQuality::High));
        assert!(low < high);
        assert!((2_000_000..4_000_000).contains(&low));
    }

    #[test]
    fn dimensions_round_down_to_even() {
        assert_eq!(even(1081), 1080);
        assert_eq!(even(1920), 1920);
    }
}
