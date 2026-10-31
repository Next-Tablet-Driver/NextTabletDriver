//! A source of time that can be replaced.
//!
//! Code that needs "now" takes a [`Clock`] instead of calling `Instant::now()`, so a test can
//! decide what time it is and the result no longer depends on the machine.

use std::sync::atomic::{AtomicU64, Ordering};
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

/// A clock that only moves when told to, for tests.
#[derive(Debug, Default)]
pub struct ManualClock {
    nanos: AtomicU64,
}

impl ManualClock {
    /// A clock stopped at its origin.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nanos: AtomicU64::new(0),
        }
    }

    /// Moves the clock forward by `by`.
    pub fn advance(&self, by: Duration) {
        let nanos = u64::try_from(by.as_nanos()).unwrap_or(u64::MAX);
        self.nanos.fetch_add(nanos, Ordering::Relaxed);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manual_clock_moves_only_when_advanced() {
        let clock = ManualClock::new();
        assert_eq!(clock.now(), Duration::ZERO);
        clock.advance(Duration::from_millis(5));
        clock.advance(Duration::from_millis(7));
        assert_eq!(clock.now(), Duration::from_millis(12));
    }

    #[test]
    fn the_system_clock_never_goes_back() {
        let clock = SystemClock::default();
        let first = clock.now();
        assert!(clock.now() >= first);
    }
}
