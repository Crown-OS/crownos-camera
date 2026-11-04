/// Microsecond timestamps from the first recorded frame, from graph-clock nanoseconds.
#[derive(Debug, Clone, Copy)]
pub struct RecordingClock {
    started_at_ns: u64,
}

impl RecordingClock {
    pub const MICROSECONDS: i32 = 1_000_000;

    pub fn starting_at(started_at_ns: u64) -> Self {
        Self { started_at_ns }
    }

    pub fn microseconds_at(&self, instant_ns: u64) -> i64 {
        (instant_ns.saturating_sub(self.started_at_ns) / 1_000) as i64
    }

    /// Nanoseconds from the start to `instant_ns`, negative when it came first.
    pub fn offset_ns(&self, instant_ns: u64) -> i128 {
        i128::from(instant_ns) - i128::from(self.started_at_ns)
    }
}

/// Drops frames that arrive faster than the target frame rate.
#[derive(Debug)]
pub struct FramePacer {
    interval: i64,
    last: Option<i64>,
}

impl FramePacer {
    pub fn new(frames_per_second: u32) -> Self {
        Self {
            interval: i64::from(RecordingClock::MICROSECONDS) / i64::from(frames_per_second.max(1)),
            last: None,
        }
    }

    pub fn accept(&mut self, timestamp: i64) -> bool {
        let due = self
            .last
            .is_none_or(|last| timestamp - last >= self.interval * 9 / 10);
        if due {
            self.last = Some(timestamp);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paces_to_frame_rate() {
        let mut pacer = FramePacer::new(30);
        assert!(pacer.accept(0));
        assert!(!pacer.accept(16_667));
        assert!(pacer.accept(33_333));
        assert!(pacer.accept(66_000));
    }

    #[test]
    fn timestamps_start_at_the_first_frame() {
        let clock = RecordingClock::starting_at(5_000_000_000);
        assert_eq!(clock.microseconds_at(5_000_000_000), 0);
        assert_eq!(clock.microseconds_at(5_033_333_333), 33_333);
        assert_eq!(clock.microseconds_at(4_000_000_000), 0);
        assert_eq!(clock.offset_ns(4_999_000_000), -1_000_000);
    }
}
