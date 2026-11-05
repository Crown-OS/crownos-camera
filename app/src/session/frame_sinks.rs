use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crownos_camera_frame::MemoryFrame;
use crownos_camera_still::{StillError, save_jpeg};

use super::THUMBNAIL_SIDE;
use super::command::SessionCommand;
use super::event::{SavedCapture, SessionEvent, SessionEvents};
use super::recording::RecordingSink;
use crate::util::rgba_image::RgbaImage;

/// Everywhere a decoded frame goes: the viewfinder, a photo asked for, a recording.
pub struct FrameSinks {
    events: SessionEvents,
    preview_pending: Arc<AtomicBool>,
    photo: Option<PathBuf>,
    recording: RecordingSink,
}

impl FrameSinks {
    pub fn new(events: SessionEvents, preview_pending: Arc<AtomicBool>) -> Self {
        Self {
            events,
            preview_pending,
            photo: None,
            recording: RecordingSink::default(),
        }
    }

    pub fn handle(&mut self, command: SessionCommand) {
        match command {
            SessionCommand::TakePhoto(path) => self.photo = Some(path),
            SessionCommand::StartRecording {
                recording,
                microphone,
            } => self.recording.request(recording, microphone, &self.events),
            SessionCommand::StopRecording => self.recording.stop(&self.events),
        }
    }

    pub fn consume(&mut self, pixels: &MemoryFrame<'_>, captured_at_ns: u64) {
        self.recording.record(pixels, captured_at_ns, &self.events);
        if let Some(path) = self.photo.take() {
            let taken = save_photo(pixels, &path).map_err(|error| error.to_string());
            self.events.send(SessionEvent::PhotoTaken(taken));
        }
        if !self.preview_pending.swap(true, Ordering::AcqRel) {
            self.events
                .send(SessionEvent::Frame(RgbaImage::copy_of(pixels)));
        }
    }

    /// Saves a recording in progress and reports why the camera stopped.
    pub fn end(mut self, reason: Option<String>) {
        self.recording.stop(&self.events);
        if let Some(reason) = reason {
            self.events.send(SessionEvent::Stopped(reason));
        }
    }
}

fn save_photo(pixels: &MemoryFrame<'_>, path: &Path) -> Result<SavedCapture, StillError> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    save_jpeg(pixels, path)?;
    Ok(SavedCapture {
        path: path.to_owned(),
        thumbnail: RgbaImage::thumbnail(pixels, THUMBNAIL_SIDE),
    })
}
