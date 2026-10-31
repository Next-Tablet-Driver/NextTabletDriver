use ntd_core::clock::Clock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// A clock that only moves when told to.
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
    fn the_clock_is_usable_through_the_trait_object() {
        let clock: std::sync::Arc<dyn Clock> = std::sync::Arc::new(ManualClock::new());
        assert_eq!(clock.now(), Duration::ZERO);
    }
}
