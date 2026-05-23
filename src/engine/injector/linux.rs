use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, InputEvent, InputId, KeyCode,
    RelativeAxisCode, UinputAbsSetup, uinput::VirtualDevice,
};

/// Maximum value for absolute axes (standard high-resolution range).
/// The compositor/X server will map this to the actual screen dimensions.
const ABS_MAX: i32 = 32767;

/// Maximum pressure value exposed by the virtual tablet.
/// Individual tablets may have different maximums, but we normalize to this range.
const PRESSURE_MAX: i32 = 8191;

pub struct Injector {
    /// The virtual tablet device for absolute coordinate injection.
    /// Registered as a pen digitizer in the kernel via `/dev/uinput`.
    virtual_tablet: evdev::uinput::VirtualDevice,

    /// A separate virtual mouse device for relative movement injection.
    /// Some compositors handle relative events differently from tablet events,
    /// so we keep them on separate virtual devices.
    virtual_mouse: evdev::uinput::VirtualDevice,

    /// Tracks the previous state of the primary pen button (tip).
    last_pressure_down: bool,

    /// Tracks the previous proximity state of the stylus (in range).
    last_proximity: Option<bool>,

    /// Sub-pixel remainder accumulators for relative mode
    remainder_x: f32,
    remainder_y: f32,
}

impl Default for Injector {
    fn default() -> Self {
        Self::new()
    }
}

fn create_uinput_device_builder() -> evdev::uinput::VirtualDeviceBuilder<'static> {
    match VirtualDevice::builder() {
        Ok(builder) => builder,
        Err(e) => {
            log::error!(
                target: "Injector",
                "CRITICAL ERROR: Failed to open /dev/uinput: {}.\n\
                This typically means the uinput kernel module is not loaded or you don't have permission to write to '/dev/uinput'.\n\
                \n\
                TO RESOLVE THIS (Linux):\n\
                1. Make sure the 'uinput' kernel module is loaded:\n\
                   sudo modprobe uinput\n\
                2. Configure udev rules to allow non-root users write access to uinput:\n\
                   echo 'KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\", OPTIONS+=\"static_node=uinput\"' | sudo tee /etc/udev/rules.d/99-uinput.rules\n\
                3. Add your user to the 'input' group:\n\
                   sudo usermod -aG input $USER\n\
                4. Log out and log back in, or run:\n\
                   sudo udevadm control --reload-rules && sudo udevadm trigger --sysname-match=uinput\n",
                e
            );
            panic!(
                "Failed to open /dev/uinput: {e}. Please configure uinput udev rules as printed above."
            );
        }
    }
}

impl Injector {
    /// Creates both virtual devices via `/dev/uinput`:
    ///
    /// 1. **Virtual Tablet** — Reports `EV_ABS` with `ABS_X`, `ABS_Y`, `ABS_PRESSURE`,
    ///    `ABS_TILT_X`, `ABS_TILT_Y`, and key events for `BTN_TOUCH`, `BTN_TOOL_PEN`,
    ///    `BTN_STYLUS`, `BTN_STYLUS2`.
    ///
    /// 2. **Virtual Mouse** — Reports `EV_REL` with `REL_X`, `REL_Y`, and
    ///    `BTN_LEFT` for relative mode operation.
    ///
    /// # Panics
    /// Panics if `/dev/uinput` cannot be opened (missing permissions or kernel module).
    pub fn new() -> Self {
        let mut tablet_keys = AttributeSet::<KeyCode>::new();
        tablet_keys.insert(KeyCode::BTN_TOUCH);
        tablet_keys.insert(KeyCode::BTN_TOOL_PEN);
        tablet_keys.insert(KeyCode::BTN_STYLUS);
        tablet_keys.insert(KeyCode::BTN_STYLUS2);

        let virtual_tablet = create_uinput_device_builder()
            .name("NextTabletDriver Virtual Pen")
            .input_id(InputId::new(BusType::BUS_USB, 0x0001, 0x0001, 1))
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_X,
                AbsInfo::new(0, 0, ABS_MAX, 0, 0, 100),
            ))
            .expect("Failed to set ABS_X")
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_Y,
                AbsInfo::new(0, 0, ABS_MAX, 0, 0, 100),
            ))
            .expect("Failed to set ABS_Y")
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_PRESSURE,
                AbsInfo::new(0, 0, PRESSURE_MAX, 0, 0, 0),
            ))
            .expect("Failed to set ABS_PRESSURE")
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_TILT_X,
                AbsInfo::new(0, -127, 127, 0, 0, 0),
            ))
            .expect("Failed to set ABS_TILT_X")
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_TILT_Y,
                AbsInfo::new(0, -127, 127, 0, 0, 0),
            ))
            .expect("Failed to set ABS_TILT_Y")
            .with_keys(&tablet_keys)
            .expect("Failed to set tablet keys")
            .build()
            .expect("Failed to create virtual tablet device");

        log::info!(target: "Injector", "Virtual tablet device created: NextTabletDriver Virtual Pen");

        let mut mouse_keys = AttributeSet::<KeyCode>::new();
        mouse_keys.insert(KeyCode::BTN_LEFT);
        mouse_keys.insert(KeyCode::BTN_RIGHT);
        mouse_keys.insert(KeyCode::BTN_MIDDLE);

        let mut rel_axes = AttributeSet::<RelativeAxisCode>::new();
        rel_axes.insert(RelativeAxisCode::REL_X);
        rel_axes.insert(RelativeAxisCode::REL_Y);

        let virtual_mouse = create_uinput_device_builder()
            .name("NextTabletDriver Virtual Mouse")
            .input_id(InputId::new(BusType::BUS_USB, 0x0001, 0x0002, 1))
            .with_relative_axes(&rel_axes)
            .expect("Failed to set REL axes")
            .with_keys(&mouse_keys)
            .expect("Failed to set mouse keys")
            .build()
            .expect("Failed to create virtual mouse device");

        log::info!(target: "Injector", "Virtual mouse device created: NextTabletDriver Virtual Mouse");

        Self {
            virtual_tablet,
            virtual_mouse,
            last_pressure_down: false,
            last_proximity: None,
            remainder_x: 0.0,
            remainder_y: 0.0,
        }
    }

    pub fn set_proximity(&mut self, in_proximity: bool) {
        if Some(in_proximity) == self.last_proximity {
            return;
        }

        let value = if in_proximity { 1 } else { 0 };
        let events = [
            InputEvent::new(evdev::EventType::KEY.0, KeyCode::BTN_TOOL_PEN.0, value),
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_tablet.emit(&events) {
            log::error!(target: "Injector", "Failed to emit proximity event: {}", e);
        }

        self.last_proximity = Some(in_proximity);
    }

    /// Injects an absolute pen position on the screen.
    /// Used by `Absolute` driver mode.
    ///
    /// On Linux, we write `ABS_X` and `ABS_Y` events to the uinput virtual tablet.
    /// The values are normalized to the [0..32767] range. The compositor or X server
    /// maps this to actual screen coordinates automatically.
    ///
    /// # Arguments
    /// * `_target_x` / `_target_y` - Screen pixel coordinates (unused on Linux).
    /// * `u` / `v` - Normalized UV coordinates in [0.0, 1.0] from the pipeline.
    /// * `pressure` - Normalized pressure.
    /// * `tilt_x` / `tilt_y` - Absolute tilt values.
    pub fn move_absolute(
        &mut self,
        _target_x: f32,
        _target_y: f32,
        u: f32,
        v: f32,
        pressure: i32,
        tilt_x: i32,
        tilt_y: i32,
    ) {
        self.set_proximity(true);

        let abs_x = (u.clamp(0.0, 1.0) * ABS_MAX as f32) as i32;
        let abs_y = (v.clamp(0.0, 1.0) * ABS_MAX as f32) as i32;
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
            log::error!(target: "Injector", "Failed to emit absolute events: {}", e);
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
                log::error!(target: "Injector", "Failed to emit relative events: {}", e);
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

        let value = if is_down { 1 } else { 0 };

        let events = [
            InputEvent::new(evdev::EventType::KEY.0, KeyCode::BTN_TOUCH.0, value),
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_tablet.emit(&events) {
            log::error!(target: "Injector", "Failed to emit button event: {}", e);
        }

        // BTN_LEFT on mouse device for apps that don't honor BTN_TOUCH
        let mouse_events = [
            InputEvent::new(evdev::EventType::KEY.0, KeyCode::BTN_LEFT.0, value),
            InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0),
        ];

        if let Err(e) = self.virtual_mouse.emit(&mouse_events) {
            log::error!(target: "Injector", "Failed to emit mouse button event: {}", e);
        }

        self.last_pressure_down = is_down;
    }
}
