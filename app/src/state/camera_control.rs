use crownui::prelude::Cx;

use super::camera_state::CameraState;
use super::viewfinder::Viewfinder;
use crate::session::{CameraSession, SessionEvents, SessionId};

impl CameraState {
    /// Streams from the configured camera, or the default one, in a new session.
    pub(super) fn open_camera(self, cx: &mut Cx) {
        self.close_session(cx);
        let session = SessionId(self.current_session(cx).0 + 1);
        self.session_id.update(cx, |id| *id = session);
        let Some(mailbox) = self.session_mailbox.with(cx, Clone::clone) else {
            return;
        };
        let (camera, request) = self.settings.with(cx, |settings| {
            (
                settings.preferred_camera().map(str::to_owned),
                settings.request(),
            )
        });
        let started = CameraSession::start(camera, request, SessionEvents::new(session, mailbox));
        match started {
            Ok(started) => {
                self.session.update(cx, |slot| *slot = Some(started));
                self.viewfinder.set(cx, Viewfinder::Starting);
            }
            Err(error) => self.viewfinder.set(
                cx,
                Viewfinder::Unavailable(format!("The camera could not start: {error}")),
            ),
        }
    }

    /// Opens the camera again after it failed or went away.
    pub fn retry(self, cx: &mut Cx) {
        self.open_camera(cx);
    }

    /// Stops the camera, saving a recording in progress, and clears the viewfinder.
    pub(super) fn close_session(self, cx: &mut Cx) {
        let closed = self.session.update(cx, Option::take);
        drop(closed);
        self.feed.clear(cx);
    }
}
