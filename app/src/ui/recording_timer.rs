use crownui::kit::use_theme;
use crownui::prelude::{
    Role, Semantic, Styled, View, WHITE, bind, component, hstack, rgba, show, text, vstack,
};

use crate::state::CameraState;
use crate::util::duration_text::duration_text;
use crate::util::tint::tint;

const DOT_DIAMETER: f32 = 10.0;
const TOP_OFFSET: f32 = 14.0;
const SCRIM: u32 = 0x0000_008c;

/// A pill with the time recorded so far, over the top of the viewfinder while recording.
pub fn recording_timer(state: CameraState) -> impl View {
    let recording = state.recording;
    vstack((show(
        bind(move |runtime| recording.get(runtime).is_recording()),
        move || pill(state),
        || vstack(()),
    ),))
    .items_center()
    .pt(TOP_OFFSET)
    .pointer_events_none()
}

fn pill(state: CameraState) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let elapsed = state.elapsed;
        hstack((
            vstack(())
                .size(DOT_DIAMETER)
                .rounded(DOT_DIAMETER / 2.0)
                .bg(tint(theme, |theme| theme.status.danger)),
            text(bind(move |runtime| duration_text(elapsed.get(runtime))))
                .font_semibold()
                .text_color(WHITE)
                .role(Role::Text)
                .label("Recording time"),
        ))
        .items_center()
        .gap(8.0)
        .px(12.0)
        .py(6.0)
        .rounded(999.0)
        .bg(rgba(SCRIM))
    })
}
