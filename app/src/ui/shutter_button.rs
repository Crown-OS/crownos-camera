use crownos_config::schema::CaptureMode;
use crownui::kit::use_theme;
use crownui::prelude::{
    CursorIcon, Cx, Listeners, Role, Runtime, Semantic, Spring, Styled, View, bind, vstack,
};

use super::shortcuts::SPACE;
use crate::state::CameraState;
use crate::util::tint::tint;

const DIAMETER: f32 = 72.0;
const RING_WIDTH: f32 = 4.0;
const DISC_DIAMETER: f32 = 58.0;
/// While recording the disc shrinks to a rounded square, like a stop button.
const STOP_SCALE: f32 = 0.46;
const STOP_RADIUS: f32 = 14.0;
const PRESSED_SCALE: f32 = 0.94;
const DISABLED_OPACITY: f32 = 0.4;

/// The big round button: a white disc takes a photo, a red one starts a video and a red square
/// stops it. Space presses it.
pub fn shutter_button(cx: &mut Cx, state: CameraState) -> impl View + use<> {
    let theme = use_theme(cx);
    let recording = state.recording;
    let is_video = move |runtime: &mut Runtime| state.capture_mode(runtime) == CaptureMode::Video;
    let is_recording = move |runtime: &mut Runtime| recording.get(runtime).is_recording();
    let disc = vstack(())
        .size(DISC_DIAMETER)
        .rounded(bind(move |runtime| {
            if is_recording(runtime) {
                STOP_RADIUS
            } else {
                DISC_DIAMETER / 2.0
            }
        }))
        .scale(bind(move |runtime| {
            if is_recording(runtime) {
                STOP_SCALE
            } else {
                1.0
            }
        }))
        .bg(bind(move |runtime| {
            let video = is_video(runtime);
            theme.with(runtime, |theme| {
                if video {
                    theme.status.danger
                } else {
                    theme.text.primary
                }
            })
        }))
        .transition(Spring::snappy());
    let label = bind(move |runtime| {
        match (is_video(runtime), is_recording(runtime)) {
            (false, _) => "Take photo",
            (true, false) => "Start recording",
            (true, true) => "Stop recording",
        }
        .to_owned()
    });
    vstack((disc,))
        .size(DIAMETER)
        .shrink(0.0)
        .rounded(DIAMETER / 2.0)
        .border(RING_WIDTH)
        .border_color(tint(theme, |theme| theme.text.primary))
        .items_center()
        .justify_center()
        .active(|style| style.scale(PRESSED_SCALE))
        .when_disabled(|style| style.opacity(DISABLED_OPACITY))
        .transition(Spring::snappy())
        .cursor(CursorIcon::Pointer)
        .role(Role::Button)
        .focusable(true)
        .label(label)
        .disabled(bind(move |runtime| !state.is_live(runtime)))
        .shortcut(SPACE)
        .on_click(move |cx, _| state.press_shutter(cx))
}
