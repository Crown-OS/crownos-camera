use crownos_config::schema::VideoCodec;
use ffmpeg_next::{Dictionary, encoder, format, frame};

use super::filter::FilterChain;
use super::{
    VideoBackend, VideoEncoder, VideoSettings, drain, even, ffmpeg_pixel, flush, open_encoder,
    video_frame,
};
use crate::{EncodedStream, PacketSink, RecordError};
use crownos_camera_frame::{FramePixels, MemoryFrame, VideoFrame};

/// CPU encoding from memory frames, for machines without a hardware encoder.
#[derive(Debug, Default, Clone, Copy)]
pub struct SoftwareBackend;

impl VideoBackend for SoftwareBackend {
    fn name(&self) -> &'static str {
        "software"
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
        let output_size = (even(memory.width), even(memory.height));
        let (name, options) = encoder_for(settings.codec);
        let encoder = open_encoder(
            name,
            format::Pixel::YUV420P,
            output_size,
            settings,
            |_| {},
            options,
        )?;
        Ok(Box::new(SoftwareEncoder {
            input: SoftwareInput::new(&memory, output_size)?,
            encoder,
            output_size,
            converted: frame::Video::empty(),
        }))
    }
}

fn encoder_for(codec: VideoCodec) -> (&'static str, Dictionary<'static>) {
    let mut options = Dictionary::new();
    let name = match codec {
        VideoCodec::H264 => {
            options.set("preset", "veryfast");
            "libx264"
        }
        VideoCodec::Hevc => {
            options.set("preset", "veryfast");
            "libx265"
        }
        VideoCodec::Av1 => {
            options.set("preset", "10");
            "libsvtav1"
        }
    };
    (name, options)
}

struct SoftwareInput {
    width: u32,
    height: u32,
    format: format::Pixel,
    filters: FilterChain,
}

impl SoftwareInput {
    fn new(frame: &MemoryFrame<'_>, (width, height): (u32, u32)) -> Result<Self, RecordError> {
        let format = ffmpeg_pixel(frame.format);
        let filters = FilterChain::new(
            format.into(),
            frame.width,
            frame.height,
            None,
            &format!("scale={width}:{height}:out_color_matrix=bt709:out_range=tv,format=yuv420p"),
        )?;
        Ok(Self {
            width: frame.width,
            height: frame.height,
            format,
            filters,
        })
    }

    fn accepts(&self, frame: &MemoryFrame<'_>) -> bool {
        (self.width, self.height, self.format)
            == (frame.width, frame.height, ffmpeg_pixel(frame.format))
    }
}

struct SoftwareEncoder {
    input: SoftwareInput,
    encoder: encoder::Video,
    output_size: (u32, u32),
    converted: frame::Video,
}

// SAFETY: every FFmpeg object here is owned by the encoder and used from one thread at a time.
unsafe impl Send for SoftwareEncoder {}

impl EncodedStream for SoftwareEncoder {
    fn context(&self) -> &ffmpeg_next::codec::Context {
        &self.encoder
    }
}

impl VideoEncoder for SoftwareEncoder {
    fn encode(
        &mut self,
        frame: &VideoFrame<'_>,
        timestamp: i64,
        sink: &mut dyn PacketSink,
    ) -> Result<(), RecordError> {
        let FramePixels::Memory(memory) = frame.pixels else {
            return Err(RecordError::WrongBuffer);
        };
        if !self.input.accepts(&memory) {
            self.input = SoftwareInput::new(&memory, self.output_size)?;
        }
        self.input.filters.push(&video_frame(&memory, timestamp))?;
        while self.input.filters.pull(&mut self.converted)? {
            self.encoder.send_frame(&self.converted)?;
            drain(&mut self.encoder, sink)?;
        }
        Ok(())
    }

    fn finish(&mut self, sink: &mut dyn PacketSink) -> Result<(), RecordError> {
        flush(&mut self.encoder, sink)
    }
}
