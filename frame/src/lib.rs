//! Camera frames as the processing pipeline hands them over: packed 32-bit
//! pixels, in memory or in a GPU buffer, stamped on the PipeWire graph clock.

use std::os::fd::OwnedFd;

pub const BYTES_PER_PIXEL: u32 = 4;

/// The byte order of a 32-bit pixel in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Bgrx,
    Bgra,
    Rgbx,
    Rgba,
}

impl PixelFormat {
    const ALL: [Self; 4] = [Self::Bgrx, Self::Bgra, Self::Rgbx, Self::Rgba];

    pub const fn drm_fourcc(self) -> u32 {
        u32::from_le_bytes(*match self {
            Self::Bgrx => b"XR24",
            Self::Bgra => b"AR24",
            Self::Rgbx => b"XB24",
            Self::Rgba => b"AB24",
        })
    }

    pub fn from_drm_fourcc(fourcc: u32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|format| format.drm_fourcc() == fourcc)
    }
}

/// Pixels in system memory, rows `stride` bytes apart.
#[derive(Debug, Clone, Copy)]
pub struct MemoryFrame<'frame> {
    pub bytes: &'frame [u8],
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
}

impl MemoryFrame<'_> {
    pub fn rows(&self) -> impl Iterator<Item = &[u8]> {
        let length = (self.width * BYTES_PER_PIXEL) as usize;
        self.bytes
            .chunks(self.stride as usize)
            .take(self.height as usize)
            .map(move |row| &row[..length.min(row.len())])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlaneLayout {
    pub offset: u32,
    pub stride: u32,
}

/// A GPU buffer shared as a dma-buf.
#[derive(Debug)]
pub struct DmabufFrame {
    pub fd: OwnedFd,
    pub width: u32,
    pub height: u32,
    pub fourcc: u32,
    pub modifier: u64,
    pub planes: Vec<PlaneLayout>,
}

#[derive(Debug, Clone, Copy)]
pub enum FramePixels<'frame> {
    Memory(MemoryFrame<'frame>),
    Gpu(&'frame DmabufFrame),
}

impl FramePixels<'_> {
    pub fn size(&self) -> (u32, u32) {
        match self {
            Self::Memory(frame) => (frame.width, frame.height),
            Self::Gpu(frame) => (frame.width, frame.height),
        }
    }
}

/// One processed frame.
#[derive(Debug, Clone, Copy)]
pub struct VideoFrame<'frame> {
    pub pixels: FramePixels<'frame>,
    /// The pool slot holding a GPU frame, stable for the life of the stream.
    pub slot: usize,
    /// When the frame was captured, in nanoseconds on the PipeWire graph clock.
    pub captured_at_ns: u64,
}
