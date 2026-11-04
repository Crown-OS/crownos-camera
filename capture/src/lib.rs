//! Camera capture through PipeWire, which fronts libcamera and V4L2 alike.

mod decode;
mod format;
mod stream;

use thiserror::Error;

pub use self::decode::FrameDecoder;
pub use self::format::{CameraFormat, CameraRequest, Negotiated};
pub use self::stream::{CameraCapture, CapturedFrame};

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("PipeWire: {0}")]
    PipeWire(#[from] pipewire::Error),
    #[error("the camera offered no format this app can decode")]
    Negotiation,
    #[error("the camera stopped")]
    Stopped,
    #[error("JPEG: {0}")]
    Jpeg(#[from] turbojpeg::Error),
    #[error("FFmpeg: {0}")]
    Ffmpeg(#[from] ffmpeg_next::Error),
    #[error("a {expected}-byte frame arrived with {actual} bytes")]
    ShortFrame { expected: usize, actual: usize },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
