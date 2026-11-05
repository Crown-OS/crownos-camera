use crownui::kit::{ToggleItem, toggle_group};
use crownui::prelude::{Semantic, Styled, View, bind};

use crate::state::{CameraState, MODES};

/// Photo or video; locked while a video records.
pub fn mode_toggle(state: CameraState) -> impl View + Styled {
    let recording = state.recording;
    let items = MODES.map(|mode| ToggleItem::text(format!("{mode:?}")));
    toggle_group(items, state.mode)
        .label("Capture mode")
        .disabled(bind(move |runtime| !recording.get(runtime).is_idle()))
}
