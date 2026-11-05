use crownui::kit::{title_bar_spacer, use_theme, window_controls};
use crownui::prelude::{Cx, Styled, View, hstack, vstack};

use super::control_bar::control_bar;
use super::stage::stage;
use crate::state::CameraState;
use crate::util::tint::tint;

const TITLE_BAR_PADDING: f32 = 6.0;

/// The title bar, the viewfinder and the capture controls under it.
pub fn shell(cx: &mut Cx, state: CameraState) -> impl View + use<> {
    let theme = use_theme(cx);
    vstack((
        hstack((title_bar_spacer(), window_controls())).p(TITLE_BAR_PADDING),
        stage(state).flex_1(),
        control_bar(cx, state),
    ))
    .w_full()
    .h_full()
    .bg(tint(theme, |theme| theme.surface.bg))
}
