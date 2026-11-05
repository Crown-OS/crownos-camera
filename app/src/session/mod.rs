mod command;
mod event;
mod frame_sinks;
mod handle;
mod recording;
mod worker;

pub use command::SessionCommand;
pub use event::{
    RecordingOutcome, SavedCapture, SessionEvent, SessionEvents, SessionId, SessionMessage,
};
pub use handle::CameraSession;

/// The longer side of a capture's thumbnail, in pixels.
const THUMBNAIL_SIDE: u32 = 192;
