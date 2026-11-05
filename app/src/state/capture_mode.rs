use crownos_config::schema::CaptureMode;

/// The modes in the order the mode toggle shows them.
pub const MODES: [CaptureMode; 2] = [CaptureMode::Photo, CaptureMode::Video];

pub fn mode_index(mode: CaptureMode) -> usize {
    MODES
        .iter()
        .position(|candidate| *candidate == mode)
        .unwrap_or_default()
}

pub fn mode_at(index: usize) -> CaptureMode {
    MODES.get(index).copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_round_trip() {
        for mode in MODES {
            assert_eq!(mode_at(mode_index(mode)), mode);
        }
        assert_eq!(mode_at(7), CaptureMode::Photo);
    }
}
