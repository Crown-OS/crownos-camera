//! Captures every Camera state for the CrownOS-Design Figma file, with a synthetic viewfinder and
//! no camera opened. Run both appearances with `../../crownui/capture/scripts/capture-app.sh .`;
//! output lands in `$CROWN_DESIGN_DIR`.

mod fixture;

use std::{cell::Cell, path::PathBuf, rc::Rc, time::Duration, time::Instant};

use anyhow::{Context, Result};
use capture::{CaptureApp, Carrier};
use crownos_camera::{
    app::app_config,
    settings::Settings,
    state::{CameraState, LastCapture, MODES, RecordingPhase, Viewfinder},
    ui::shell,
    util::shown_image::ShownImage,
};
use crownos_config::schema::CaptureMode;
use crownui::{
    kit::{Toast, Toasts, icons, provide_configured_theme, use_toasts},
    prelude::component,
};

const COMPACT_WINDOW: (f32, f32) = (480.0, 440.0);
const LAST_PHOTO: &str = "/home/alex/Pictures/Camera/Photo 2026-11-11 at 18.42.07.jpg";
const RECORDED: Duration = Duration::from_secs(83);
const TOAST_LINGER: Duration = Duration::from_secs(3600);

const VIEWFINDER: &str = "Viewfinder";
const ERRORS: &str = "Empty & error states";
const TOASTS: &str = "Toasts";

struct Camera {
    app: CaptureApp,
    state: CameraState,
    toasts: Toasts,
}

impl Camera {
    fn open(size: Option<(f32, f32)>) -> Result<Self> {
        let mounted = Rc::new(Cell::new(None));
        let root = component({
            let mounted = Rc::clone(&mounted);
            move |cx| {
                provide_configured_theme(cx);
                let state = CameraState::without_camera(cx, Settings::load());
                mounted.set(Some((state, use_toasts(cx))));
                shell(cx, state)
            }
        });
        let app = capture::mount_app(root, &app_config(), size)?;
        let (state, toasts) = mounted.take().context("the camera window did not mount")?;
        Ok(Self { app, state, toasts })
    }

    fn live(size: Option<(f32, f32)>, mode: CaptureMode) -> Result<Self> {
        let mut camera = Self::open(size)?;
        let state = camera.state;
        let cx = camera.app.ui().cx();
        let mode_index = MODES.iter().position(|candidate| *candidate == mode);
        state.mode.set(cx, mode_index.unwrap_or_default());
        state.feed.push(cx, fixture::viewfinder_frame(), |_| {});
        state.viewfinder.set(cx, Viewfinder::Live);
        Ok(camera)
    }

    fn with_last_capture(mut self) -> Self {
        let cx = self.app.ui().cx();
        let thumbnail = ShownImage::register(cx, fixture::thumbnail());
        self.state.last_capture.set(
            cx,
            Some(LastCapture {
                path: PathBuf::from(LAST_PHOTO),
                thumbnail,
            }),
        );
        self
    }

    fn recording(mut self, phase: RecordingPhase) -> Self {
        let cx = self.app.ui().cx();
        self.state.recording.set(cx, phase);
        self.state.elapsed.set(cx, RECORDED);
        self
    }

    fn unavailable(reason: &str) -> Result<Self> {
        let mut camera = Self::open(None)?;
        let cx = camera.app.ui().cx();
        camera
            .state
            .viewfinder
            .set(cx, Viewfinder::Unavailable(reason.to_owned()));
        Ok(camera)
    }

    /// Raises the same toast as `CameraState::tell`, kept up while the capture settles.
    fn tell(mut self, message: &str) -> Self {
        let cx = self.app.ui().cx();
        let toast = Toast::new(message)
            .icon(icons::CIRCLE_ALERT)
            .duration(TOAST_LINGER);
        self.toasts.show_with(cx, toast);
        self
    }

    fn capture(
        mut self,
        carrier: &mut Carrier,
        group: &str,
        name: &str,
        about: &str,
    ) -> Result<()> {
        carrier.capture_state(&mut self.app, group, name, about)?;
        Ok(())
    }
}

fn recording_now() -> RecordingPhase {
    RecordingPhase::Recording {
        since: Instant::now(),
    }
}

fn main() -> Result<()> {
    let mut carrier = Carrier::new(
        "Camera",
        "crownos-camera: the camera window with its viewfinder, photo and video modes, recording and error states, captured headlessly from the current source with a synthetic viewfinder.",
    );

    Camera::live(None, CaptureMode::Photo)?.capture(
        &mut carrier,
        VIEWFINDER,
        "Photo",
        "Live viewfinder in photo mode before the first capture",
    )?;
    Camera::live(None, CaptureMode::Photo)?
        .with_last_capture()
        .capture(
            &mut carrier,
            VIEWFINDER,
            "Photo with last capture",
            "The newest capture sits left of the shutter",
        )?;
    let mut flashing = Camera::live(None, CaptureMode::Photo)?.with_last_capture();
    let cx = flashing.app.ui().cx();
    flashing.state.flash.set(cx, true);
    flashing.capture(
        &mut carrier,
        VIEWFINDER,
        "Taking a photo",
        "The white flash over the viewfinder as the shutter fires",
    )?;
    Camera::live(None, CaptureMode::Video)?
        .with_last_capture()
        .capture(
            &mut carrier,
            VIEWFINDER,
            "Video",
            "Video mode: the red shutter starts a recording",
        )?;
    Camera::live(None, CaptureMode::Video)?
        .with_last_capture()
        .recording(recording_now())
        .capture(
            &mut carrier,
            VIEWFINDER,
            "Recording",
            "Timer pill over the picture, stop square, mode locked",
        )?;
    Camera::live(None, CaptureMode::Video)?
        .with_last_capture()
        .recording(RecordingPhase::Saving)
        .capture(
            &mut carrier,
            VIEWFINDER,
            "Saving video",
            "Stopped and waiting for the file to close; mode still locked",
        )?;
    Camera::live(Some(COMPACT_WINDOW), CaptureMode::Photo)?
        .with_last_capture()
        .capture(
            &mut carrier,
            VIEWFINDER,
            "Compact window",
            "The smallest window the camera allows",
        )?;

    Camera::open(None)?.capture(
        &mut carrier,
        ERRORS,
        "Starting camera",
        "The camera is opening and has not sent a frame yet",
    )?;
    Camera::unavailable("The camera could not start: PipeWire: Creation failed")?.capture(
        &mut carrier,
        ERRORS,
        "Camera could not start",
        "No camera could be opened",
    )?;
    Camera::unavailable("The camera was disconnected.")?.capture(
        &mut carrier,
        ERRORS,
        "Camera disconnected",
        "The camera went away while streaming",
    )?;
    Camera::unavailable("The camera is not sending pictures. Another app may be using it.")?
        .capture(
            &mut carrier,
            ERRORS,
            "Camera in use",
            "The camera opened but sends no pictures",
        )?;

    Camera::live(None, CaptureMode::Video)?
        .with_last_capture()
        .recording(recording_now())
        .tell("The microphone is not available, so this video has no sound.")
        .capture(
            &mut carrier,
            TOASTS,
            "Microphone unavailable",
            "Notice raised when a recording starts without sound",
        )?;
    Camera::live(None, CaptureMode::Photo)?
        .with_last_capture()
        .tell("The photo could not be saved: Permission denied (os error 13)")
        .capture(
            &mut carrier,
            TOASTS,
            "Photo not saved",
            "Error toast when a photo cannot be written",
        )?;

    let path = carrier.write(None)?;
    println!("{}", path.display());
    capture::finish()
}
