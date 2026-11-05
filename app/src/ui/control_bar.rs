use crownui::prelude::{Cx, Styled, View, hstack, spacer, vstack};

use super::last_capture_button::last_capture_button;
use super::mode_toggle::mode_toggle;
use super::shutter_button::shutter_button;
use crate::state::CameraState;

/// Each side of the shutter, equally wide so the shutter stays centered.
const SIDE_WIDTH: f32 = 96.0;

/// The mode toggle over the last capture and the shutter.
pub fn control_bar(cx: &mut Cx, state: CameraState) -> impl View + use<> {
    vstack((
        mode_toggle(state).self_center(),
        hstack((
            hstack((last_capture_button(cx, state),))
                .w(SIDE_WIDTH)
                .justify_center(),
            spacer(),
            shutter_button(cx, state),
            spacer(),
            hstack(()).w(SIDE_WIDTH),
        ))
        .items_center()
        .w_full()
        .max_w(520.0),
    ))
    .items_center()
    .gap(14.0)
    .pt(14.0)
    .pb(18.0)
    .px(16.0)
}
