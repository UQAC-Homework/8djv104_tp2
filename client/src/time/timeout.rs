use std::time::{Duration, Instant};

/// Structure used to represent a timeout after a given duration
#[derive(Debug, Clone, Copy)]
pub struct Timeout {
    /// Time after which the timeout is expired
    deadline: Instant,
}

impl Timeout {
    /// Creates a timeout with the given duration
    pub fn new(duration: Duration) -> Self {
        let now: Instant = Instant::now();

        Self {
            deadline: now + duration,
        }
    }

    /// Checks if the timeout has expired
    pub fn has_expired(&self) -> bool {
        Instant::now() > self.deadline
    }

    /// Gets the remaining duration of the timeout
    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
}
