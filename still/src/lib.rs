//! Photos: one processed frame, compressed and written to disk.

mod jpeg;

pub use self::jpeg::{EXTENSION, StillError, save_jpeg};
