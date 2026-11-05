use std::path::PathBuf;

use crownos_camera_record::Recording;
use crownui_audio::MicrophoneOptions;

/// What the UI asks of a running camera.
pub enum SessionCommand {
    /// Saves the next frame as a JPEG at the path.
    TakePhoto(PathBuf),
    /// Records from the next frame on, with sound from the microphone when it opens.
    StartRecording {
        recording: Recording,
        microphone: MicrophoneOptions,
    },
    StopRecording,
}
