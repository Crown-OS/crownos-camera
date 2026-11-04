//! The soundtrack: interleaved float samples from the microphone, encoded to Opus.

use ffmpeg_next::format::{Sample, sample};
use ffmpeg_next::{ChannelLayout, Packet, Rational, codec, encoder, frame};

use super::{EncodedStream, PacketSink, RecordError};

pub const SAMPLE_RATE: u32 = 48_000;
const BIT_RATE: usize = 128_000;
const PACKED_FLOAT: Sample = Sample::F32(sample::Type::Packed);
const NANOSECONDS: i128 = 1_000_000_000;

/// Mono or stereo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channels {
    Mono,
    Stereo,
}

impl Channels {
    const fn count(self) -> usize {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }

    fn layout(self) -> ChannelLayout {
        match self {
            Self::Mono => ChannelLayout::MONO,
            Self::Stereo => ChannelLayout::STEREO,
        }
    }
}

/// Opus through libopus, fed samples in whatever chunks the microphone delivers.
pub struct AudioTrack {
    encoder: encoder::Audio,
    frame: frame::Audio,
    channels: usize,
    pending: Vec<f32>,
    /// Samples per channel from the start of the recording to `pending[0]`.
    position: Option<i64>,
}

// SAFETY: the encoder and its frame are owned here and used from one thread at a time.
unsafe impl Send for AudioTrack {}

impl AudioTrack {
    pub fn open(channels: Channels) -> Result<Self, RecordError> {
        let codec = encoder::find_by_name("libopus").ok_or(RecordError::NoEncoder("libopus"))?;
        let mut audio = codec::Context::new_with_codec(codec).encoder().audio()?;
        audio.set_rate(SAMPLE_RATE as i32);
        audio.set_format(PACKED_FLOAT);
        audio.set_channel_layout(channels.layout());
        audio.set_bit_rate(BIT_RATE);
        audio.set_time_base(Rational(1, SAMPLE_RATE as i32));
        audio.set_flags(codec::Flags::GLOBAL_HEADER);
        let encoder = audio.open()?;
        let frame_samples = encoder.frame_size() as usize;
        let frame = frame::Audio::new(PACKED_FLOAT, frame_samples, channels.layout());
        Ok(Self {
            encoder,
            frame,
            channels: channels.count(),
            pending: Vec::with_capacity(frame_samples * channels.count() * 2),
            position: None,
        })
    }

    fn frame_len(&self) -> usize {
        self.encoder.frame_size() as usize * self.channels
    }

    /// Adds `samples` captured `offset_ns` after the first video frame. Sound
    /// from before the first frame is dropped.
    pub fn push(
        &mut self,
        samples: &[f32],
        offset_ns: i128,
        sink: &mut dyn PacketSink,
    ) -> Result<(), RecordError> {
        let samples = match self.position {
            Some(_) => samples,
            None => {
                let start = offset_ns * i128::from(SAMPLE_RATE) / NANOSECONDS;
                let skipped = usize::try_from(-start.min(0)).unwrap_or(usize::MAX);
                let Some(kept) = samples.get(skipped.saturating_mul(self.channels)..) else {
                    return Ok(());
                };
                self.position = Some(start.max(0) as i64);
                kept
            }
        };
        self.pending.extend_from_slice(samples);
        let frame_len = self.frame_len();
        let mut consumed = 0;
        while self.pending.len() - consumed >= frame_len {
            self.encode_frame(consumed, sink)?;
            consumed += frame_len;
        }
        self.pending.drain(..consumed);
        Ok(())
    }

    /// Pads the last partial frame with silence and flushes the encoder.
    pub fn finish(&mut self, sink: &mut dyn PacketSink) -> Result<(), RecordError> {
        if !self.pending.is_empty() {
            self.pending.resize(self.frame_len(), 0.0);
            self.encode_frame(0, sink)?;
            self.pending.clear();
        }
        self.encoder.send_eof()?;
        self.drain(sink)
    }

    fn encode_frame(&mut self, start: usize, sink: &mut dyn PacketSink) -> Result<(), RecordError> {
        let frame_len = self.frame_len();
        let samples = &self.pending[start..start + frame_len];
        for (target, sample) in self
            .frame
            .data_mut(0)
            .chunks_exact_mut(size_of::<f32>())
            .zip(samples)
        {
            target.copy_from_slice(&sample.to_ne_bytes());
        }
        let position = self.position.unwrap_or_default();
        self.frame.set_pts(Some(position));
        self.position = Some(position + self.encoder.frame_size() as i64);
        self.encoder.send_frame(&self.frame)?;
        self.drain(sink)
    }

    fn drain(&mut self, sink: &mut dyn PacketSink) -> Result<(), RecordError> {
        loop {
            let mut packet = Packet::empty();
            match self.encoder.receive_packet(&mut packet) {
                Ok(()) => sink.audio(packet)?,
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
}

impl EncodedStream for AudioTrack {
    fn context(&self) -> &codec::Context {
        &self.encoder
    }
}
