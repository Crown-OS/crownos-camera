//! Records a few seconds from the default camera and saves one photo, with no window.
//!
//! `cargo run --release --example capture -- [seconds] [output directory]`

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crownos_camera::util::paths::timestamped_file;
use crownos_camera_capture::{CameraCapture, CameraRequest, FrameDecoder};
use crownos_camera_frame::{FramePixels, VideoFrame};
use crownos_camera_record::{Recorder, Recording, VideoSettings, extension};
use crownos_camera_still::{EXTENSION, save_jpeg};
use crownos_config::schema::{RecordingQuality, VideoCodec, VideoContainer};

fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let seconds: u64 = arguments.next().map_or(Ok(3), |value| value.parse())?;
    let directory = arguments
        .next()
        .map_or_else(std::env::temp_dir, PathBuf::from);

    let request = CameraRequest::default();
    let (capture, frames) = CameraCapture::open(None, request).context("opening the camera")?;
    let mut decoder = FrameDecoder::new()?;
    let recording = Recording {
        path: timestamped_file(&directory, "Video", extension(VideoContainer::Mp4)),
        container: VideoContainer::Mp4,
        video: VideoSettings {
            codec: VideoCodec::H264,
            quality: RecordingQuality::High,
            frames_per_second: request.frames_per_second,
        },
        audio: None,
    };

    let mut recorder: Option<Recorder> = None;
    let mut photo = None;
    let mut decode_time = Duration::ZERO;
    let mut encode_time = Duration::ZERO;
    let mut count = 0u32;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(seconds) {
        let captured = frames.recv_timeout(Duration::from_secs(5))?;
        let decoding = Instant::now();
        let pixels = decoder.decode(&captured)?;
        decode_time += decoding.elapsed();
        let frame = VideoFrame {
            pixels: FramePixels::Memory(pixels),
            slot: 0,
            captured_at_ns: captured.captured_at_ns,
        };
        let encoding = Instant::now();
        match &mut recorder {
            Some(recorder) => recorder.video(&frame)?,
            None => {
                let started = Recorder::start(&recording, &frame)?;
                println!("recording with {}", started.encoder_name());
                recorder = Some(started);
            }
        }
        encode_time += encoding.elapsed();
        if photo.is_none() {
            let path = timestamped_file(&directory, "Photo", EXTENSION);
            save_jpeg(&pixels, &path)?;
            println!(
                "{}x{} {:?} → {}",
                pixels.width,
                pixels.height,
                captured.format.format,
                path.display()
            );
            photo = Some(path);
        }
        count += 1;
        capture.recycle(captured.bytes);
    }
    let video = recorder.context("no frame arrived")?.finish()?;
    println!(
        "{count} frames in {seconds}s, {:.1} ms to decode and {:.1} ms to encode each → {}",
        decode_time.as_secs_f64() * 1000.0 / f64::from(count.max(1)),
        encode_time.as_secs_f64() * 1000.0 / f64::from(count.max(1)),
        video.display()
    );
    Ok(())
}
