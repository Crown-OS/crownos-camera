use std::time::Duration;

use crossbeam_channel::{Receiver, select};
use crownos_camera_capture::{
    CameraCapture, CameraRequest, CaptureError, CapturedFrame, FrameDecoder,
};

use super::command::SessionCommand;
use super::frame_sinks::FrameSinks;

/// How long the camera may go without sending a frame before it counts as gone.
const FRAME_TIMEOUT: Duration = Duration::from_secs(6);
const CAMERA_LOST: &str = "The camera was disconnected.";
const CAMERA_SILENT: &str = "The camera is not sending pictures. Another app may be using it.";

struct Stream {
    capture: CameraCapture,
    decoder: FrameDecoder,
}

impl Stream {
    fn open(
        camera: Option<&str>,
        request: CameraRequest,
    ) -> Result<(Self, Receiver<CapturedFrame>), CaptureError> {
        let (capture, frames) = CameraCapture::open(camera, request)?;
        let decoder = FrameDecoder::new()?;
        Ok((Self { capture, decoder }, frames))
    }

    fn deliver(&mut self, captured: CapturedFrame, sinks: &mut FrameSinks) {
        if let Ok(pixels) = self.decoder.decode(&captured) {
            sinks.consume(&pixels, captured.captured_at_ns);
        }
        self.capture.recycle(captured.bytes);
    }
}

/// Streams `camera`, or the default camera, into `sinks` until the UI hangs up or the camera
/// stops sending frames.
pub fn run(
    camera: Option<&str>,
    request: CameraRequest,
    commands: &Receiver<SessionCommand>,
    mut sinks: FrameSinks,
) {
    let (mut stream, frames) = match Stream::open(camera, request) {
        Ok(opened) => opened,
        Err(error) => return sinks.end(Some(format!("The camera could not start: {error}"))),
    };
    let reason = loop {
        select! {
            recv(commands) -> command => match command {
                Ok(command) => sinks.handle(command),
                Err(_) => break None,
            },
            recv(frames) -> captured => match captured {
                Ok(captured) => stream.deliver(captured, &mut sinks),
                Err(_) => break Some(CAMERA_LOST),
            },
            default(FRAME_TIMEOUT) => break Some(CAMERA_SILENT),
        }
    };
    sinks.end(reason.map(str::to_owned));
}
