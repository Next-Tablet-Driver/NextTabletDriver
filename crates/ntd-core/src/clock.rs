//! A source of time that can be replaced.
//!
//! Code that needs "now" takes a [`Clock`] instead of calling `Instant::now()`, so a test can
//! decide what time it is and the result no longer depends on the machine.

use std::time::{Duration, Instant};

/// A monotonic clock. The time is a [`Duration`] since an origin the clock chooses; only the
/// difference between two readings of the same clock means something.
pub trait Clock: Send + Sync {
    /// The time elapsed since the origin of this clock.
    fn now(&self) -> Duration;
}

/// The real clock: time since it was created.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    /// A clock whose origin is now.
    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    #[inline]
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_clock_never_goes_back() {
        let clock = SystemClock::default();
        let first = clock.now();
        assert!(clock.now() >= first);
    }
}
