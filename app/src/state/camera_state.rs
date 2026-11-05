use std::time::Duration;

use crownui::kit::{Toast, Toasts, icons, provide_toasts};
use crownui::prelude::{Cx, Runtime, Signal, Store};
use tokio::sync::mpsc::UnboundedSender;

use super::capture_mode::mode_index;
use super::last_capture::LastCapture;
use super::recording_phase::RecordingPhase;
use super::viewfinder::Viewfinder;
use crate::session::{CameraSession, SessionId, SessionMessage};
use crate::settings::Settings;
use crate::util::mailbox::mailbox;
use crate::viewfinder::ViewfinderFeed;

/// The window's state, shared by every view.
#[derive(Clone, Copy)]
pub struct CameraState {
    pub settings: Store<Settings>,
    pub viewfinder: Signal<Viewfinder>,
    pub feed: ViewfinderFeed,
    /// The index of the capture mode in [`MODES`](super::capture_mode::MODES).
    pub mode: Signal<usize>,
    pub recording: Signal<RecordingPhase>,
    pub elapsed: Signal<Duration>,
    pub last_capture: Signal<Option<LastCapture>>,
    /// Raised for a moment when a photo is taken.
    pub flash: Signal<bool>,
    pub(super) session: Store<Option<CameraSession>>,
    pub(super) session_id: Store<SessionId>,
    pub(super) session_mailbox: Store<Option<UnboundedSender<SessionMessage>>>,
    pub(super) close_when_saved: Store<bool>,
    toasts: Toasts,
}

impl CameraState {
    /// Builds the state and opens the configured camera, or the default one.
    pub fn new(cx: &mut Cx, settings: Settings) -> Self {
        let mode = mode_index(settings.initial_mode());
        let state = Self {
            settings: cx.store(settings),
            viewfinder: cx.signal(Viewfinder::Starting),
            feed: ViewfinderFeed::new(cx),
            mode: cx.signal(mode),
            recording: cx.signal(RecordingPhase::Idle),
            elapsed: cx.signal(Duration::ZERO),
            last_capture: cx.signal(None),
            flash: cx.signal(false),
            session: cx.store(None),
            session_id: cx.store(SessionId(0)),
            session_mailbox: cx.store(None),
            close_when_saved: cx.store(false),
            toasts: provide_toasts(cx),
        };
        let sessions = mailbox(cx, move |cx, message| state.receive(cx, message));
        state
            .session_mailbox
            .update(cx, |mailbox| *mailbox = Some(sessions));
        cx.on_close_requested(move |cx, event| state.close_requested(cx, event));
        state.open_camera(cx);
        state
    }

    pub fn tell(self, cx: &mut Cx, message: impl Into<String>) {
        self.toasts
            .show_with(cx, Toast::new(message).icon(icons::CIRCLE_ALERT));
    }

    pub fn is_live(self, runtime: &mut Runtime) -> bool {
        self.viewfinder
            .with(runtime, |viewfinder| *viewfinder == Viewfinder::Live)
    }

    pub(super) fn with_session(self, runtime: &Runtime, act: impl FnOnce(&CameraSession)) {
        self.session
            .with(runtime, |session| session.as_ref().map(act));
    }

    pub(super) fn current_session(self, runtime: &Runtime) -> SessionId {
        self.session_id.with(runtime, |id| *id)
    }
}
