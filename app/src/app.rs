use anyhow::Result;
use crownui::AppConfig;
use crownui::kit::{INTER_FAMILY, inter_font, provide_configured_theme};
use crownui::prelude::{BLACK, component};

use crate::settings::Settings;
use crate::state::CameraState;
use crate::ui::shell;

const TITLE: &str = "Camera";
const APP_ID: &str = "crownos-camera";
const INITIAL_SIZE: (u32, u32) = (960, 720);
const MIN_SIZE: (u32, u32) = (480, 440);

/// Opens the camera window and runs it until it closes.
pub fn run() -> Result<()> {
    let settings = Settings::load();
    let config = app_config();
    let root = component(move |cx| {
        provide_configured_theme(cx);
        let state = CameraState::new(cx, settings);
        shell(cx, state)
    });
    Ok(crownui::run(root, config)?)
}

/// The camera window's title, size, background and font.
pub fn app_config() -> AppConfig {
    AppConfig::new(TITLE)
        .app_id(APP_ID)
        .size(INITIAL_SIZE.0, INITIAL_SIZE.1)
        .min_size(MIN_SIZE.0, MIN_SIZE.1)
        .clear_color(BLACK)
        .mcp(cfg!(debug_assertions))
        .font(inter_font().to_vec(), Some(INTER_FAMILY))
}
