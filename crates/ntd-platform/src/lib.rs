//! # `ntd-platform`
//!
//! The small pieces of operating system integration that are neither HID nor input injection:
//!
//! - **autostart**: run the application when the user logs in
//!   ([`set_run_at_startup`], [`is_run_at_startup_registered`]);
//! - **named locks**: a machine-wide, non-blocking lock that the OS releases when its owner dies
//!   ([`try_acquire_lock`]);
//! - **thread priority**: raise the polling thread ([`raise_current_thread_priority`]);
//! - **timer resolution**: ask Windows for a finer timer ([`set_high_resolution_timer`]).
//!
//! Each system has its own file (`windows.rs`, `linux.rs`, `macos.rs`); any other target builds
//! with `unsupported.rs`, whose functions say [`PlatformError::Unsupported`]. Nothing here knows
//! where the user's folders are: the caller passes them in [`UserPaths`].

use std::path::PathBuf;

/// The folder where the settings of the current user live.
///
/// It is `Settings` inside the per-user configuration folder of the application, or a relative
/// `Settings` when the system has none. The entry point of a program calls it once and hands
/// the result down (the settings crate never looks for it).
#[must_use]
pub fn default_settings_dir() -> PathBuf {
    settings_dir_in(
        directories::ProjectDirs::from("com", "NextTabletDriver", "NextTabletReader")
            .map(|dirs| dirs.config_dir().to_path_buf()),
    )
}

/// `<config_dir>/Settings`; a relative `Settings` when there is no configuration folder.
fn settings_dir_in(config_dir: Option<PathBuf>) -> PathBuf {
    config_dir.map_or_else(|| PathBuf::from("Settings"), |dir| dir.join("Settings"))
}

/// The name the application goes by in the registry, the autostart entry and the lock names.
#[cfg(any(windows, target_os = "linux"))]
const APP_NAME: &str = "NextTabletDriver";

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as os;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as os;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
use unsupported as os;

/// Why an operating system call failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlatformError {
    /// This system does not offer it, or the target is not implemented yet.
    #[error("not supported on this platform")]
    Unsupported,
    /// The system refuses: the process lacks the privilege.
    #[error("permission denied")]
    PermissionDenied,
    /// The system call failed, with its own message.
    #[error("{0}")]
    Os(String),
}

/// The folders of the current user that autostart needs. The caller finds them once (with the
/// `directories` crate, for example) and passes them in: this crate never looks for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPaths {
    /// The home folder of the user.
    pub home: PathBuf,
    /// The configuration folder (`$XDG_CONFIG_HOME` or `~/.config` on Linux).
    pub config: PathBuf,
}

/// Enables or disables the launch of the application at login. The command that is registered
/// runs the current executable with `--autostart`.
///
/// # Errors
/// [`PlatformError::Os`] when the registry or the file system refuses,
/// [`PlatformError::Unsupported`] on a system without autostart.
pub fn set_run_at_startup(enabled: bool, paths: &UserPaths) -> Result<(), PlatformError> {
    os::set_run_at_startup(enabled, paths)
}

/// Whether the application is registered to run at login.
#[must_use]
pub fn is_run_at_startup_registered(paths: &UserPaths) -> bool {
    os::is_run_at_startup_registered(paths)
}

/// The machine-wide locks of the application. Their names are part of the contract between the
/// desktop app and the SDK of integrators, so they never change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedLock {
    /// Held by the one process that is allowed to open the tablet's HID device.
    HidOwner,
}

/// Holds a [`NamedLock`]; dropping it releases the lock.
pub struct LockGuard {
    #[allow(
        dead_code,
        reason = "held only so that dropping the guard releases the lock"
    )]
    handle: os::LockHandle,
}

/// Tries to take `lock` without waiting.
///
/// `None` when another process (or another thread, on Windows and Linux alike) holds it. The
/// system releases the lock when its holder exits or crashes, so there is never a stale lock to
/// clean up.
#[must_use]
pub fn try_acquire_lock(lock: NamedLock) -> Option<LockGuard> {
    os::try_acquire_lock(lock).map(|handle| LockGuard { handle })
}

/// Raises the priority of the calling thread for low-latency polling: `TIME_CRITICAL` on
/// Windows, `nice -11` on Linux.
///
/// # Errors
/// [`PlatformError::PermissionDenied`] when the process may not raise its priority (Linux needs
/// `CAP_SYS_NICE`), [`PlatformError::Os`] on any other failure.
pub fn raise_current_thread_priority() -> Result<(), PlatformError> {
    os::raise_current_thread_priority()
}

/// Asks the system for a finer timer resolution, or gives it back. Only Windows needs it (its
/// default tick is about 15.6 ms); other systems return `Ok` and do nothing.
///
/// # Errors
/// [`PlatformError::Os`] when the system functions are missing or refuse.
pub fn set_high_resolution_timer(enable: bool) -> Result<(), PlatformError> {
    os::set_high_resolution_timer(enable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_live_in_a_settings_folder_of_the_config_folder() {
        assert_eq!(
            settings_dir_in(Some(PathBuf::from("cfg"))),
            PathBuf::from("cfg").join("Settings")
        );
        assert_eq!(settings_dir_in(None), PathBuf::from("Settings"));
        assert!(default_settings_dir().ends_with("Settings"));
    }

    #[test]
    fn errors_read_as_sentences() {
        assert_eq!(
            PlatformError::Unsupported.to_string(),
            "not supported on this platform"
        );
        assert_eq!(
            PlatformError::PermissionDenied.to_string(),
            "permission denied"
        );
        assert_eq!(PlatformError::Os("boom".into()).to_string(), "boom");
    }

    #[test]
    fn the_timer_request_is_harmless_to_repeat() {
        // Windows balances `timeBeginPeriod` and `timeEndPeriod`; elsewhere it is a no-op.
        assert!(set_high_resolution_timer(true).is_ok());
        assert!(set_high_resolution_timer(false).is_ok());
    }
}
