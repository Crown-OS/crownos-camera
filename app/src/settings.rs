use std::path::PathBuf;

use crownos_camera_capture::CameraRequest;
use crownos_camera_record::{Recording, VideoSettings, extension};
use crownos_camera_still::EXTENSION as PHOTO_EXTENSION;
use crownos_config::schema::{Camera, CaptureMode};
use crownui_audio::{Enhancement, MicrophoneOptions};

use crate::util::paths::{
    default_photo_directory, default_video_directory, directory_or, timestamped_file,
};

const PHOTO_PREFIX: &str = "Photo";
const VIDEO_PREFIX: &str = "Video";

/// The camera section of the desktop configuration, read once at launch.
#[derive(Debug, Clone)]
pub struct Settings {
    camera: Camera,
    photo_directory: PathBuf,
    video_directory: PathBuf,
}

impl Settings {
    pub fn load() -> Self {
        let camera: Camera = crownos_config::load(Camera::SECTION);
        let photo_directory =
            directory_or(camera.photo_directory.as_deref(), default_photo_directory);
        let video_directory =
            directory_or(camera.video_directory.as_deref(), default_video_directory);
        Self {
            camera,
            photo_directory,
            video_directory,
        }
    }

    pub fn preferred_camera(&self) -> Option<&str> {
        self.camera.device.as_deref()
    }

    pub fn initial_mode(&self) -> CaptureMode {
        self.camera.mode
    }

    pub fn mirror_preview(&self) -> bool {
        self.camera.mirror_preview
    }

    pub fn request(&self) -> CameraRequest {
        CameraRequest {
            width: self.camera.width,
            height: self.camera.height,
            frames_per_second: self.camera.frame_rate,
        }
    }

    pub fn microphone(&self) -> MicrophoneOptions {
        let enhancement = if self.camera.noise_cancellation {
            Enhancement::NoiseSuppression
        } else {
            Enhancement::Off
        };
        MicrophoneOptions::default().enhancement(enhancement)
    }

    pub fn new_photo_path(&self) -> PathBuf {
        timestamped_file(&self.photo_directory, PHOTO_PREFIX, PHOTO_EXTENSION)
    }

    /// A silent recording into a new file in the video folder; sound is added once the
    /// microphone opens.
    pub fn new_recording(&self) -> Recording {
        let container = self.camera.video_container;
        Recording {
            path: timestamped_file(&self.video_directory, VIDEO_PREFIX, extension(container)),
            container,
            video: VideoSettings {
                codec: self.camera.video_codec,
                quality: self.camera.quality,
                frames_per_second: self.camera.frame_rate,
            },
            audio: None,
        }
    }
}
