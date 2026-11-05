use std::path::PathBuf;

use crate::util::shown_image::ShownImage;

/// The newest photo or video, opened in Preview from its thumbnail.
#[derive(Debug, Clone, PartialEq)]
pub struct LastCapture {
    pub path: PathBuf,
    pub thumbnail: ShownImage,
}
