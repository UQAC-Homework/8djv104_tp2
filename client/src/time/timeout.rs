use std::time::{Duration, Instant};

/// Structure used to represent a timeout after a given duration
#[derive(Debug, Clone, Copy)]
pub struct Timeout {
    start: Instant,
    duration: Duration,
}

impl Timeout {
    /// Creates a timeout with the given duration
    pub fn new(duration: Duration) -> Self {
        let now: Instant = Instant::now();

        Self {
            start: now,
            duration,
        }
    }

    /// Checks if the given timeout has expired
    pub fn has_expired(&self) -> bool {
        self.start.elapsed() >= self.duration
    }

    /// Gets the remaining duration of the timeout
    pub fn remaining(&self) -> Duration {
        if self.has_expired() {
            return Duration::ZERO;
        }

        self.duration.checked_sub(self.start.elapsed()).unwrap_or(Duration::ZERO)
    }
}
