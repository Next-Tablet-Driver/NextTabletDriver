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

/// Keeps the new, empty settings folder of a test alive. The folder is removed when this is dropped.
pub(crate) struct EmptySettingsDir(std::path::PathBuf);

impl Drop for EmptySettingsDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Points the settings folder of the current test at a new, empty directory.
///
/// Code such as `PluginManager::new` reads the folders of the current user. Without this, a test
/// that expects "no plugins" fails on any machine where a plugin is installed, which includes
/// the machine of a developer who tries the plugins.
#[must_use]
pub(crate) fn empty_settings_dir() -> EmptySettingsDir {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path =
        std::env::temp_dir().join(format!("ntd_empty_settings_{}_{nanos}", std::process::id()));
    let _ = std::fs::create_dir_all(&path);
    crate::settings::set_test_settings_dir(path.clone());
    EmptySettingsDir(path)
}
