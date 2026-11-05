use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use crossbeam_channel::Sender;
use crownos_camera_capture::CameraRequest;

use super::command::SessionCommand;
use super::event::SessionEvents;
use super::frame_sinks::FrameSinks;
use super::worker;

/// One camera streaming on its own thread. Dropping it saves a recording in progress and
/// closes the camera.
pub struct CameraSession {
    commands: Option<Sender<SessionCommand>>,
    preview_pending: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl CameraSession {
    /// Streams from the camera node named `camera`, or the default camera.
    pub fn start(
        camera: Option<String>,
        request: CameraRequest,
        events: SessionEvents,
    ) -> io::Result<Self> {
        let (commands, received) = crossbeam_channel::unbounded();
        let preview_pending = Arc::new(AtomicBool::new(false));
        let sinks = FrameSinks::new(events, preview_pending.clone());
        let worker = thread::Builder::new()
            .name("camera-session".into())
            .spawn(move || worker::run(camera.as_deref(), request, &received, sinks))?;
        Ok(Self {
            commands: Some(commands),
            preview_pending,
            worker: Some(worker),
        })
    }

    pub fn send(&self, command: SessionCommand) {
        if let Some(commands) = &self.commands {
            let _ = commands.send(command);
        }
    }

    /// Lets the worker hand over its next frame now that the last one is on screen.
    pub fn preview_shown(&self) {
        self.preview_pending.store(false, Ordering::Release);
    }
}

impl Drop for CameraSession {
    fn drop(&mut self) {
        drop(self.commands.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
