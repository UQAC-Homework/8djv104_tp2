use std::time::{Duration, Instant};

/// Structure used to control a repeating trigger
#[derive(Clone, Copy, Debug)]
pub struct Repeater {
    start: Instant,
    interval: Duration,
}

impl Repeater {
    /// Creates a repeater with the given interval
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            start: Instant::now(),
        }
    }

    /// Checks if a trigger is due
    pub fn is_due(&self) -> bool {
        self.start.elapsed() >= self.interval
    }

    /// Gets the remaining duration until the next trigger
    pub fn remaining(&self) -> Duration {
        if self.is_due() {
            return Duration::ZERO;
        }

        self.interval.checked_sub(self.start.elapsed()).unwrap_or(Duration::ZERO) 
    }

    /// Resets the repeater
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}
