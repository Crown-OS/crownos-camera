//! The live picture: frames uploaded to the renderer and the view that draws them. Nothing else
//! touches viewfinder images, so a zero-copy camera preview can replace this module alone.

mod feed;
mod view;

pub use feed::ViewfinderFeed;
pub use view::viewfinder_view;
