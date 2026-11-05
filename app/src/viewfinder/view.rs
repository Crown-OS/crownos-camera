use crownui::ext::{Paint, Transform};
use crownui::prelude::{BLACK, CornerRadii, Role, Semantic, Styled, View, canvas};

use super::feed::ViewfinderFeed;
use crate::util::fit::fitted;

const PICTURE_RADIUS: f32 = 10.0;

/// The feed's newest frame, as large as fits without cropping, mirrored like a selfie view when
/// `mirrored`. Black until the first frame.
pub fn viewfinder_view(feed: ViewfinderFeed, mirrored: bool) -> impl View + Styled {
    canvas(move |runtime, size, list| {
        let Some(frame) = feed.frame.get(runtime) else {
            return;
        };
        let picture = fitted(frame.size, size);
        list.save();
        if mirrored {
            list.transform(Transform::scale(-1.0, 1.0).about(picture.center()));
        }
        list.fill(
            picture,
            CornerRadii::uniform(PICTURE_RADIUS),
            Paint::image(frame.image, picture),
        );
        list.restore();
    })
    .bg(BLACK)
    .role(Role::Image)
    .label("Viewfinder")
}
