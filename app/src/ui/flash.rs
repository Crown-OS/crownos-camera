use crownui::prelude::{Curve, Styled, View, WHITE, bind, vstack};

use crate::state::CameraState;

const FLASH_OPACITY: f32 = 0.7;
const FADE_MILLIS: u64 = 160;

/// A white wash over the viewfinder the moment a photo is taken.
pub fn flash(state: CameraState) -> impl View {
    let flash = state.flash;
    vstack(())
        .bg(WHITE)
        .opacity(bind(move |runtime| {
            if flash.get(runtime) {
                FLASH_OPACITY
            } else {
                0.0
            }
        }))
        .transition(Curve::ease_in_out(FADE_MILLIS))
        .pointer_events_none()
}
