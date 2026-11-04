use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, TrySendError};
use pipewire as pw;
use pw::properties::properties;
use pw::spa::param::ParamType;
use pw::spa::pod::Pod;
use pw::spa::utils::Direction;
use pw::stream::{StreamBox, StreamFlags};
use rustix::time::{ClockId, clock_gettime};

use super::CaptureError;
use super::format::{self, CameraRequest, Negotiated};

const STREAM_NAME: &str = "crownos-camera";
/// Frames in flight between the PipeWire thread and the consumer. More only
/// adds latency: when the consumer falls behind, new frames are dropped.
const QUEUE_DEPTH: usize = 2;

/// One frame as the camera delivered it: compressed or packed YUV.
#[derive(Debug)]
pub struct CapturedFrame {
    pub format: Negotiated,
    pub stride: u32,
    /// When the frame was dequeued, on `CLOCK_MONOTONIC`, which the PipeWire graph clock follows.
    pub captured_at_ns: u64,
    pub bytes: Vec<u8>,
}

/// A camera stream running on its own PipeWire loop thread.
pub struct CameraCapture {
    quit: pw::channel::Sender<()>,
    recycle: Sender<Vec<u8>>,
    thread: Option<JoinHandle<()>>,
}

impl CameraCapture {
    /// Streams from the camera node named `target`, or the default camera.
    pub fn open(
        target: Option<&str>,
        request: CameraRequest,
    ) -> Result<(Self, Receiver<CapturedFrame>), CaptureError> {
        let (frames, received) = crossbeam_channel::bounded(QUEUE_DEPTH);
        let (recycle, recycled) = crossbeam_channel::bounded(QUEUE_DEPTH + 1);
        let (quit, quit_receiver) = pw::channel::channel();
        let (ready_sender, ready) = mpsc::sync_channel(1);
        let target = target.map(str::to_owned);
        let thread = thread::Builder::new()
            .name("camera".into())
            .spawn(move || {
                let state = StreamState {
                    negotiated: None,
                    frames,
                    recycled,
                };
                if let Err(error) = run_loop(
                    target.as_deref(),
                    request,
                    state,
                    quit_receiver,
                    &ready_sender,
                ) {
                    let _ = ready_sender.send(Err(error));
                }
            })?;
        let capture = Self {
            quit,
            recycle,
            thread: Some(thread),
        };
        match ready.recv() {
            Ok(Ok(())) => Ok((capture, received)),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(CaptureError::Stopped),
        }
    }

    /// Hands a consumed frame's memory back for the next one.
    pub fn recycle(&self, mut bytes: Vec<u8>) {
        bytes.clear();
        let _ = self.recycle.try_send(bytes);
    }
}

impl Drop for CameraCapture {
    fn drop(&mut self) {
        let _ = self.quit.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct StreamState {
    negotiated: Option<Negotiated>,
    frames: Sender<CapturedFrame>,
    recycled: Receiver<Vec<u8>>,
}

impl StreamState {
    fn deliver(&mut self, stream: &pw::stream::Stream) {
        let Some(format) = self.negotiated else {
            return;
        };
        let Some(mut buffer) = stream.dequeue_buffer() else {
            return;
        };
        let Some(data) = buffer.datas_mut().first_mut() else {
            return;
        };
        let chunk = data.chunk();
        let (offset, size, stride) = (
            chunk.offset() as usize,
            chunk.size() as usize,
            chunk.stride().unsigned_abs(),
        );
        let Some(bytes) = data
            .data()
            .and_then(|bytes| bytes.get(offset..offset + size))
        else {
            return;
        };
        let mut owned = self.recycled.try_recv().unwrap_or_default();
        owned.extend_from_slice(bytes);
        let frame = CapturedFrame {
            format,
            stride,
            captured_at_ns: monotonic_ns(),
            bytes: owned,
        };
        if let Err(TrySendError::Disconnected(_)) = self.frames.try_send(frame) {
            self.negotiated = None;
        }
    }
}

fn monotonic_ns() -> u64 {
    let now = clock_gettime(ClockId::Monotonic);
    now.tv_sec.unsigned_abs() * 1_000_000_000 + now.tv_nsec.unsigned_abs()
}

fn run_loop(
    target: Option<&str>,
    request: CameraRequest,
    state: StreamState,
    quit: pw::channel::Receiver<()>,
    ready: &mpsc::SyncSender<Result<(), CaptureError>>,
) -> Result<(), CaptureError> {
    pw::init();
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let mut properties = properties! {
        *pw::keys::MEDIA_TYPE => "Video",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Camera",
        *pw::keys::NODE_NAME => STREAM_NAME,
    };
    if let Some(target) = target {
        properties.insert(*pw::keys::TARGET_OBJECT, target);
    }
    let stream = StreamBox::new(&core, STREAM_NAME, properties)?;
    let buffers = format::cpu_buffers()?;
    let _listener = stream
        .add_local_listener_with_user_data(state)
        .param_changed(move |stream, state, id, param| {
            if id != ParamType::Format.as_raw() {
                return;
            }
            state.negotiated = param.and_then(format::negotiated);
            if state.negotiated.is_some()
                && let Some(buffers) = Pod::from_bytes(&buffers)
            {
                let _ = stream.update_params(&mut [buffers]);
            }
        })
        .process(|stream, state| state.deliver(stream))
        .register()?;
    let offers = format::enum_formats(request)?;
    let mut params: Vec<&Pod> = offers
        .iter()
        .filter_map(|bytes| Pod::from_bytes(bytes))
        .collect();
    stream.connect(
        Direction::Input,
        None,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;
    let _quit = quit.attach(main_loop.loop_(), {
        let main_loop = main_loop.clone();
        move |()| main_loop.quit()
    });
    let _ = ready.send(Ok(()));
    main_loop.run();
    Ok(())
}
