use std::io;
use std::path::Path;
use std::process::Command;

const PREVIEW: &str = "crownos-preview";

/// Opens `path` in CrownOS Preview, reaping the viewer when it exits.
pub fn open_in_preview(path: &Path) -> io::Result<()> {
    let mut viewer = Command::new(PREVIEW).arg(path).spawn()?;
    std::thread::Builder::new()
        .name("preview-reaper".into())
        .spawn(move || viewer.wait())?;
    Ok(())
}
