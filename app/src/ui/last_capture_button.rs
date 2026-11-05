use crownui::kit::use_theme;
use crownui::prelude::{
    CursorIcon, Cx, Listeners, Role, Semantic, Spring, Styled, View, cover_image, keyed, vstack,
};

use crate::state::CameraState;
use crate::util::tint::tint;

const SIZE: f32 = 52.0;
const RADIUS: f32 = 12.0;
const HOVER_OPACITY: f32 = 0.85;

/// The newest photo or video in miniature; clicking it opens the file in Preview.
pub fn last_capture_button(cx: &mut Cx, state: CameraState) -> impl View + use<> {
    let theme = use_theme(cx);
    let last_capture = state.last_capture;
    keyed(
        move |runtime| {
            last_capture.with(runtime, |capture| {
                capture.as_ref().map(|capture| capture.thumbnail)
            })
        },
        move |_, thumbnail| match thumbnail {
            Some(thumbnail) => cover_image(thumbnail.image, thumbnail.size, RADIUS)
                .size(SIZE)
                .hover(|style| style.opacity(HOVER_OPACITY))
                .transition(Spring::snappy())
                .cursor(CursorIcon::Pointer)
                .role(Role::Button)
                .focusable(true)
                .label("Open the last capture")
                .on_click(move |cx, _| state.open_last_capture(cx))
                .boxed(),
            None => vstack(())
                .size(SIZE)
                .rounded(RADIUS)
                .bg(tint(theme, |theme| theme.surface.sunken))
                .boxed(),
        },
    )
}
