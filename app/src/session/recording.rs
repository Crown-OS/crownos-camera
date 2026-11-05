use std::mem;

use crownos_camera_frame::{FramePixels, MemoryFrame, VideoFrame};
use crownos_camera_record::{Channels, RecordError, Recorder, Recording};
use crownui_audio::{FrameConsumer, Microphone, MicrophoneOptions, MicrophoneStream};

use super::THUMBNAIL_SIDE;
use super::event::{RecordingOutcome, SavedCapture, SessionEvent, SessionEvents};
use crate::util::rgba_image::RgbaImage;

const SILENT_RECORDING: &str = "The microphone is not available, so this video has no sound.";

/// An open microphone and the ring its frames arrive on.
struct Soundtrack {
    _stream: MicrophoneStream,
    frames: FrameConsumer,
}

impl Soundtrack {
    fn open(options: MicrophoneOptions) -> Option<Self> {
        let mut stream = Microphone::open(options).ok()?;
        let frames = stream.take_frames()?;
        Some(Self {
            _stream: stream,
            frames,
        })
    }

    fn drain_into(&mut self, recorder: &mut Recorder) -> Result<(), RecordError> {
        while let Ok(frame) = self.frames.pop() {
            recorder.audio(&frame.samples, frame.pts.0)?;
        }
        Ok(())
    }
}

struct ActiveRecording {
    recorder: Recorder,
    soundtrack: Option<Soundtrack>,
    thumbnail: RgbaImage,
}

#[derive(Default)]
enum RecordingState {
    #[default]
    Idle,
    Waiting(Recording, Option<Soundtrack>),
    Active(Box<ActiveRecording>),
}

/// A recording waiting for its first frame, or running.
#[derive(Default)]
pub struct RecordingSink {
    state: RecordingState,
}

impl RecordingSink {
    /// Opens the microphone now, so recording starts with sound on the next frame.
    pub fn request(
        &mut self,
        mut recording: Recording,
        microphone: MicrophoneOptions,
        events: &SessionEvents,
    ) {
        let soundtrack = Soundtrack::open(microphone);
        match soundtrack {
            Some(_) => recording.audio = Some(Channels::Mono),
            None => events.send(SessionEvent::Notice(SILENT_RECORDING.into())),
        }
        self.state = RecordingState::Waiting(recording, soundtrack);
    }

    pub fn record(
        &mut self,
        pixels: &MemoryFrame<'_>,
        captured_at_ns: u64,
        events: &SessionEvents,
    ) {
        let frame = VideoFrame {
            pixels: FramePixels::Memory(*pixels),
            slot: 0,
            captured_at_ns,
        };
        let failure = match mem::take(&mut self.state) {
            RecordingState::Idle => None,
            RecordingState::Waiting(recording, soundtrack) => match start(&recording, &frame) {
                Ok(recorder) => {
                    events.send(SessionEvent::RecordingStarted);
                    self.state = RecordingState::Active(Box::new(ActiveRecording {
                        recorder,
                        soundtrack,
                        thumbnail: RgbaImage::thumbnail(pixels, THUMBNAIL_SIDE),
                    }));
                    None
                }
                Err(error) => Some(error),
            },
            RecordingState::Active(mut active) => match active.append(&frame) {
                Ok(()) => {
                    self.state = RecordingState::Active(active);
                    None
                }
                Err(error) => Some(error),
            },
        };
        if let Some(error) = failure {
            events.send(SessionEvent::RecordingEnded(RecordingOutcome::Failed(
                error.to_string(),
            )));
        }
    }

    /// Closes the file, if any, and tells the UI how it went.
    pub fn stop(&mut self, events: &SessionEvents) {
        let outcome = match mem::take(&mut self.state) {
            RecordingState::Idle => return,
            RecordingState::Waiting(..) => RecordingOutcome::Discarded,
            RecordingState::Active(active) => active.finish(),
        };
        events.send(SessionEvent::RecordingEnded(outcome));
    }
}

impl ActiveRecording {
    fn append(&mut self, frame: &VideoFrame<'_>) -> Result<(), RecordError> {
        self.recorder.video(frame)?;
        match &mut self.soundtrack {
            Some(soundtrack) => soundtrack.drain_into(&mut self.recorder),
            None => Ok(()),
        }
    }

    fn finish(mut self) -> RecordingOutcome {
        let drained = match &mut self.soundtrack {
            Some(soundtrack) => soundtrack.drain_into(&mut self.recorder),
            None => Ok(()),
        };
        match drained.and_then(|()| self.recorder.finish()) {
            Ok(path) => RecordingOutcome::Saved(SavedCapture {
                path,
                thumbnail: self.thumbnail,
            }),
            Err(error) => RecordingOutcome::Failed(error.to_string()),
        }
    }
}

fn start(recording: &Recording, first: &VideoFrame<'_>) -> Result<Recorder, RecordError> {
    if let Some(directory) = recording.path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    Recorder::start(recording, first)
}
