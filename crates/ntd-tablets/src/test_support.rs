//! Helpers shared by the unit tests.

/// Raises the `log` level so that the arguments of `log::*!` calls are evaluated.
///
/// The macros skip their arguments when the level is off, which would leave those lines
/// unexecuted (and unchecked for panics) in the tests.
pub(crate) fn evaluate_log_arguments() {
    log::set_max_level(log::LevelFilter::Trace);
}
