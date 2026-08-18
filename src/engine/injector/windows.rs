//! # Windows Mouse Injection
//!
//! Calls `SendInput` directly instead of going through the `enigo` abstraction
//! that used to sit here. `enigo`'s `Coordinate::Abs` path re-queries
//! `GetSystemMetrics` on every call, and its `Coordinate::Rel` path (with
//! default settings) round-trips through `GetCursorPos` plus another
//! `GetSystemMetrics` call by re-implementing relative motion as an absolute
//! move - three Win32 calls per HID packet on the `TIME_CRITICAL` polling
//! thread. `GetSystemMetrics` is cached here and refreshed on a throttle
//! instead of per packet. `GetCursorPos` is kept for relative mode: the OS
//! cursor is the source of truth, since another input source could move it
//! between our packets, and expressing relative moves as absolute ones keeps
//! them immune to the OS pointer speed/acceleration curve.

use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};

/// Minimum interval between `GetSystemMetrics` refreshes for absolute-mode
/// normalization. Mirrors the 50ms throttle already used for config
/// hot-reload checks in `tablet_manager::polling`: display resolution changes
/// are rare and not latency sensitive, so polling them at a few Hz instead of
/// on every packet removes the syscall from the hot path without risking
/// stale metrics for more than a fraction of a second after a resolution
/// change or monitor hot-plug.
const SCREEN_METRICS_REFRESH: Duration = Duration::from_millis(250);

pub struct Injector {
    /// Tracks the previous state of the primary pen button (tip) to avoid spamming
    /// unnecessary "Button Down" events every frame while dragging.
    last_pressure_down: bool,

    /// Sub-pixel remainder accumulators for relative mode
    remainder_x: f32,
    remainder_y: f32,

    screen_w: i32,
    screen_h: i32,
    last_metrics_refresh: Instant,
}

impl Default for Injector {
    fn default() -> Self {
        Self::new()
    }
}

impl Injector {
    /// Instantiates a new Injector, caching the primary display's pixel size
    /// for absolute-move normalization.
    #[must_use]
    pub fn new() -> Self {
        let (screen_w, screen_h) = query_screen_size();

        log::info!(target: "Injector", "Windows Injector initialized successfully.");

        Self {
            last_pressure_down: false,
            remainder_x: 0.0,
            remainder_y: 0.0,
            screen_w,
            screen_h,
            last_metrics_refresh: Instant::now(),
        }
    }

    /// Updates stylus proximity state. Unused on Windows since proximity
    /// tracking is handled at the system level.
    pub const fn set_proximity(&mut self, _in_proximity: bool) {}

    fn refresh_screen_metrics(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_metrics_refresh) < SCREEN_METRICS_REFRESH {
            return;
        }
        self.last_metrics_refresh = now;

        let (w, h) = query_screen_size();
        if w > 0 && h > 0 {
            self.screen_w = w;
            self.screen_h = h;
        }
    }

    /// Injects an absolute cursor position on the screen.
    /// Used by `Absolute` driver mode.
    ///
    /// # Arguments
    /// * `target_x` - Target X coordinate in OS pixels.
    /// * `target_y` - Target Y coordinate in OS pixels.
    /// * `_u` / `_v` - Normalized UV coordinates (unused on Windows).
    /// * `_pressure` - Pressure (unused on Windows).
    /// * `_tilt_x` / `_tilt_y` - Tilt (unused on Windows).
    #[allow(clippy::too_many_arguments)]
    pub fn move_absolute(
        &mut self,
        target_x: f32,
        target_y: f32,
        _u: f32,
        _v: f32,
        _pressure: i32,
        _tilt_x: i32,
        _tilt_y: i32,
    ) {
        self.refresh_screen_metrics();
        send_absolute_move(target_x, target_y, self.screen_w, self.screen_h);

        // Reset accumulators so relative mode starts clean after a mode switch
        self.remainder_x = 0.0;
        self.remainder_y = 0.0;
    }

    /// Injects relative mouse movement on the screen.
    /// Used by `Relative` driver mode.
    ///
    /// Accumulates sub-pixel movement remainders and emits relative move
    /// events once they accumulate to at least a full integer pixel, applied
    /// against the live OS cursor position so the motion is not subject to
    /// mouse speed/acceleration settings.
    pub fn move_relative(&mut self, dx: f32, dy: f32) {
        let total_dx = dx + self.remainder_x;
        let total_dy = dy + self.remainder_y;

        let ix = total_dx.trunc() as i32;
        let iy = total_dy.trunc() as i32;

        self.remainder_x = total_dx.fract();
        self.remainder_y = total_dy.fract();

        if ix == 0 && iy == 0 {
            return;
        }

        let mut cursor = POINT { x: 0, y: 0 };
        // SAFETY: `cursor` is a valid, writable `POINT` for the duration of the call.
        let ok = unsafe { GetCursorPos(&raw mut cursor) };
        if ok == 0 {
            return;
        }

        self.refresh_screen_metrics();
        send_absolute_move(
            (cursor.x + ix) as f32,
            (cursor.y + iy) as f32,
            self.screen_w,
            self.screen_h,
        );
    }

    /// Synthesizes a Left Mouse Button click or release event.
    ///
    /// The injector maintains internal state and only fires OS events when the
    /// requested `is_down` state differs from the currently held state, preventing
    /// API spam.
    pub fn set_left_button(&mut self, is_down: bool) {
        if is_down == self.last_pressure_down {
            return;
        }

        let flags = if is_down {
            log::debug!(target: "Injector", "Pen tip DOWN (Left Click Pressed)");
            MOUSEEVENTF_LEFTDOWN
        } else {
            log::debug!(target: "Injector", "Pen tip UP (Left Click Released)");
            MOUSEEVENTF_LEFTUP
        };
        self.last_pressure_down = is_down;

        send_mouse_input(0, 0, flags);
    }
}

fn query_screen_size() -> (i32, i32) {
    // SAFETY: `GetSystemMetrics` is a pure metrics query with no preconditions.
    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    // SAFETY: same as above.
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    (w, h)
}

/// Normalizes `(target_x, target_y)` to the 0-65535 range `SendInput` expects
/// for `MOUSEEVENTF_ABSOLUTE` and dispatches the move.
///
/// See <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-mouse_event#remarks>.
fn send_absolute_move(target_x: f32, target_y: f32, screen_w: i32, screen_h: i32) {
    if screen_w <= 1 || screen_h <= 1 {
        return;
    }

    let w = i64::from(screen_w) - 1;
    let h = i64::from(screen_h) - 1;
    let x = target_x.round() as i64;
    let y = target_y.round() as i64;

    // Add w/2 or h/2 (signed) to round to the nearest normalized unit instead of truncating.
    let nx = (x * 65535 + w / 2 * x.signum()) / w;
    let ny = (y * 65535 + h / 2 * y.signum()) / h;

    send_mouse_input(
        nx as i32,
        ny as i32,
        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
    );
}

fn send_mouse_input(dx: i32, dy: i32, flags: u32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let cbsize = i32::try_from(size_of::<INPUT>()).unwrap_or(i32::MAX);

    // SAFETY: `input` is a fully-initialized, valid `INPUT` for the duration of the call.
    unsafe {
        SendInput(1, &raw const input, cbsize);
    }
}
