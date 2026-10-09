//! Helpers shared by the unit tests.
//!
//! Some of these touch state that belongs to the whole process (the `log` level, a panic hook,
//! the telemetry sender). The suite is meant to be run with `cargo nextest`, which gives every
//! test its own process; under plain `cargo test` the tests share one process and these helpers
//! are only safe because they never lower anything another test relies on.

/// Raises the `log` level so that the arguments of `log::*!` calls are evaluated.
///
/// The macros skip their arguments when the level is off, which would leave those lines
/// unexecuted (and unchecked for panics) in the tests.
pub fn evaluate_log_arguments() {
    log::set_max_level(log::LevelFilter::Trace);
}
