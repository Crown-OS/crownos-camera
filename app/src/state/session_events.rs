use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use crownui::prelude::Cx;

use super::camera_state::CameraState;
use super::last_capture::LastCapture;
use super::recording_phase::RecordingPhase;
use super::viewfinder::Viewfinder;
use crate::session::{CameraSession, RecordingOutcome, SavedCapture, SessionEvent, SessionMessage};
use crate::util::repeat_every::repeat_every;
use crate::util::rgba_image::RgbaImage;
use crate::util::shown_image::ShownImage;

const ELAPSED_TICK: Duration = Duration::from_millis(250);

impl CameraState {
    /// Applies what a camera session reports. Frames and failures of a session that was replaced
    /// are dropped; finished captures always count.
    pub(super) fn receive(self, cx: &mut Cx, message: SessionMessage) {
        let current = message.session == self.current_session(cx);
        match message.event {
            SessionEvent::Frame(image) if current => self.show_frame(cx, image),
            SessionEvent::PhotoTaken(Ok(saved)) => self.remember_capture(cx, saved),
            SessionEvent::PhotoTaken(Err(error)) => {
                self.tell(cx, format!("The photo could not be saved: {error}"));
            }
            SessionEvent::RecordingStarted if current => self.recording_started(cx),
            SessionEvent::RecordingEnded(outcome) => self.recording_ended(cx, outcome),
            SessionEvent::Notice(notice) if current => self.tell(cx, notice),
            SessionEvent::Stopped(reason) if current => {
                self.close_session(cx);
                self.viewfinder.set(cx, Viewfinder::Unavailable(reason));
            }
            _ => {}
        }
    }

    fn show_frame(self, cx: &mut Cx, frame: RgbaImage) {
        self.feed.push(cx, frame, move |cx| {
            self.with_session(cx, CameraSession::preview_shown);
        });
        if !self.is_live(cx) {
            self.viewfinder.set(cx, Viewfinder::Live);
        }
    }

    fn remember_capture(self, cx: &mut Cx, saved: SavedCapture) {
        let thumbnail = ShownImage::register(cx, saved.thumbnail);
        if let Some(previous) = self.last_capture.get(cx) {
            cx.release_image(previous.thumbnail.image);
        }
        self.last_capture.set(
            cx,
            Some(LastCapture {
                path: saved.path,
                thumbnail,
            }),
        );
    }

    fn recording_started(self, cx: &mut Cx) {
        let since = Instant::now();
        self.recording.set(cx, RecordingPhase::Recording { since });
        self.elapsed.set(cx, Duration::ZERO);
        repeat_every(cx, ELAPSED_TICK, move |cx| match self.recording.get(cx) {
            RecordingPhase::Recording { since } => {
                self.elapsed.set(cx, since.elapsed());
                ControlFlow::Continue(())
            }
            _ => ControlFlow::Break(()),
        });
    }

    fn recording_ended(self, cx: &mut Cx, outcome: RecordingOutcome) {
        self.recording.set(cx, RecordingPhase::Idle);
        match outcome {
            RecordingOutcome::Saved(saved) => self.remember_capture(cx, saved),
            RecordingOutcome::Failed(error) => {
                self.tell(cx, format!("The video could not be saved: {error}"));
            }
            RecordingOutcome::Discarded => {}
        }
        if self.close_when_saved.with(cx, |close| *close) {
            cx.close_window();
        }
    }
}
