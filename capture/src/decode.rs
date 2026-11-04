use ffmpeg_next::format::Pixel;
use ffmpeg_next::frame;
use ffmpeg_next::software::scaling;
use turbojpeg::{Decompressor, Image};

use super::{CameraFormat, CaptureError, CapturedFrame, Negotiated};
use crownos_camera_frame::{BYTES_PER_PIXEL, MemoryFrame, PixelFormat};

/// Turns captured frames into packed RGBA, reusing its buffers frame to frame.
pub struct FrameDecoder {
    jpeg: Decompressor,
    rgba: Vec<u8>,
    converter: Option<YuvConverter>,
}

struct YuvConverter {
    format: Negotiated,
    scaler: scaling::Context,
    input: frame::Video,
    output: frame::Video,
}

// SAFETY: the scaler and frames are owned here and used from one thread at a time.
unsafe impl Send for YuvConverter {}

impl FrameDecoder {
    pub fn new() -> Result<Self, CaptureError> {
        Ok(Self {
            jpeg: Decompressor::new()?,
            rgba: Vec::new(),
            converter: None,
        })
    }

    pub fn decode(&mut self, frame: &CapturedFrame) -> Result<MemoryFrame<'_>, CaptureError> {
        match frame.format.format {
            CameraFormat::Mjpeg => self.decode_jpeg(&frame.bytes),
            CameraFormat::Yuyv | CameraFormat::Nv12 => self.convert_yuv(frame),
        }
    }

    /// libjpeg-turbo's SIMD decoder, straight to RGBA.
    fn decode_jpeg(&mut self, jpeg: &[u8]) -> Result<MemoryFrame<'_>, CaptureError> {
        let header = self.jpeg.read_header(jpeg)?;
        let stride = header.width * BYTES_PER_PIXEL as usize;
        self.rgba.resize(stride * header.height, 0);
        self.jpeg.decompress(
            jpeg,
            Image {
                pixels: self.rgba.as_mut_slice(),
                width: header.width,
                pitch: stride,
                height: header.height,
                format: turbojpeg::PixelFormat::RGBA,
            },
        )?;
        Ok(MemoryFrame {
            bytes: &self.rgba,
            width: header.width as u32,
            height: header.height as u32,
            stride: stride as u32,
            format: PixelFormat::Rgba,
        })
    }

    /// FFmpeg's SIMD converter, from packed or semi-planar YUV to RGBA.
    fn convert_yuv(&mut self, frame: &CapturedFrame) -> Result<MemoryFrame<'_>, CaptureError> {
        let stale = self
            .converter
            .as_ref()
            .is_none_or(|converter| converter.format != frame.format);
        if stale {
            self.converter = Some(YuvConverter::new(frame.format)?);
        }
        let converter = self.converter.as_mut().ok_or(CaptureError::Negotiation)?;
        converter.fill_input(frame)?;
        converter
            .scaler
            .run(&converter.input, &mut converter.output)?;
        let output = &converter.output;
        Ok(MemoryFrame {
            bytes: output.data(0),
            width: output.width(),
            height: output.height(),
            stride: output.stride(0) as u32,
            format: PixelFormat::Rgba,
        })
    }
}

impl YuvConverter {
    fn new(format: Negotiated) -> Result<Self, CaptureError> {
        let pixel = match format.format {
            CameraFormat::Yuyv => Pixel::YUYV422,
            CameraFormat::Nv12 => Pixel::NV12,
            CameraFormat::Mjpeg => return Err(CaptureError::Negotiation),
        };
        let (width, height) = (format.width, format.height);
        Ok(Self {
            format,
            scaler: scaling::Context::get(
                pixel,
                width,
                height,
                Pixel::RGBA,
                width,
                height,
                scaling::Flags::POINT,
            )?,
            input: frame::Video::new(pixel, width, height),
            output: frame::Video::new(Pixel::RGBA, width, height),
        })
    }

    /// Copies the captured planes into FFmpeg's, row by row since their strides differ.
    fn fill_input(&mut self, frame: &CapturedFrame) -> Result<(), CaptureError> {
        let (width, height) = (self.format.width as usize, self.format.height as usize);
        let source_stride = frame.stride as usize;
        let planes: &[(usize, usize)] = match self.format.format {
            CameraFormat::Yuyv => &[(width * 2, height)],
            CameraFormat::Nv12 => &[(width, height), (width, height / 2)],
            CameraFormat::Mjpeg => return Err(CaptureError::Negotiation),
        };
        let expected = planes.iter().map(|(_, rows)| rows * source_stride).sum();
        if frame.bytes.len() < expected {
            return Err(CaptureError::ShortFrame {
                expected,
                actual: frame.bytes.len(),
            });
        }
        let mut source = frame.bytes.as_slice();
        for (plane, &(row_bytes, rows)) in planes.iter().enumerate() {
            let target_stride = self.input.stride(plane);
            let (plane_bytes, rest) = source.split_at(rows * source_stride);
            for (target, row) in self
                .input
                .data_mut(plane)
                .chunks_exact_mut(target_stride)
                .zip(plane_bytes.chunks_exact(source_stride))
            {
                target[..row_bytes].copy_from_slice(&row[..row_bytes]);
            }
            source = rest;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captured(format: CameraFormat, bytes: Vec<u8>, stride: u32) -> CapturedFrame {
        CapturedFrame {
            format: Negotiated {
                format,
                width: 16,
                height: 8,
            },
            stride,
            captured_at_ns: 0,
            bytes,
        }
    }

    #[test]
    fn mid_grey_yuyv_becomes_mid_grey_rgba() -> Result<(), CaptureError> {
        let yuyv: Vec<u8> = std::iter::repeat_n([128u8, 128, 128, 128], 16 * 8 / 2)
            .flatten()
            .collect();
        let mut decoder = FrameDecoder::new()?;
        let rgba = decoder.decode(&captured(CameraFormat::Yuyv, yuyv, 32))?;
        assert_eq!((rgba.width, rgba.height), (16, 8));
        let pixel = &rgba.bytes[..4];
        assert!(
            pixel[..3]
                .iter()
                .all(|&channel| (120..=140).contains(&channel))
        );
        assert_eq!(pixel[3], 255);
        Ok(())
    }

    #[test]
    fn a_jpeg_decodes_to_its_own_size() -> Result<(), CaptureError> {
        let pixels = vec![90u8; 16 * 8 * 4];
        let jpeg = turbojpeg::compress(
            Image {
                pixels: pixels.as_slice(),
                width: 16,
                pitch: 64,
                height: 8,
                format: turbojpeg::PixelFormat::RGBA,
            },
            90,
            turbojpeg::Subsamp::Sub2x2,
        )?;
        let mut decoder = FrameDecoder::new()?;
        let rgba = decoder.decode(&captured(CameraFormat::Mjpeg, jpeg.to_vec(), 0))?;
        assert_eq!((rgba.width, rgba.height, rgba.stride), (16, 8, 64));
        assert!((85..=95).contains(&rgba.bytes[0]));
        Ok(())
    }

    #[test]
    fn a_truncated_frame_is_refused() -> Result<(), CaptureError> {
        let mut decoder = FrameDecoder::new()?;
        let result = decoder.decode(&captured(CameraFormat::Nv12, vec![0; 10], 16));
        assert!(matches!(result, Err(CaptureError::ShortFrame { .. })));
        Ok(())
    }
}
