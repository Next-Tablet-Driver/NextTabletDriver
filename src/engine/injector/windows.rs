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
#![allow(
    unsafe_code,
    reason = "Win32 cursor and input injection calls through windows-sys"
)]

use crate::core::input::{Key, MouseButton};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
    SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, XBUTTON1, XBUTTON2,
};

/// Minimum interval between `GetSystemMetrics` refreshes for absolute-mode
/// normalization. Mirrors the 50ms throttle already used for config
/// hot-reload checks in `tablet_manager::polling`: display resolution changes
/// are rare and not latency sensitive, so polling them at a few Hz instead of
/// on every packet removes the syscall from the hot path without risking
/// stale metrics for more than a fraction of a second after a resolution
/// change or monitor hot-plug.
const SCREEN_METRICS_REFRESH: Duration = Duration::from_millis(250);

/// Sends the processed pen frames to Windows as cursor movement and button events.
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

/// Creating the injector cannot fail on Windows; the alias keeps `Injector::try_new` shaped
/// like the Linux one.
pub type InjectorError = std::convert::Infallible;

impl Default for Injector {
    fn default() -> Self {
        Self::new()
    }
}

impl Injector {
    /// Fallible constructor, mirroring the Linux injector (which can fail to open `/dev/uinput`).
    ///
    /// # Errors
    /// Never fails on Windows; the `Result` keeps the cross-platform API uniform.
    #[must_use]
    pub fn try_new() -> Result<Self, InjectorError> {
        Ok(Self::new())
    }

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
    #[allow(
        clippy::too_many_arguments,
        reason = "implements the `InputSink` method of the same signature"
    )]
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

    /// Presses or releases a mouse button on behalf of a button binding.
    pub fn set_mouse_button(&mut self, button: MouseButton, is_down: bool) {
        let (flags, data) = mouse_button_event(button, is_down);
        send_mouse_input_with_data(0, 0, data, flags);
    }

    /// Presses or releases a keyboard key on behalf of a button binding.
    ///
    /// The key is sent as a scan code, like a real keyboard does, because games and other
    /// programs that read the keyboard directly ignore events made of a virtual key only.
    pub fn set_key(&mut self, key: Key, is_down: bool) {
        send_key_input(key, is_down);
    }
}

/// The `SendInput` flags and extra data that press or release `button`.
const fn mouse_button_event(button: MouseButton, is_down: bool) -> (u32, u32) {
    match (button, is_down) {
        (MouseButton::Left, true) => (MOUSEEVENTF_LEFTDOWN, 0),
        (MouseButton::Left, false) => (MOUSEEVENTF_LEFTUP, 0),
        (MouseButton::Right, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
        (MouseButton::Right, false) => (MOUSEEVENTF_RIGHTUP, 0),
        (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
        (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
        (MouseButton::Back, true) => (MOUSEEVENTF_XDOWN, XBUTTON1 as u32),
        (MouseButton::Back, false) => (MOUSEEVENTF_XUP, XBUTTON1 as u32),
        (MouseButton::Forward, true) => (MOUSEEVENTF_XDOWN, XBUTTON2 as u32),
        (MouseButton::Forward, false) => (MOUSEEVENTF_XUP, XBUTTON2 as u32),
    }
}

/// The hardware scan code of `key` (set 1), by position on a US keyboard.
///
/// The code does not depend on the keyboard layout, so the key that was recorded in the
/// interface is the key that is pressed here, on any layout. Windows turns it into the right
/// character for the layout in use.
const fn scan_code(key: Key) -> u16 {
    match key {
        Key::A => 0x1E,
        Key::B => 0x30,
        Key::C => 0x2E,
        Key::D => 0x20,
        Key::E => 0x12,
        Key::F => 0x21,
        Key::G => 0x22,
        Key::H => 0x23,
        Key::I => 0x17,
        Key::J => 0x24,
        Key::K => 0x25,
        Key::L => 0x26,
        Key::M => 0x32,
        Key::N => 0x31,
        Key::O => 0x18,
        Key::P => 0x19,
        Key::Q => 0x10,
        Key::R => 0x13,
        Key::S => 0x1F,
        Key::T => 0x14,
        Key::U => 0x16,
        Key::V => 0x2F,
        Key::W => 0x11,
        Key::X => 0x2D,
        Key::Y => 0x15,
        Key::Z => 0x2C,
        Key::Num1 => 0x02,
        Key::Num2 => 0x03,
        Key::Num3 => 0x04,
        Key::Num4 => 0x05,
        Key::Num5 => 0x06,
        Key::Num6 => 0x07,
        Key::Num7 => 0x08,
        Key::Num8 => 0x09,
        Key::Num9 => 0x0A,
        Key::Num0 => 0x0B,
        Key::F1 => 0x3B,
        Key::F2 => 0x3C,
        Key::F3 => 0x3D,
        Key::F4 => 0x3E,
        Key::F5 => 0x3F,
        Key::F6 => 0x40,
        Key::F7 => 0x41,
        Key::F8 => 0x42,
        Key::F9 => 0x43,
        Key::F10 => 0x44,
        Key::F11 => 0x57,
        Key::F12 => 0x58,
        Key::F13 => 0x64,
        Key::F14 => 0x65,
        Key::F15 => 0x66,
        Key::F16 => 0x67,
        Key::F17 => 0x68,
        Key::F18 => 0x69,
        Key::F19 => 0x6A,
        Key::F20 => 0x6B,
        Key::F21 => 0x6C,
        Key::F22 => 0x6D,
        Key::F23 => 0x6E,
        Key::F24 => 0x76,
        Key::Escape => 0x01,
        Key::Tab => 0x0F,
        Key::Enter => 0x1C,
        Key::Space => 0x39,
        Key::Backspace => 0x0E,
        Key::Delete => 0x53,
        Key::Insert => 0x52,
        Key::Home => 0x47,
        Key::End => 0x4F,
        Key::PageUp => 0x49,
        Key::PageDown => 0x51,
        Key::Up => 0x48,
        Key::Down => 0x50,
        Key::Left => 0x4B,
        Key::Right => 0x4D,
        Key::Ctrl => 0x1D,
        Key::Shift => 0x2A,
        Key::Alt => 0x38,
        Key::Meta => 0x5B,
        Key::Minus => 0x0C,
        Key::Equals => 0x0D,
        Key::LeftBracket => 0x1A,
        Key::RightBracket => 0x1B,
        Key::Backslash => 0x2B,
        Key::Semicolon => 0x27,
        Key::Quote => 0x28,
        Key::Backtick => 0x29,
        Key::Comma => 0x33,
        Key::Period => 0x34,
        Key::Slash => 0x35,
    }
}

/// Keys whose scan code needs the "extended" flag (the grey keys next to the main block).
const fn is_extended_key(key: Key) -> bool {
    matches!(
        key,
        Key::Delete
            | Key::Insert
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::Up
            | Key::Down
            | Key::Left
            | Key::Right
            | Key::Meta
    )
}

fn send_key_input(key: Key, is_down: bool) {
    let mut flags = KEYEVENTF_SCANCODE;
    if !is_down {
        flags |= KEYEVENTF_KEYUP;
    }
    if is_extended_key(key) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: scan_code(key),
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
    if let Some((nx, ny)) = normalize_absolute(target_x, target_y, screen_w, screen_h) {
        send_mouse_input(nx, ny, MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE);
    }
}

/// Maps a pixel position to the 0-65535 range of `MOUSEEVENTF_ABSOLUTE`; `None` for a screen too
/// small to normalize against.
fn normalize_absolute(
    target_x: f32,
    target_y: f32,
    screen_w: i32,
    screen_h: i32,
) -> Option<(i32, i32)> {
    if screen_w <= 1 || screen_h <= 1 {
        return None;
    }

    let w = i64::from(screen_w) - 1;
    let h = i64::from(screen_h) - 1;
    let x = target_x.round() as i64;
    let y = target_y.round() as i64;

    // Add w/2 or h/2 (signed) to round to the nearest normalized unit instead of truncating.
    let nx = (x * 65535 + w / 2 * x.signum()) / w;
    let ny = (y * 65535 + h / 2 * y.signum()) / h;
    Some((nx as i32, ny as i32))
}

fn send_mouse_input(dx: i32, dy: i32, flags: u32) {
    send_mouse_input_with_data(dx, dy, 0, flags);
}

fn send_mouse_input_with_data(dx: i32, dy: i32, mouse_data: u32, flags: u32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: mouse_data,
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

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;

    #[test]
    fn the_injector_starts_with_the_screen_size_and_no_button_held() {
        let injector = Injector::try_new().unwrap();
        assert!(!injector.last_pressure_down);
        assert_eq!((injector.remainder_x, injector.remainder_y), (0.0, 0.0));
        assert!(injector.screen_w > 0 && injector.screen_h > 0);
        let default = Injector::default();
        assert!(default.screen_w > 0);
    }

    #[test]
    fn the_screen_metrics_are_not_re_queried_within_the_throttle() {
        let mut injector = Injector::new();
        injector.screen_w = 1234;
        injector.screen_h = 567;
        injector.refresh_screen_metrics();
        assert_eq!((injector.screen_w, injector.screen_h), (1234, 567));
    }

    #[test]
    fn the_screen_metrics_are_refreshed_once_the_throttle_has_elapsed() {
        let mut injector = Injector::new();
        injector.screen_w = 1;
        injector.screen_h = 1;
        injector.last_metrics_refresh = Instant::now()
            .checked_sub(SCREEN_METRICS_REFRESH * 2)
            .unwrap();
        injector.refresh_screen_metrics();
        assert!(injector.screen_w > 1 && injector.screen_h > 1);
    }

    #[test]
    fn sub_pixel_relative_motion_accumulates_without_touching_the_cursor() {
        let mut injector = Injector::new();
        injector.move_relative(0.4, -0.3);
        assert!((injector.remainder_x - 0.4).abs() < 1e-6);
        assert!((injector.remainder_y + 0.3).abs() < 1e-6);
        injector.move_relative(0.4, -0.3);
        assert!((injector.remainder_x - 0.8).abs() < 1e-6);
        assert!((injector.remainder_y + 0.6).abs() < 1e-6);
    }

    #[test]
    fn releasing_a_button_that_is_not_held_does_nothing() {
        let mut injector = Injector::new();
        injector.set_left_button(false);
        assert!(!injector.last_pressure_down);
        injector.set_proximity(true);
    }

    #[test]
    fn each_mouse_button_has_its_own_down_and_up_event() {
        assert_eq!(
            mouse_button_event(MouseButton::Right, true),
            (MOUSEEVENTF_RIGHTDOWN, 0)
        );
        assert_eq!(
            mouse_button_event(MouseButton::Right, false),
            (MOUSEEVENTF_RIGHTUP, 0)
        );
        assert_eq!(
            mouse_button_event(MouseButton::Middle, true),
            (MOUSEEVENTF_MIDDLEDOWN, 0)
        );
    }

    #[test]
    fn the_side_buttons_are_told_apart_by_the_extra_data() {
        assert_eq!(
            mouse_button_event(MouseButton::Back, true),
            (MOUSEEVENTF_XDOWN, u32::from(XBUTTON1))
        );
        assert_eq!(
            mouse_button_event(MouseButton::Forward, false),
            (MOUSEEVENTF_XUP, u32::from(XBUTTON2))
        );
    }

    #[test]
    fn keys_use_the_scan_codes_of_their_position_on_a_us_keyboard() {
        assert_eq!(scan_code(Key::A), 0x1E);
        assert_eq!(scan_code(Key::Q), 0x10);
        assert_eq!(scan_code(Key::Z), 0x2C);
        assert_eq!(scan_code(Key::Num1), 0x02);
        assert_eq!(scan_code(Key::Num0), 0x0B);
        assert_eq!(scan_code(Key::F1), 0x3B);
        assert_eq!(scan_code(Key::F12), 0x58);
        assert_eq!(scan_code(Key::Escape), 0x01);
        assert_eq!(scan_code(Key::Space), 0x39);
        assert_eq!(scan_code(Key::Ctrl), 0x1D);
    }

    #[test]
    fn no_two_keys_share_a_scan_code_and_extended_flag() {
        for (index, &key) in Key::ALL.iter().enumerate() {
            for &other in Key::ALL.iter().skip(index + 1) {
                let same = scan_code(key) == scan_code(other)
                    && is_extended_key(key) == is_extended_key(other);
                assert!(!same, "{key:?} / {other:?}");
            }
        }
    }

    #[test]
    fn only_the_grey_keys_are_extended() {
        assert!(is_extended_key(Key::Left));
        assert!(is_extended_key(Key::Delete));
        assert!(!is_extended_key(Key::A));
        assert!(!is_extended_key(Key::Enter));
    }

    #[test]
    fn absolute_positions_are_normalized_to_the_sendinput_range() {
        // 1920x1080: the last pixel maps to 65535, the first to 0, the middle to about half.
        assert_eq!(normalize_absolute(0.0, 0.0, 1920, 1080), Some((0, 0)));
        assert_eq!(
            normalize_absolute(1919.0, 1079.0, 1920, 1080),
            Some((65535, 65535))
        );
        let (mx, my) = normalize_absolute(960.0, 540.0, 1920, 1080).unwrap();
        assert!((32_700..=32_900).contains(&mx), "{mx}");
        assert!((32_700..=32_900).contains(&my), "{my}");
    }

    #[test]
    fn positions_are_rounded_to_the_nearest_pixel() {
        assert_eq!(
            normalize_absolute(10.4, 10.4, 1920, 1080),
            normalize_absolute(10.0, 10.0, 1920, 1080)
        );
        assert_ne!(
            normalize_absolute(10.6, 10.0, 1920, 1080),
            normalize_absolute(10.0, 10.0, 1920, 1080)
        );
    }

    #[test]
    fn positions_left_of_or_above_the_screen_stay_negative() {
        let (nx, ny) = normalize_absolute(-10.0, -10.0, 1920, 1080).unwrap();
        assert!(nx < 0 && ny < 0);
    }

    #[test]
    fn a_screen_too_small_to_normalize_against_is_ignored() {
        assert_eq!(normalize_absolute(5.0, 5.0, 1, 1080), None);
        assert_eq!(normalize_absolute(5.0, 5.0, 1920, 0), None);
        // Nothing is sent either (and nothing can move the cursor in this test).
        send_absolute_move(5.0, 5.0, 0, 0);
    }
}
