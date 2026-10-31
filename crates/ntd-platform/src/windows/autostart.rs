//! Windows autostart: a value in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
#![allow(unsafe_code, reason = "Win32 registry calls through windows-sys")]

use crate::{APP_NAME, PlatformError, UserPaths};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::{env, fs};
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn to_wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// Builds the command line stored in the `Run` key: the quoted executable path followed
/// by the `--autostart` flag. Quoting keeps paths containing spaces working and avoids
/// the classic unquoted-path ambiguity.
fn autostart_command(exe: &Path) -> String {
    format!("\"{}\" --autostart", exe.display())
}

/// RAII wrapper so the registry key is always closed.
struct RunKey(HKEY);

impl RunKey {
    fn open(access: u32, create: bool) -> Result<Self, PlatformError> {
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
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, access, &raw mut key) }
        };
        if status == ERROR_SUCCESS {
            Ok(Self(key))
        } else {
            Err(PlatformError::Os(format!(
                "Cannot open the Run registry key (error {status})"
            )))
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
fn remove_legacy_shortcut(paths: &UserPaths) {
    let shortcut: PathBuf = paths
        .home
        .join(r"AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup")
        .join(format!("{APP_NAME}.lnk"));
    if shortcut.exists() && fs::remove_file(&shortcut).is_ok() {
        log::info!(target: "Startup", "Removed legacy startup shortcut: \
            {}", shortcut.display());
    }
}

fn write_value(name: &str, command: &str) -> Result<(), PlatformError> {
    let name = to_wide(name);
    let value = to_wide(command);
    let key = RunKey::open(KEY_SET_VALUE, true)?;
    let byte_len = u32::try_from(value.len() * std::mem::size_of::<u16>())
        .map_err(|e| PlatformError::Os(e.to_string()))?;
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
        Err(PlatformError::Os(format!(
            "Failed to write the startup registry value (error {status})"
        )))
    }
}

fn delete_value(name: &str) -> Result<(), PlatformError> {
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
        Err(PlatformError::Os(format!(
            "Failed to remove the startup registry value (error {status})"
        )))
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
pub(crate) fn set_run_at_startup(enabled: bool, paths: &UserPaths) -> Result<(), PlatformError> {
    remove_legacy_shortcut(paths);

    if enabled {
        let command =
            autostart_command(&env::current_exe().map_err(|e| PlatformError::Os(e.to_string()))?);
        write_value(APP_NAME, &command)?;
        log::info!(target: "Startup", "Registered run-at-startup: {command}");
    } else {
        delete_value(APP_NAME)?;
        log::info!(target: "Startup", "Removed run-at-startup registration");
    }
    Ok(())
}

/// Checks if the application is currently configured to run at startup.
pub(crate) fn is_run_at_startup_registered(_paths: &UserPaths) -> bool {
    has_value(APP_NAME)
}

#[cfg(test)]
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
