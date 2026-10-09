// Always a GUI program on Windows, debug builds included: a console-subsystem binary opens a
// terminal window next to the app whenever it is launched outside a terminal.
#![windows_subsystem = "windows"]

fn main() {
    #[cfg(all(windows, debug_assertions))]
    attach_parent_console();

    app_lib::run();
}

/// Debug builds only: a GUI process has no console, so the logs of a `cargo run` started from a
/// terminal would be lost. Reuse the parent's console when there is one (this never opens a
/// window) and leave stdout/stderr alone when they are already redirected to a pipe or a file.
#[cfg(all(windows, debug_assertions))]
fn attach_parent_console() {
    use std::fs::OpenOptions;
    use std::os::windows::io::IntoRawHandle;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        SetStdHandle,
    };

    // SAFETY: no pointer is passed; the call fails harmlessly when there is no parent console.
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0 {
        return;
    }

    for id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: plain query of this process's standard handle.
        let current = unsafe { GetStdHandle(id) };
        if !current.is_null() && current != INVALID_HANDLE_VALUE {
            continue;
        }
        if let Ok(console) = OpenOptions::new().write(true).open("CONOUT$") {
            // SAFETY: the handle is leaked on purpose so it stays valid for the whole process.
            unsafe { SetStdHandle(id, console.into_raw_handle()) };
        }
    }
}
