use crownos_camera_frame::{BYTES_PER_PIXEL, MemoryFrame, PixelFormat};

const CHANNELS: usize = BYTES_PER_PIXEL as usize;

/// Tightly packed straight-alpha RGBA8 pixels, the layout crownui uploads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    /// `frame` without its row padding.
    pub fn copy_of(frame: &MemoryFrame<'_>) -> Self {
        let mut pixels =
            Vec::with_capacity(frame.width as usize * frame.height as usize * CHANNELS);
        frame.rows().for_each(|row| pixels.extend_from_slice(row));
        reorder_to_rgba(&mut pixels, frame.format);
        Self {
            width: frame.width,
            height: frame.height,
            pixels,
        }
    }

    /// `frame` shrunk by a whole factor until its longer side fits `longest_side`, each pixel
    /// the average of the block it covers.
    pub fn thumbnail(frame: &MemoryFrame<'_>, longest_side: u32) -> Self {
        let factor = frame
            .width
            .max(frame.height)
            .div_ceil(longest_side.max(1))
            .max(1) as usize;
        let width = frame.width as usize / factor;
        let block_bytes = factor * CHANNELS;
        let block_area = (factor * factor) as u32;
        let rows: Vec<&[u8]> = frame.rows().collect();
        let mut pixels = Vec::with_capacity(width * (rows.len() / factor) * CHANNELS);
        for block_rows in rows.chunks_exact(factor) {
            for column in 0..width {
                let mut sums = [0u32; CHANNELS];
                for row in block_rows {
                    let block = &row[column * block_bytes..(column + 1) * block_bytes];
                    for pixel in block.chunks_exact(CHANNELS) {
                        sums.iter_mut()
                            .zip(pixel)
                            .for_each(|(sum, &channel)| *sum += u32::from(channel));
                    }
                }
                pixels.extend(sums.map(|sum| (sum / block_area) as u8));
            }
        }
        reorder_to_rgba(&mut pixels, frame.format);
        Self {
            width: width as u32,
            height: (rows.len() / factor) as u32,
            pixels,
        }
    }
}

fn reorder_to_rgba(pixels: &mut [u8], format: PixelFormat) {
    let (swap_red_and_blue, opaque) = match format {
        PixelFormat::Rgba => return,
        PixelFormat::Rgbx => (false, true),
        PixelFormat::Bgra => (true, false),
        PixelFormat::Bgrx => (true, true),
    };
    for pixel in pixels.chunks_exact_mut(CHANNELS) {
        if swap_red_and_blue {
            pixel.swap(0, 2);
        }
        if opaque {
            pixel[3] = u8::MAX;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(bytes: &[u8], width: u32, height: u32, stride: u32) -> MemoryFrame<'_> {
        MemoryFrame {
            bytes,
            width,
            height,
            stride,
            format: PixelFormat::Rgba,
        }
    }

    #[test]
    fn copies_drop_row_padding() {
        let bytes = [1, 2, 3, 4, 0, 0, 5, 6, 7, 8, 0, 0];
        let image = RgbaImage::copy_of(&frame(&bytes, 1, 2, 6));
        assert_eq!(image.pixels, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn thumbnails_average_blocks() {
        let bytes: Vec<u8> = [0u8, 100, 0, 100]
            .iter()
            .flat_map(|&red| [red, 0, 0, 255])
            .collect();
        let image = RgbaImage::thumbnail(&frame(&bytes, 2, 2, 8), 1);
        assert_eq!((image.width, image.height), (1, 1));
        assert_eq!(image.pixels, [50, 0, 0, 255]);
    }

    #[test]
    fn bgrx_becomes_opaque_rgba() {
        let bytes = [3, 2, 1, 0];
        let image = RgbaImage::copy_of(&MemoryFrame {
            format: PixelFormat::Bgrx,
            ..frame(&bytes, 1, 1, 4)
        });
        assert_eq!(image.pixels, [1, 2, 3, 255]);
    }
}
