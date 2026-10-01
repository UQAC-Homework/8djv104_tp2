use std::time::{Duration, Instant};

/// Structure used to control a repeating trigger
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

    /// Resets the 
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}
