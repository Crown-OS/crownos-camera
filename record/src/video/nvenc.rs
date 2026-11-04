use crownos_config::schema::VideoCodec;
use ffmpeg_next::{Dictionary, encoder};

use super::{
    VideoBackend, VideoEncoder, VideoSettings, drain, even, ffmpeg_pixel, flush, open_encoder,
    video_frame,
};
use crate::{EncodedStream, PacketSink, RecordError};
use crownos_camera_frame::{FramePixels, VideoFrame};

/// NVIDIA's hardware encoder, fed packed RGB from memory and converting it on the GPU.
#[derive(Debug, Default, Clone, Copy)]
pub struct NvencBackend;

impl VideoBackend for NvencBackend {
    fn name(&self) -> &'static str {
        "NVENC"
    }

    fn accepts(&self, pixels: &FramePixels<'_>) -> bool {
        matches!(pixels, FramePixels::Memory(_))
    }

    fn open(
        &self,
        first: &VideoFrame<'_>,
        settings: &VideoSettings,
    ) -> Result<Box<dyn VideoEncoder>, RecordError> {
        let FramePixels::Memory(memory) = first.pixels else {
            return Err(RecordError::WrongBuffer);
        };
        let mut options = Dictionary::new();
        options.set("preset", "p4");
        options.set("tune", "hq");
        let encoder = open_encoder(
            encoder_name(settings.codec),
            ffmpeg_pixel(memory.format),
            (even(memory.width), even(memory.height)),
            settings,
            |_| {},
            options,
        )?;
        Ok(Box::new(NvencEncoder { encoder }))
    }
}

fn encoder_name(codec: VideoCodec) -> &'static str {
    match codec {
        VideoCodec::H264 => "h264_nvenc",
        VideoCodec::Hevc => "hevc_nvenc",
        VideoCodec::Av1 => "av1_nvenc",
    }
}

struct NvencEncoder {
    encoder: encoder::Video,
}

// SAFETY: the encoder is owned here and used from one thread at a time.
unsafe impl Send for NvencEncoder {}

impl EncodedStream for NvencEncoder {
    fn context(&self) -> &ffmpeg_next::codec::Context {
        &self.encoder
    }
}

impl VideoEncoder for NvencEncoder {
    fn encode(
        &mut self,
        frame: &VideoFrame<'_>,
        timestamp: i64,
        sink: &mut dyn PacketSink,
    ) -> Result<(), RecordError> {
        let FramePixels::Memory(memory) = frame.pixels else {
            return Err(RecordError::WrongBuffer);
        };
        self.encoder.send_frame(&video_frame(&memory, timestamp))?;
        drain(&mut self.encoder, sink)
    }

    fn finish(&mut self, sink: &mut dyn PacketSink) -> Result<(), RecordError> {
        flush(&mut self.encoder, sink)
    }
}
