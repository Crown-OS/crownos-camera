use crownui::ext::ImageId;
use crownui::prelude::{Cx, Size};

use super::rgba_image::RgbaImage;

/// An image registered with the renderer and the size it was registered at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShownImage {
    pub image: ImageId,
    pub size: Size,
}

impl ShownImage {
    /// Hands `pixels` to the renderer without copying them.
    pub fn register(cx: &mut Cx, pixels: RgbaImage) -> Self {
        let size = Size::new(pixels.width as f32, pixels.height as f32);
        let image = cx.register_image(pixels.width, pixels.height, pixels.pixels);
        Self { image, size }
    }
}
