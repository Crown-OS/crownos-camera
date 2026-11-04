use std::path::Path;

use thiserror::Error;
use turbojpeg::{Compressor, Image, Subsamp};

use crownos_camera_frame::{MemoryFrame, PixelFormat};

pub const EXTENSION: &str = "jpg";
const QUALITY: i32 = 92;

#[derive(Debug, Error)]
pub enum StillError {
    #[error("JPEG: {0}")]
    Jpeg(#[from] turbojpeg::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Compresses `frame` with libjpeg-turbo's SIMD encoder, reading it in place.
pub fn save_jpeg(frame: &MemoryFrame<'_>, path: &Path) -> Result<(), StillError> {
    let mut compressor = Compressor::new()?;
    compressor.set_quality(QUALITY)?;
    compressor.set_subsamp(Subsamp::Sub2x2)?;
    let image = Image {
        pixels: frame.bytes,
        width: frame.width as usize,
        pitch: frame.stride as usize,
        height: frame.height as usize,
        format: turbojpeg_format(frame.format),
    };
    std::fs::write(path, compressor.compress_to_owned(image)?)?;
    Ok(())
}

fn turbojpeg_format(format: PixelFormat) -> turbojpeg::PixelFormat {
    match format {
        PixelFormat::Bgrx => turbojpeg::PixelFormat::BGRX,
        PixelFormat::Bgra => turbojpeg::PixelFormat::BGRA,
        PixelFormat::Rgbx => turbojpeg::PixelFormat::RGBX,
        PixelFormat::Rgba => turbojpeg::PixelFormat::RGBA,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_decodable_jpeg() -> Result<(), StillError> {
        let (width, height) = (64u32, 48u32);
        let bytes: Vec<u8> = (0..width * height)
            .flat_map(|index| [(index % 256) as u8, 128, 64, 255])
            .collect();
        let frame = MemoryFrame {
            bytes: &bytes,
            width,
            height,
            stride: width * 4,
            format: PixelFormat::Rgba,
        };
        let path = std::env::temp_dir().join(format!("crownos-camera-{}.jpg", std::process::id()));
        save_jpeg(&frame, &path)?;
        let header = turbojpeg::read_header(&std::fs::read(&path)?)?;
        std::fs::remove_file(&path)?;
        assert_eq!((header.width, header.height), (64, 48));
        Ok(())
    }
}
