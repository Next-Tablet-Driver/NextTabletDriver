//! Linux injection through virtual `/dev/uinput` devices (an absolute pen and a relative mouse).

use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, InputEvent, InputId, KeyCode,
    RelativeAxisCode, UinputAbsSetup, uinput::VirtualDevice,
};
use ntd_core::input::{Key, MouseButton};

/// Maximum value for absolute axes (standard high-resolution range).
/// The compositor/X server will map this to the actual screen dimensions.
const ABS_MAX: i32 = 32767;

/// Maximum pressure value exposed by the virtual tablet.
/// Individual tablets may have different maximums, but we normalize to this range.
const PRESSURE_MAX: i32 = 8191;

/// Sends the processed pen frames to the kernel as events of two `/dev/uinput` devices.
pub struct Injector {
    /// The virtual tablet device for absolute coordinate injection.
    /// Registered as a pen digitizer in the kernel via `/dev/uinput`.
    virtual_tablet: evdev::uinput::VirtualDevice,

    /// A separate virtual mouse device for relative movement injection.
    /// Some compositors handle relative events differently from tablet events,
    /// so we keep them on separate virtual devices.
    virtual_mouse: evdev::uinput::VirtualDevice,

    /// A virtual keyboard used by button bindings that press keys.
    virtual_keyboard: evdev::uinput::VirtualDevice,

    /// Tracks the previous state of the primary pen button (tip).
    last_pressure_down: bool,

    /// Tracks the previous proximity state of the stylus (in range).
    last_proximity: Option<bool>,

    /// Sub-pixel remainder accumulators for relative mode
    remainder_x: f32,
    remainder_y: f32,

    /// Bounding box of the virtual desktop: (`min_x`, `min_y`, `max_x`, `max_y`)
    screen_bounds: (f32, f32, f32, f32),
    /// Timestamp of the last time we updated `screen_bounds`
    last_bounds_update: std::time::Instant,
    /// Set once a display enumeration failure has been logged, so the warning
    /// is not repeated on every retry while the fallback bounds are in use.
    bounds_query_failed: bool,
    /// Number of displays seen on the last successful enumeration, used to
    /// decide when a bounds change is worth logging (e.g. monitor hot-plug).
    last_known_display_count: Option<usize>,
}

const UINPUT_HELP: &str = "Failed to open /dev/uinput. This typically means the uinput kernel \
module is not loaded or you don't have permission to write to it.\n\
1. Load the module: sudo modprobe uinput\n\
2. Allow the 'input' group to use it: echo 'KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\", \
OPTIONS+=\"static_node=uinput\"' | sudo tee /etc/udev/rules.d/99-uinput.rules\n\
3. Add your user to the group: sudo usermod -aG input $USER\n\
4. Log out/in, or run: sudo udevadm control --reload-rules \
&& sudo udevadm trigger --sysname-match=uinput";

/// Errors produced while creating the virtual input devices.
#[derive(Debug, thiserror::Error)]
pub enum InjectorError {
    /// `/dev/uinput` could not be opened (missing permissions or kernel module).
    #[error("{source}. {}", UINPUT_HELP)]
    Uinput {
        /// The error reported by the kernel.
        #[source]
        source: std::io::Error,
    },
    /// A virtual device could not be configured or created.
    #[error("Failed to {what}: {source}")]
    Setup {
        /// The step that failed, for example `set ABS_X`.
        what: &'static str,
        /// The error reported by the kernel.
        #[source]
        source: std::io::Error,
    },
}

/// Builds a `map_err` closure that names the setup step that failed.
const fn setup(what: &'static str) -> impl FnOnce(std::io::Error) -> InjectorError {
    move |source| InjectorError::Setup { what, source }
}

fn create_uinput_device_builder()
-> Result<evdev::uinput::VirtualDeviceBuilder<'static>, InjectorError> {
    VirtualDevice::builder().map_err(|source| {
        log::error!(target: "Injector", "CRITICAL ERROR: {source}
{UINPUT_HELP}");
        InjectorError::Uinput { source }
    })
}

impl Injector {
    /// Creates both virtual devices via `/dev/uinput`: a virtual tablet reporting
    /// `EV_ABS` with `ABS_X`, `ABS_Y`, `ABS_PRESSURE`, `ABS_TILT_X`, `ABS_TILT_Y`, and key
    /// events for `BTN_TOUCH`, `BTN_TOOL_PEN`, `BTN_STYLUS`, `BTN_STYLUS2`; and a virtual
    /// mouse reporting `EV_REL` with `REL_X`, `REL_Y`, and `BTN_LEFT` for relative mode
    /// operation.
    ///
    /// # Errors
    /// Returns a human-readable error if `/dev/uinput` cannot be opened (missing
    /// permissions or kernel module) or a virtual device cannot be configured.
    pub fn try_new() -> Result<Self, InjectorError> {
        let mut tablet_keys = AttributeSet::<KeyCode>::new();
        tablet_keys.insert(KeyCode::BTN_TOUCH);
        tablet_keys.insert(KeyCode::BTN_TOOL_PEN);
        tablet_keys.insert(KeyCode::BTN_STYLUS);
        tablet_keys.insert(KeyCode::BTN_STYLUS2);

        let virtual_tablet = create_uinput_device_builder()?
            .name("NextTabletDriver Virtual Pen")
            .input_id(InputId::new(BusType::BUS_USB, 0x0001, 0x0001, 1))
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_X,
                AbsInfo::new(0, 0, ABS_MAX, 0, 0, 100),
            ))
            .map_err(setup("set ABS_X"))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_Y,
                AbsInfo::new(0, 0, ABS_MAX, 0, 0, 100),
            ))
            .map_err(setup("set ABS_Y"))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_PRESSURE,
                AbsInfo::new(0, 0, PRESSURE_MAX, 0, 0, 0),
            ))
            .map_err(setup("set ABS_PRESSURE"))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_TILT_X,
                AbsInfo::new(0, -127, 127, 0, 0, 0),
            ))
            .map_err(setup("set ABS_TILT_X"))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_TILT_Y,
                AbsInfo::new(0, -127, 127, 0, 0, 0),
            ))
            .map_err(setup("set ABS_TILT_Y"))?
            .with_keys(&tablet_keys)
            .map_err(setup("set tablet keys"))?
            .build()
            .map_err(setup("create virtual tablet device"))?;

        log::info!(target: "Injector", "Virtual tablet device created: NextTabletDriver Virtual \
            Pen");

        let mut mouse_keys = AttributeSet::<KeyCode>::new();
        mouse_keys.insert(KeyCode::BTN_LEFT);
        mouse_keys.insert(KeyCode::BTN_RIGHT);
        mouse_keys.insert(KeyCode::BTN_MIDDLE);
        mouse_keys.insert(KeyCode::BTN_SIDE);
        mouse_keys.insert(KeyCode::BTN_EXTRA);

        let mut rel_axes = AttributeSet::<RelativeAxisCode>::new();
        rel_axes.insert(RelativeAxisCode::REL_X);
        rel_axes.insert(RelativeAxisCode::REL_Y);

        let virtual_mouse = create_uinput_device_builder()?
            .name("NextTabletDriver Virtual Mouse")
            .input_id(InputId::new(BusType::BUS_USB, 0x0001, 0x0002, 1))
            .with_relative_axes(&rel_axes)
            .map_err(setup("set REL axes"))?
            .with_keys(&mouse_keys)
            .map_err(setup("set mouse keys"))?
            .build()
            .map_err(setup("create virtual mouse device"))?;

        log::info!(target: "Injector", "Virtual mouse device created: NextTabletDriver Virtual \
            Mouse");

        let mut keyboard_keys = AttributeSet::<KeyCode>::new();
        for &key in Key::ALL {
            keyboard_keys.insert(evdev_key(key));
        }

        let virtual_keyboard = create_uinput_device_builder()?
            .name("NextTabletDriver Virtual Keyboard")
            .input_id(InputId::new(BusType::BUS_USB, 0x0001, 0x0003, 1))
            .with_keys(&keyboard_keys)
            .map_err(setup("set keyboard keys"))?
            .build()
            .map_err(setup("create virtual keyboard device"))?;

        log::info!(target: "Injector", "Virtual keyboard device created: NextTabletDriver Virtual \
            Keyboard");

        let now = std::time::Instant::now();
        Ok(Self {
            virtual_tablet,
            virtual_mouse,
            virtual_keyboard,
            last_pressure_down: false,
            last_proximity: None,
            remainder_x: 0.0,
            remainder_y: 0.0,
            screen_bounds: (0.0, 0.0, 1920.0, 1080.0),
            last_bounds_update: now
                .checked_sub(std::time::Duration::from_secs(10))
                .unwrap_or(now),
            bounds_query_failed: false,
            last_known_display_count: None,
        })
    }

    /// Refreshes `screen_bounds` from the current display layout, at most once
    /// every 2 seconds. `display_info` queries `wl_output` on Wayland and
    /// `RandR` on X11; both are cheap but not free, hence the rate limit.
    ///
    /// If enumeration fails or reports no displays, the previous bounds are
    /// kept (initially the hardcoded 1920x1080 fallback), and a warning is
    /// logged once so the AREA mismatch is diagnosable instead of silent.
    fn update_screen_bounds(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_bounds_update) < std::time::Duration::from_secs(2) {
            return;
        }
        self.last_bounds_update = now;

        match display_info::DisplayInfo::all() {
            Ok(displays) if !displays.is_empty() => {
                let mut mx = i32::MAX;
                let mut my = i32::MAX;
                let mut ax = i32::MIN;
                let mut ay = i32::MIN;
                for d in &displays {
                    mx = mx.min(d.x);
                    my = my.min(d.y);
                    ax = ax.max(d.x + d.width.cast_signed());
                    ay = ay.max(d.y + d.height.cast_signed());
                }
                self.screen_bounds = (mx as f32, my as f32, ax as f32, ay as f32);

                let display_count = displays.len();
                if self.bounds_query_failed || self.last_known_display_count != Some(display_count)
                {
                    log::info!(
                        target: "Injector",
                        "Screen bounds resolved from {display_count} display(s): desktop spans \
                            ({mx}, {my}) to ({ax}, {ay})"
                    );
                }

                self.bounds_query_failed = false;
                self.last_known_display_count = Some(display_count);
            }
            Ok(_) => {
                self.warn_bounds_fallback("Display enumeration returned zero displays");
            }
            Err(e) => {
                let reason = format!("Failed to enumerate displays: {e}");
                self.warn_bounds_fallback(&reason);
            }
        }
    }

    /// Logs a one-time warning explaining that AREA mapping is running on the
    /// hardcoded fallback bounds because display geometry could not be read.
    /// Rearmed automatically once a later enumeration succeeds.
    fn warn_bounds_fallback(&mut self, reason: &str) {
        if self.bounds_query_failed {
            return;
        }
        self.bounds_query_failed = true;

        let (min_x, min_y, max_x, max_y) = self.screen_bounds;
        log::warn!(
            target: "Injector",
            "{reason}. AREA mapping will use the hardcoded fallback area ({min_x}, {min_y}) to \
            ({max_x}, {max_y}) until display geometry becomes available. On Wayland this \
            commonly happens when the compositor does not expose output geometry through the \
            standard wl_output protocol."
        );
    }

    /// Reports the pen entering or leaving the tablet range, only when the state changes.
    pub fn set_proximity(&mut self, in_proximity: bool) {
        if Some(in_proximity) == self.last_proximity {
            return;
        }

        let value = i32::from(in_proximity);
        let events = [
            InputEvent::new(evdev::EventType::KEY.0, KeyCode::BTN_TOOL_PEN.0, value),
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_tablet.emit(&events) {
            log::error!(target: "Injector", "Failed to emit proximity event: {e}");
        }

        self.last_proximity = Some(in_proximity);
    }

    /// Injects an absolute pen position on the screen.
    /// Used by `Absolute` driver mode.
    ///
    /// On Linux, we write `ABS_X` and `ABS_Y` events to the uinput virtual tablet.
    /// The values are mapped relative to the total screen dimensions across all displays.
    ///
    /// # Arguments
    /// * `target_x` / `target_y` - Screen pixel coordinates.
    /// * `u` / `v` - Normalized UV coordinates in [0.0, 1.0] from the pipeline.
    /// * `pressure` - Normalized pressure.
    /// * `tilt_x` / `tilt_y` - Absolute tilt values.
    #[allow(
        clippy::too_many_arguments,
        reason = "implements the `OutputSink` method of the same signature"
    )]
    pub fn move_absolute(
        &mut self,
        target_x: f32,
        target_y: f32,
        u: f32,
        v: f32,
        pressure: i32,
        tilt_x: i32,
        tilt_y: i32,
    ) {
        self.set_proximity(true);

        self.update_screen_bounds();

        let (min_x, min_y, max_x, max_y) = self.screen_bounds;
        let desk_w = max_x - min_x;
        let desk_h = max_y - min_y;

        let (abs_x, abs_y) = if desk_w > 0.0 && desk_h > 0.0 {
            (
                (((target_x - min_x) / desk_w).clamp(0.0, 1.0) * ABS_MAX as f32) as i32,
                (((target_y - min_y) / desk_h).clamp(0.0, 1.0) * ABS_MAX as f32) as i32,
            )
        } else {
            (
                (u.clamp(0.0, 1.0) * ABS_MAX as f32) as i32,
                (v.clamp(0.0, 1.0) * ABS_MAX as f32) as i32,
            )
        };

        let pressure = pressure.clamp(0, PRESSURE_MAX);
        let tilt_x = tilt_x.clamp(-127, 127);
        let tilt_y = tilt_y.clamp(-127, 127);

        let events = [
            InputEvent::new(
                evdev::EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_X.0,
                abs_x,
            ),
            InputEvent::new(
                evdev::EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_Y.0,
                abs_y,
            ),
            InputEvent::new(
                evdev::EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_PRESSURE.0,
                pressure,
            ),
            InputEvent::new(
                evdev::EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_TILT_X.0,
                tilt_x,
            ),
            InputEvent::new(
                evdev::EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_TILT_Y.0,
                tilt_y,
            ),
            // SYN_REPORT to flush the event packet
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_tablet.emit(&events) {
            log::error!(target: "Injector", "Failed to emit absolute events: {e}");
        }
    }

    /// Injects relative mouse movement.
    /// Used by `Relative` driver mode.
    pub fn move_relative(&mut self, dx: f32, dy: f32) {
        let total_dx = dx + self.remainder_x;
        let total_dy = dy + self.remainder_y;

        let ix = total_dx.trunc() as i32;
        let iy = total_dy.trunc() as i32;

        self.remainder_x = total_dx.fract();
        self.remainder_y = total_dy.fract();

        if ix != 0 || iy != 0 {
            let events = [
                InputEvent::new(evdev::EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, ix),
                InputEvent::new(evdev::EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, iy),
                InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
            ];

            if let Err(e) = self.virtual_mouse.emit(&events) {
                log::error!(target: "Injector", "Failed to emit relative events: {e}");
            }
        }
    }

    /// Synthesizes a pen tip press/release event.
    ///
    /// On Linux, we emit `BTN_TOUCH` on the virtual tablet device (in absolute mode)
    /// or `BTN_LEFT` on the virtual mouse (in relative mode).
    /// The injector maintains internal state and only fires events on state transitions.
    pub fn set_left_button(&mut self, is_down: bool) {
        if is_down == self.last_pressure_down {
            return;
        }

        let value = i32::from(is_down);

        let events = [
            InputEvent::new(evdev::EventType::KEY.0, KeyCode::BTN_TOUCH.0, value),
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_tablet.emit(&events) {
            log::error!(target: "Injector", "Failed to emit button event: {e}");
        }

        // BTN_LEFT on mouse device for apps that don't honor BTN_TOUCH
        let mouse_events = [
            InputEvent::new(evdev::EventType::KEY.0, KeyCode::BTN_LEFT.0, value),
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_mouse.emit(&mouse_events) {
            log::error!(target: "Injector", "Failed to emit mouse button event: {e}");
        }

        self.last_pressure_down = is_down;
    }

    /// Presses or releases a mouse button on behalf of a button binding.
    pub fn set_mouse_button(&mut self, button: MouseButton, is_down: bool) {
        let events = [
            InputEvent::new(
                evdev::EventType::KEY.0,
                evdev_mouse_button(button).0,
                i32::from(is_down),
            ),
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_mouse.emit(&events) {
            log::error!(target: "Injector", "Failed to emit mouse button event: {e}");
        }
    }

    /// Presses or releases a keyboard key on behalf of a button binding.
    pub fn set_key(&mut self, key: Key, is_down: bool) {
        let events = [
            InputEvent::new(
                evdev::EventType::KEY.0,
                evdev_key(key).0,
                i32::from(is_down),
            ),
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_keyboard.emit(&events) {
            log::error!(target: "Injector", "Failed to emit key event: {e}");
        }
    }
}

/// The Linux button code of a mouse button.
const fn evdev_mouse_button(button: MouseButton) -> KeyCode {
    match button {
        MouseButton::Left => KeyCode::BTN_LEFT,
        MouseButton::Right => KeyCode::BTN_RIGHT,
        MouseButton::Middle => KeyCode::BTN_MIDDLE,
        MouseButton::Back => KeyCode::BTN_SIDE,
        MouseButton::Forward => KeyCode::BTN_EXTRA,
    }
}

/// The Linux key code of `key`. Linux names keys by their place on a US keyboard, which is
/// how [`Key`] is defined, so there is a direct match for each one.
const fn evdev_key(key: Key) -> KeyCode {
    match key {
        Key::A => KeyCode::KEY_A,
        Key::B => KeyCode::KEY_B,
        Key::C => KeyCode::KEY_C,
        Key::D => KeyCode::KEY_D,
        Key::E => KeyCode::KEY_E,
        Key::F => KeyCode::KEY_F,
        Key::G => KeyCode::KEY_G,
        Key::H => KeyCode::KEY_H,
        Key::I => KeyCode::KEY_I,
        Key::J => KeyCode::KEY_J,
        Key::K => KeyCode::KEY_K,
        Key::L => KeyCode::KEY_L,
        Key::M => KeyCode::KEY_M,
        Key::N => KeyCode::KEY_N,
        Key::O => KeyCode::KEY_O,
        Key::P => KeyCode::KEY_P,
        Key::Q => KeyCode::KEY_Q,
        Key::R => KeyCode::KEY_R,
        Key::S => KeyCode::KEY_S,
        Key::T => KeyCode::KEY_T,
        Key::U => KeyCode::KEY_U,
        Key::V => KeyCode::KEY_V,
        Key::W => KeyCode::KEY_W,
        Key::X => KeyCode::KEY_X,
        Key::Y => KeyCode::KEY_Y,
        Key::Z => KeyCode::KEY_Z,
        Key::Num0 => KeyCode::KEY_0,
        Key::Num1 => KeyCode::KEY_1,
        Key::Num2 => KeyCode::KEY_2,
        Key::Num3 => KeyCode::KEY_3,
        Key::Num4 => KeyCode::KEY_4,
        Key::Num5 => KeyCode::KEY_5,
        Key::Num6 => KeyCode::KEY_6,
        Key::Num7 => KeyCode::KEY_7,
        Key::Num8 => KeyCode::KEY_8,
        Key::Num9 => KeyCode::KEY_9,
        Key::F1 => KeyCode::KEY_F1,
        Key::F2 => KeyCode::KEY_F2,
        Key::F3 => KeyCode::KEY_F3,
        Key::F4 => KeyCode::KEY_F4,
        Key::F5 => KeyCode::KEY_F5,
        Key::F6 => KeyCode::KEY_F6,
        Key::F7 => KeyCode::KEY_F7,
        Key::F8 => KeyCode::KEY_F8,
        Key::F9 => KeyCode::KEY_F9,
        Key::F10 => KeyCode::KEY_F10,
        Key::F11 => KeyCode::KEY_F11,
        Key::F12 => KeyCode::KEY_F12,
        Key::F13 => KeyCode::KEY_F13,
        Key::F14 => KeyCode::KEY_F14,
        Key::F15 => KeyCode::KEY_F15,
        Key::F16 => KeyCode::KEY_F16,
        Key::F17 => KeyCode::KEY_F17,
        Key::F18 => KeyCode::KEY_F18,
        Key::F19 => KeyCode::KEY_F19,
        Key::F20 => KeyCode::KEY_F20,
        Key::F21 => KeyCode::KEY_F21,
        Key::F22 => KeyCode::KEY_F22,
        Key::F23 => KeyCode::KEY_F23,
        Key::F24 => KeyCode::KEY_F24,
        Key::Escape => KeyCode::KEY_ESC,
        Key::Tab => KeyCode::KEY_TAB,
        Key::Enter => KeyCode::KEY_ENTER,
        Key::Space => KeyCode::KEY_SPACE,
        Key::Backspace => KeyCode::KEY_BACKSPACE,
        Key::Delete => KeyCode::KEY_DELETE,
        Key::Insert => KeyCode::KEY_INSERT,
        Key::Home => KeyCode::KEY_HOME,
        Key::End => KeyCode::KEY_END,
        Key::PageUp => KeyCode::KEY_PAGEUP,
        Key::PageDown => KeyCode::KEY_PAGEDOWN,
        Key::Up => KeyCode::KEY_UP,
        Key::Down => KeyCode::KEY_DOWN,
        Key::Left => KeyCode::KEY_LEFT,
        Key::Right => KeyCode::KEY_RIGHT,
        Key::Ctrl => KeyCode::KEY_LEFTCTRL,
        Key::Shift => KeyCode::KEY_LEFTSHIFT,
        Key::Alt => KeyCode::KEY_LEFTALT,
        Key::Meta => KeyCode::KEY_LEFTMETA,
        Key::Minus => KeyCode::KEY_MINUS,
        Key::Equals => KeyCode::KEY_EQUAL,
        Key::LeftBracket => KeyCode::KEY_LEFTBRACE,
        Key::RightBracket => KeyCode::KEY_RIGHTBRACE,
        Key::Backslash => KeyCode::KEY_BACKSLASH,
        Key::Semicolon => KeyCode::KEY_SEMICOLON,
        Key::Quote => KeyCode::KEY_APOSTROPHE,
        Key::Backtick => KeyCode::KEY_GRAVE,
        Key::Comma => KeyCode::KEY_COMMA,
        Key::Period => KeyCode::KEY_DOT,
        Key::Slash => KeyCode::KEY_SLASH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_has_its_own_linux_code() {
        for (index, &key) in Key::ALL.iter().enumerate() {
            for &other in Key::ALL.iter().skip(index + 1) {
                assert_ne!(evdev_key(key).0, evdev_key(other).0, "{key:?} / {other:?}");
            }
        }
    }

    #[test]
    fn the_keys_use_the_codes_of_the_linux_input_header() {
        assert_eq!(evdev_key(Key::A).0, 30);
        assert_eq!(evdev_key(Key::Z).0, 44);
        assert_eq!(evdev_key(Key::Escape).0, 1);
        assert_eq!(evdev_key(Key::F13).0, 183);
        assert_eq!(evdev_key(Key::Ctrl).0, 29);
    }

    #[test]
    fn each_mouse_button_has_its_own_linux_code() {
        assert_eq!(evdev_mouse_button(MouseButton::Left).0, 0x110);
        assert_eq!(evdev_mouse_button(MouseButton::Right).0, 0x111);
        assert_eq!(evdev_mouse_button(MouseButton::Middle).0, 0x112);
        assert_eq!(evdev_mouse_button(MouseButton::Back).0, 0x113);
        assert_eq!(evdev_mouse_button(MouseButton::Forward).0, 0x114);
    }
}
