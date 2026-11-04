use std::borrow::Cow;
use std::path::{Path, PathBuf};

use chrono::Local;

/// `~/…` with the home directory filled in.
pub fn expand_home(path: &str) -> Cow<'_, Path> {
    match (path.strip_prefix("~/"), dirs::home_dir()) {
        (Some(rest), Some(home)) => Cow::Owned(home.join(rest)),
        _ => Cow::Borrowed(Path::new(path)),
    }
}

/// `directory/<prefix> <date> at <time>.<extension>`, unique within `directory`.
pub fn timestamped_file(directory: &Path, prefix: &str, extension: &str) -> PathBuf {
    let stamp = Local::now().format("%Y-%m-%d at %H.%M.%S");
    let base = format!("{prefix} {stamp}");
    let mut path = directory.join(format!("{base}.{extension}"));
    let mut copy = 2;
    while path.exists() {
        path = directory.join(format!("{base} ({copy}).{extension}"));
        copy += 1;
    }
    path
}

/// The configured folder, or `fallback` under the home directory.
pub fn directory_or(configured: Option<&str>, fallback: impl FnOnce() -> PathBuf) -> PathBuf {
    configured.map_or_else(fallback, |path| expand_home(path).into_owned())
}

pub fn default_photo_directory() -> PathBuf {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_default()
        .join("Camera")
}

pub fn default_video_directory() -> PathBuf {
    dirs::video_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_default()
        .join("Camera")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_home_only_under_tilde() {
        let home = dirs::home_dir().unwrap_or_default();
        assert_eq!(expand_home("~/Pictures"), home.join("Pictures"));
        assert_eq!(expand_home("/tmp/camera"), Path::new("/tmp/camera"));
    }

    #[test]
    fn avoids_existing_files() -> std::io::Result<()> {
        let directory =
            std::env::temp_dir().join(format!("crownos-camera-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory)?;
        let first = timestamped_file(&directory, "Photo", "jpg");
        std::fs::write(&first, b"")?;
        let second = timestamped_file(&directory, "Photo", "jpg");
        assert_ne!(first, second);
        assert!(second.to_string_lossy().ends_with(" (2).jpg"));
        std::fs::remove_dir_all(directory)
    }
}
