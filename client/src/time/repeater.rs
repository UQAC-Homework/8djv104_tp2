use std::time::{Duration, Instant};

/// Structure used to control a repeating trigger
#[derive(Clone, Copy, Debug)]
pub struct Repeater {
    interval: Duration,
    next: Instant,
}

impl Repeater {
    /// Creates a repeater with the given interval
    pub fn new(interval: Duration) -> Self {
        assert!(!interval.is_zero(), "interval must be non-zero");

        let now = Instant::now();

        Self {
            interval,
            next: now + interval,
        }
    }

    /// Checks if a trigger is due
    pub fn pool(&mut self) -> bool {
        let now = Instant::now();

        if !self.is_due_at(now) {
            return false;
        }

        while self.is_due_at(now) {
            self.next += self.interval;
        }

        true
    }

    /// Checks if a trigger is due at the given instant
    fn is_due_at(&self, now: Instant) -> bool {
        now >= self.next
    }

    /// Gets the remaining duration until the next trigger
    pub fn remaining(&self) -> Duration {
        self.next.saturating_duration_since(Instant::now())
    }
}
