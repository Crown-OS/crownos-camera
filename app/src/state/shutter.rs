use std::time::Duration;

use crownos_config::schema::CaptureMode;
use crownui::prelude::{Cx, Event, Runtime};

use super::camera_state::CameraState;
use super::capture_mode::mode_at;
use super::recording_phase::RecordingPhase;
use crate::session::SessionCommand;
use crate::util::open_in_preview::open_in_preview;

const FLASH: Duration = Duration::from_millis(90);

impl CameraState {
    pub fn capture_mode(self, runtime: &mut Runtime) -> CaptureMode {
        mode_at(self.mode.get(runtime))
    }

    /// Takes a photo, or starts or stops a video, by the mode the toggle is in.
    pub fn press_shutter(self, cx: &mut Cx) {
        if !self.is_live(cx) {
            return;
        }
        match (self.capture_mode(cx), self.recording.get(cx)) {
            (CaptureMode::Photo, _) => self.take_photo(cx),
            (CaptureMode::Video, RecordingPhase::Idle) => self.start_recording(cx),
            (CaptureMode::Video, RecordingPhase::Recording { .. }) => self.stop_recording(cx),
            (CaptureMode::Video, RecordingPhase::Starting | RecordingPhase::Saving) => {}
        }
    }

    fn take_photo(self, cx: &mut Cx) {
        let path = self.settings.with(cx, |settings| settings.new_photo_path());
        self.with_session(cx, |session| session.send(SessionCommand::TakePhoto(path)));
        self.flash.set(cx, true);
        cx.spawn(async { tokio::time::sleep(FLASH).await }, move |cx, ()| {
            self.flash.set(cx, false);
        });
    }

    fn start_recording(self, cx: &mut Cx) {
        let (recording, microphone) = self.settings.with(cx, |settings| {
            (settings.new_recording(), settings.microphone())
        });
        self.with_session(cx, |session| {
            session.send(SessionCommand::StartRecording {
                recording,
                microphone,
            });
        });
        self.recording.set(cx, RecordingPhase::Starting);
    }

    fn stop_recording(self, cx: &mut Cx) {
        self.with_session(cx, |session| session.send(SessionCommand::StopRecording));
        self.recording.set(cx, RecordingPhase::Saving);
    }

    pub fn open_last_capture(self, cx: &mut Cx) {
        let Some(capture) = self.last_capture.get(cx) else {
            return;
        };
        if let Err(error) = open_in_preview(&capture.path) {
            self.tell(cx, format!("Preview could not open: {error}"));
        }
    }

    /// Keeps the window open until a recording in progress is saved.
    pub(super) fn close_requested(self, cx: &mut Cx, event: &mut Event<()>) {
        let has_session = self.session.with(cx, Option::is_some);
        if !has_session {
            return;
        }
        match self.recording.get(cx) {
            RecordingPhase::Idle => return,
            RecordingPhase::Starting | RecordingPhase::Recording { .. } => self.stop_recording(cx),
            RecordingPhase::Saving => {}
        }
        event.prevent_default();
        self.close_when_saved.update(cx, |close| *close = true);
    }
}
