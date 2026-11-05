use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;

use crate::util::rgba_image::RgbaImage;

/// Which camera session an event came from; events of a replaced session are stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionId(pub u64);

/// A photo or video written to disk, with a small picture of it.
#[derive(Debug, Clone)]
pub struct SavedCapture {
    pub path: PathBuf,
    pub thumbnail: RgbaImage,
}

#[derive(Debug)]
pub enum RecordingOutcome {
    Saved(SavedCapture),
    Failed(String),
    /// Stopped before the camera sent a frame, so there is no file.
    Discarded,
}

/// What a camera session tells the UI.
#[derive(Debug)]
pub enum SessionEvent {
    /// The newest frame, sent only once the UI has shown the previous one.
    Frame(RgbaImage),
    PhotoTaken(Result<SavedCapture, String>),
    RecordingStarted,
    RecordingEnded(RecordingOutcome),
    /// Something worth telling that does not stop the camera.
    Notice(String),
    /// The camera went away or could not start; the session is over.
    Stopped(String),
}

pub struct SessionMessage {
    pub session: SessionId,
    pub event: SessionEvent,
}

/// The session's end of the UI's mailbox.
#[derive(Clone)]
pub struct SessionEvents {
    session: SessionId,
    sender: UnboundedSender<SessionMessage>,
}

impl SessionEvents {
    pub fn new(session: SessionId, sender: UnboundedSender<SessionMessage>) -> Self {
        Self { session, sender }
    }

    pub fn send(&self, event: SessionEvent) {
        let _ = self.sender.send(SessionMessage {
            session: self.session,
            event,
        });
    }
}
