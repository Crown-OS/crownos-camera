use crownui::kit::{button, icons, spinner, toast_host};
use crownui::prelude::{BLACK, Cx, Listeners, Styled, View, ViewSeq, keyed, vstack, zstack};

use super::empty_state::empty_state;
use super::flash::flash;
use super::recording_timer::recording_timer;
use crate::state::{CameraState, Viewfinder};
use crate::viewfinder::viewfinder_view;

const STAGE_RADIUS: f32 = 14.0;
const STAGE_MARGIN: f32 = 8.0;

/// The live picture, or why there is none, with toasts raised over it.
pub fn stage(state: CameraState) -> impl View + Styled {
    let content = keyed(
        move |runtime| state.viewfinder.get(runtime),
        move |cx, phase| match phase {
            Viewfinder::Starting => centered(spinner()).bg(BLACK).boxed(),
            Viewfinder::Live => live(cx, state).boxed(),
            Viewfinder::Unavailable(reason) => centered((
                empty_state(cx, icons::CAMERA_OFF, "Camera unavailable", reason),
                button("Try again")
                    .self_center()
                    .on_click(move |cx, _| state.retry(cx)),
            ))
            .boxed(),
        },
    )
    .w_full()
    .h_full();
    toast_host(zstack((content, flash(state), recording_timer(state))))
        .mx(STAGE_MARGIN)
        .rounded(STAGE_RADIUS)
        .overflow_hidden()
}

fn live(cx: &mut Cx, state: CameraState) -> impl View + use<> {
    let mirrored = state
        .settings
        .with(cx, |settings| settings.mirror_preview());
    viewfinder_view(state.feed, mirrored).flex_1()
}

fn centered(content: impl ViewSeq) -> impl View + Styled {
    vstack(content)
        .gap(16.0)
        .p(24.0)
        .flex_1()
        .items_center()
        .justify_center()
}
