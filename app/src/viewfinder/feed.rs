use crownui::prelude::{Cx, Signal};

use crate::util::rgba_image::RgbaImage;
use crate::util::shown_image::ShownImage;

/// The newest camera frame on the renderer, replaced as frames arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewfinderFeed {
    pub(super) frame: Signal<Option<ShownImage>>,
}

impl ViewfinderFeed {
    pub fn new(cx: &mut Cx) -> Self {
        Self {
            frame: cx.signal(None),
        }
    }

    /// Uploads `frame` in place of the last one; `on_shown` runs once it is on screen, so the
    /// camera thread can send the next.
    pub fn push(self, cx: &mut Cx, frame: RgbaImage, on_shown: impl FnOnce(&mut Cx) + 'static) {
        let shown = ShownImage::register(cx, frame);
        self.replace(cx, Some(shown));
        cx.request_animation_frame(move |cx, _| on_shown(cx));
    }

    pub fn clear(self, cx: &mut Cx) {
        self.replace(cx, None);
    }

    fn replace(self, cx: &mut Cx, replacement: Option<ShownImage>) {
        if let Some(previous) = self.frame.get(cx) {
            cx.release_image(previous.image);
        }
        self.frame.set(cx, replacement);
    }
}
