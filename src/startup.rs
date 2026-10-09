//! # Startup Management
//!
//! This module provides utilities to manage the application's lifecycle,
//! specifically handling the "run at startup" functionality.
//!
//! # Platform Specifics
//! - **Windows**: Creates/removes a value in `HKCU\...\CurrentVersion\Run`.
//! - **Linux**: Creates/removes a `.desktop` file in `~/.config/autostart/`.

use directories::UserDirs;
use std::env;
use std::fs;
use std::path::PathBuf;

/// The name of the application used for shortcut/autostart naming.
const APP_NAME: &str = "NextTabletDriver";

// Windows Implementation: value under HKCU\Software\Microsoft\Windows\CurrentVersion\Run
#[cfg(windows)]
mod platform {
    use super::{APP_NAME, PathBuf, UserDirs, env, fs};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn to_wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
        s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Builds the command line stored in the `Run` key: the quoted executable path followed
    /// by the `--autostart` flag. Quoting keeps paths containing spaces working and avoids
    /// the classic unquoted-path ambiguity.
    pub(super) fn autostart_command(exe: &Path) -> String {
        format!("\"{}\" --autostart", exe.display())
    }

    /// RAII wrapper so the registry key is always closed.
    struct RunKey(HKEY);

    impl RunKey {
        fn open(access: u32, create: bool) -> Result<Self, String> {
            let subkey = to_wide(RUN_KEY);
            let mut key: HKEY = std::ptr::null_mut();
            let status = if create {
                // SAFETY: `subkey` is a NUL-terminated UTF-16 string that outlives the call and
                // `key` is a valid out-pointer.
                unsafe {
                    RegCreateKeyExW(
                        HKEY_CURRENT_USER,
                        subkey.as_ptr(),
                        0,
                        std::ptr::null(),
                        REG_OPTION_NON_VOLATILE,
                        access,
                        std::ptr::null(),
                        &raw mut key,
                        std::ptr::null_mut(),
                    )
                }
            } else {
                // SAFETY: same as above.
                unsafe {
                    RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, access, &raw mut key)
                }
            };
            if status == ERROR_SUCCESS {
                Ok(Self(key))
            } else {
                Err(format!("Cannot open the Run registry key (error {status})"))
            }
        }
    }

    impl Drop for RunKey {
        fn drop(&mut self) {
            // SAFETY: `self.0` is a key handle obtained from a successful open/create call.
            unsafe { RegCloseKey(self.0) };
        }
    }

    /// Best-effort removal of the `.lnk` shortcut that older versions created in the Startup
    /// folder, so that users upgrading do not end up launching the app twice.
    fn remove_legacy_shortcut() {
        let Some(dirs) = UserDirs::new() else {
            return;
        };
        let shortcut: PathBuf = dirs
            .home_dir()
            .join(r"AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup")
            .join(format!("{APP_NAME}.lnk"));
        if shortcut.exists() && fs::remove_file(&shortcut).is_ok() {
            log::info!(target: "Startup", "Removed legacy startup shortcut: {}", shortcut.display());
        }
    }

    fn write_value(name: &str, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        let name = to_wide(name);
        let value = to_wide(command);
        let key = RunKey::open(KEY_SET_VALUE, true)?;
        let byte_len = u32::try_from(value.len() * std::mem::size_of::<u16>())?;
        // SAFETY: `name` and `value` are NUL-terminated UTF-16 buffers and `byte_len` is the
        // exact byte size of `value`, terminator included, as REG_SZ requires.
        let status = unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast::<u8>(),
                byte_len,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("Failed to write the startup registry value (error {status})").into())
        }
    }

    fn delete_value(name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let Ok(key) = RunKey::open(KEY_SET_VALUE, false) else {
            return Ok(()); // the Run key does not exist: nothing registered
        };
        let name = to_wide(name);
        // SAFETY: `name` is a NUL-terminated UTF-16 string and `key.0` is a valid open key.
        let status = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
        // ERROR_FILE_NOT_FOUND simply means it was not registered.
        if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(format!("Failed to remove the startup registry value (error {status})").into())
        }
    }

    fn has_value(name: &str) -> bool {
        let Ok(key) = RunKey::open(KEY_QUERY_VALUE, false) else {
            return false;
        };
        let name = to_wide(name);
        // SAFETY: `name` is NUL-terminated; null data/size pointers only ask whether the
        // value exists.
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        status == ERROR_SUCCESS
    }

    /// Enables or disables the application's automatic launch at Windows startup by
    /// writing (or deleting) a value in the per-user `Run` registry key.
    ///
    /// This replaces the former approach of generating a `VBScript` in `%TEMP%` and running it
    /// through `wscript.exe` to create a `.lnk` file: no temporary executable script, no
    /// child process, no COM, and no antivirus false positives.
    ///
    /// # Errors
    /// Returns an error if the executable path cannot be determined or the registry cannot be
    /// written.
    pub fn set_run_at_startup(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        remove_legacy_shortcut();

        if enabled {
            let command = autostart_command(&env::current_exe()?);
            write_value(APP_NAME, &command)?;
            log::info!(target: "Startup", "Registered run-at-startup: {command}");
        } else {
            delete_value(APP_NAME)?;
            log::info!(target: "Startup", "Removed run-at-startup registration");
        }
        Ok(())
    }

    /// Checks if the application is currently configured to run at startup.
    #[must_use]
    pub fn is_run_at_startup_registered() -> bool {
        has_value(APP_NAME)
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod tests {
        use super::*;

        #[test]
        fn command_quotes_the_path_and_passes_autostart() {
            let cmd = autostart_command(Path::new(
                r"C:\Program Files\NextTabletDriver\NextTabletDriver.exe",
            ));
            assert_eq!(
                cmd,
                r#""C:\Program Files\NextTabletDriver\NextTabletDriver.exe" --autostart"#
            );
        }

        /// Round-trips a value through the real registry under a unique, self-cleaning name.
        #[test]
        fn registry_value_roundtrip() {
            let name = format!("NextTabletDriverTest_{}", std::process::id());
            assert!(!has_value(&name));

            write_value(&name, r#""C:\x\app.exe" --autostart"#).unwrap();
            assert!(has_value(&name));

            delete_value(&name).unwrap();
            assert!(!has_value(&name));
            // Deleting something that is not registered is not an error.
            delete_value(&name).unwrap();
        }
    }
}

// Linux Implementation .desktop file in ~/.config/autostart/
#[cfg(target_os = "linux")]
mod platform {
    use super::{APP_NAME, PathBuf, UserDirs, env, fs};

    /// Returns the path to the autostart directory: `~/.config/autostart/`.
    fn get_autostart_dir() -> std::path::PathBuf {
        let config_dir = env::var("XDG_CONFIG_HOME").map_or_else(
            |_| {
                UserDirs::new().map_or_else(
                    || PathBuf::from(".config"),
                    |dirs| dirs.home_dir().join(".config"),
                )
            },
            PathBuf::from,
        );
        config_dir.join("autostart")
    }

    /// Returns the full path to the `.desktop` autostart entry.
    fn get_desktop_path() -> PathBuf {
        let mut p = get_autostart_dir();
        p.push(format!("{APP_NAME}.desktop"));
        p
    }

    /// Enables or disables the application's automatic launch at session startup.
    ///
    /// Creates or removes a `.desktop` file following the XDG Autostart specification.
    /// # Errors
    /// Returns an error if the autostart directory cannot be determined, the executable
    /// path is invalid, or if file system operations fail.
    pub fn set_run_at_startup(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        let desktop_path = get_desktop_path();

        if enabled {
            if let Some(parent) = desktop_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let exe_path = env::current_exe()?;
            let exe_path_str = exe_path.to_str().ok_or("Invalid executable path")?;

            let desktop_content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name={APP_NAME}\n\
                 Comment=Tablet Driver for Osu! and Drawing\n\
                 Exec={exe_path_str} --autostart\n\
                 Terminal=false\n\
                 X-GNOME-Autostart-enabled=true\n\
                 StartupNotify=false\n"
            );

            fs::write(&desktop_path, desktop_content)?;
            log::info!(target: "Startup", "Created autostart entry: {}", desktop_path.display());
        } else if desktop_path.exists() {
            fs::remove_file(&desktop_path)?;
            log::info!(target: "Startup", "Removed autostart entry: {}", desktop_path.display());
        }
        Ok(())
    }

    /// Checks if the application is currently configured to run at startup.
    #[must_use]
    pub fn is_run_at_startup_registered() -> bool {
        get_desktop_path().exists()
    }
}

// Public re-exports unified cross-platform API
pub use platform::is_run_at_startup_registered;
pub use platform::set_run_at_startup;

/// Queries the operating system for the total amount of physical memory (RAM) in bytes.
///
/// On Windows, queries `GlobalMemoryStatusEx`.
#[cfg(windows)]
#[must_use]
pub fn get_memory_info() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut mem_status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };
    // SAFETY: mem_status is valid, dwLength is initialized, and GlobalMemoryStatusEx is a safe Win32 query.
    let success = unsafe { GlobalMemoryStatusEx(&raw mut mem_status) };
    if success != 0 {
        Some(mem_status.ullTotalPhys)
    } else {
        None
    }
}

/// Queries the operating system for the total amount of physical memory (RAM) in bytes.
///
/// On Linux, parses `/proc/meminfo`.
#[cfg(not(windows))]
#[must_use]
pub fn get_memory_info() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(kb) = parts.get(1).and_then(|s| s.parse::<u64>().ok()) {
                        return Some(kb * 1024); // KB to Bytes
                    }
                }
            }
        }
    }
    None
}

/// Adjusts the Windows system timer resolution to minimize input latency.
///
/// # Technical Details
/// By default, Windows uses a timer interval of ~15.6ms. For a high-performance
/// tablet driver, this can lead to "aliasing" or "jitter" where tablet reports
/// (often 1000Hz+) are processed in inconsistent batches.
///
/// This function calls the undocumented `NtSetTimerResolution` in `ntdll.dll`
/// to force a **0.5ms** (5000 units of 100ns) resolution, the maximum
/// precision supported by the Windows kernel.
#[cfg(windows)]
pub fn set_fast_timer(enable: bool) {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

    // SAFETY: `GetModuleHandleA` is called with a valid static C string literal.
    let ntdll = unsafe { GetModuleHandleA(c"ntdll.dll".as_ptr().cast::<u8>()) };
    if ntdll.is_null() {
        log::warn!(target: "Timer", "Failed to get ntdll handle for timer resolution");
        return;
    }

    // SAFETY: `GetProcAddress` is called with a valid static C string function name on a verified ntdll handle.
    let addr_set = unsafe { GetProcAddress(ntdll, c"NtSetTimerResolution".as_ptr().cast::<u8>()) };
    // SAFETY: Same as above; the name is a valid static C string and `ntdll` is a valid handle.
    let addr_query =
        unsafe { GetProcAddress(ntdll, c"NtQueryTimerResolution".as_ptr().cast::<u8>()) };

    if let (Some(addr_set), Some(addr_query)) = (addr_set, addr_query) {
        // SAFETY: The signature matches the documented prototype.
        // SAFETY: `NtSetTimerResolution` has this exact signature (see above).
        let nt_set: unsafe extern "system" fn(u32, u8, *mut u32) -> i32 =
            unsafe { std::mem::transmute(addr_set) };
        // SAFETY: `NtQueryTimerResolution` has this exact signature (see above).
        let nt_query: unsafe extern "system" fn(*mut u32, *mut u32, *mut u32) -> i32 =
            unsafe { std::mem::transmute(addr_query) };

        let mut min = 0;
        let mut max = 0;
        let mut cur = 0;

        // SAFETY: All three pointers refer to live local `u32`s.
        let _ = unsafe { nt_query(&raw mut min, &raw mut max, &raw mut cur) };

        log::debug!(target: "Timer", "System Timer Resolution: Min={:.1}ms, Max={:.1}ms, Current={:.1}ms",
            f64::from(min) / 10000.0, f64::from(max) / 10000.0, f64::from(cur) / 10000.0);

        let enable_val = u8::from(enable);

        if enable {
            // SAFETY: `timeBeginPeriod` is called with a valid parameter.
            unsafe { windows_sys::Win32::Media::timeBeginPeriod(1) };
        } else {
            // Release the standard timeBeginPeriod if disabling
            // SAFETY: `timeEndPeriod` is called with the same period passed to `timeBeginPeriod`.
            unsafe { windows_sys::Win32::Media::timeEndPeriod(1) };
        }

        let mut new_cur = 0;
        // SAFETY: `new_cur` is a live local `u32`; `max` comes from `NtQueryTimerResolution`.
        let status = unsafe { nt_set(max, enable_val, &raw mut new_cur) };

        if status == 0 {
            log::info!(target: "Timer", "Timer resolution adjusted to {:.1}ms (enabled: {})", f64::from(new_cur) / 10000.0, enable);
        } else {
            log::warn!(target: "Timer", "Failed to adjust timer resolution (NTSTATUS: 0x{status:08X})");
        }
    } else {
        log::warn!(target: "Timer", "Could not find timer resolution functions in ntdll.dll");
    }
}

#[cfg(not(windows))]
pub const fn set_fast_timer(_enable: bool) {}

/// Reads `/etc/os-release` to extract the system's human-readable distribution name.
#[cfg(target_os = "linux")]
fn get_linux_distro() -> Option<String> {
    if let Ok(release) = std::fs::read_to_string("/etc/os-release") {
        for line in release.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Gathers and logs detailed OS, CPU, Hostname, Username and physical memory (RAM) specifications.
pub fn log_system_hardware() {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let cpu_identifier =
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "Unknown".to_string());
    let num_processors =
        std::env::var("NUMBER_OF_PROCESSORS").unwrap_or_else(|_| "Unknown".to_string());
    let username = std::env::var(if os == "windows" { "USERNAME" } else { "USER" })
        .unwrap_or_else(|_| "Unknown".to_string());
    let hostname = std::env::var(if os == "windows" {
        "COMPUTERNAME"
    } else {
        "HOSTNAME"
    })
    .unwrap_or_else(|_| {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/etc/hostname")
                .map_or_else(|_| "Unknown".to_string(), |s| s.trim().to_string())
        }
        #[cfg(not(target_os = "linux"))]
        {
            "Unknown".to_string()
        }
    });

    let total_ram = get_memory_info();

    log::info!(target: "Tracking", "OS: {os} | Architecture: {arch}");

    #[cfg(target_os = "linux")]
    if let Some(distro) = get_linux_distro() {
        log::info!(target: "Tracking", "Distribution: {distro}");
    }

    log::info!(target: "Tracking", "Hostname: {hostname} | User: {username}");
    log::info!(target: "Tracking", "CPU Model: {cpu_identifier} | Cores: {num_processors}");
    if let Some(ram) = total_ram {
        log::info!(target: "Tracking", "Total Physical RAM: {:.2} GB", ram as f64 / 1_073_741_824.0);
    } else {
        log::info!(target: "Tracking", "Total Physical RAM: Unknown");
    }
}
