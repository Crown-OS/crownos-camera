use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingPhase {
    Idle,
    /// Asked to record; waiting for the encoder to take the first frame.
    Starting,
    Recording {
        since: Instant,
    },
    /// Asked to stop; waiting for the file to close.
    Saving,
}

impl RecordingPhase {
    pub fn is_idle(self) -> bool {
        self == Self::Idle
    }

    pub fn is_recording(self) -> bool {
        matches!(self, Self::Recording { .. })
    }
}
