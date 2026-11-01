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
pub(crate) fn evaluate_log_arguments() {
    log::set_max_level(log::LevelFilter::Trace);
}

/// A new, empty settings folder for one test, removed when this is dropped.
pub(crate) struct TestPaths {
    root: std::path::PathBuf,
    paths: ntd_settings::Paths,
}

impl std::ops::Deref for TestPaths {
    type Target = ntd_settings::Paths;

    fn deref(&self) -> &Self::Target {
        &self.paths
    }
}

impl Drop for TestPaths {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A new, empty settings folder for the current test.
///
/// Anything that reads or writes the settings takes its folder as a parameter, so a test gives
/// itself a folder of its own and never meets the files of the machine it runs on.
#[must_use]
pub(crate) fn test_paths() -> TestPaths {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let root =
        std::env::temp_dir().join(format!("ntd_test_settings_{}_{nanos}", std::process::id()));
    let paths = ntd_settings::Paths::new(root.clone());
    TestPaths { root, paths }
}
