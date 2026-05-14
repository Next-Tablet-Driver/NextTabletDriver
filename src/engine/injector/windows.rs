use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};

pub struct Injector {
    enigo: Enigo,
    /// Tracks the previous state of the primary pen button (tip) to avoid spamming
    /// unnecessary "Button Down" events every frame while dragging.
    last_pressure_down: bool,

    /// Sub-pixel remainder accumulators for relative mode
    remainder_x: f32,
    remainder_y: f32,
}

impl Default for Injector {
    fn default() -> Self {
        Self::new()
    }
}

impl Injector {
    /// Instantiates a new Injector using the default OS settings provided by Enigo.
    #[must_use]
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default())
                .expect("Failed to initialize Enigo mouse injection backend"),
            last_pressure_down: false,
            remainder_x: 0.0,
            remainder_y: 0.0,
        }
    }

    /// Injects an absolute cursor position on the screen.
    /// Used by `Absolute` driver mode.
    ///
    /// On Windows, reads the current cursor position via `GetCursorPos` and
    /// applies a relative delta to reach the target. This avoids the DPI scaling
    /// issues that come with `SendInput` absolute coordinate encoding.
    ///
    /// # Arguments
    /// * `target_x` - Target X coordinate in OS pixels.
    /// * `target_y` - Target Y coordinate in OS pixels.
    /// * `_u` / `_v` - Normalized UV coordinates (unused on Windows).
    pub fn move_absolute(&mut self, target_x: f32, target_y: f32, _u: f32, _v: f32) {
        #[cfg(windows)]
        use windows_sys::Win32::Foundation::POINT;
        #[cfg(windows)]
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        unsafe {
            let mut current_pos = POINT { x: 0, y: 0 };
            if GetCursorPos(&raw mut current_pos) != 0 {
                let target_px = target_x.round() as i32;
                let target_py = target_y.round() as i32;

                let dx = target_px - current_pos.x;
                let dy = target_py - current_pos.y;

                if dx != 0 || dy != 0 {
                    let _ = self.enigo.move_mouse(dx, dy, Coordinate::Rel);

                    // Reset accumulators so relative mode starts clean after a mode switch
                    self.remainder_x = 0.0;
                    self.remainder_y = 0.0;
                }
            }
        }
    }

    pub fn move_relative(&mut self, dx: f32, dy: f32) {
        let total_dx = dx + self.remainder_x;
        let total_dy = dy + self.remainder_y;

        let ix = total_dx.trunc() as i32;
        let iy = total_dy.trunc() as i32;

        self.remainder_x = total_dx.fract();
        self.remainder_y = total_dy.fract();

        if ix != 0 || iy != 0 {
            let _ = self.enigo.move_mouse(ix, iy, Coordinate::Rel);
        }
    }

    /// Synthesizes a Left Mouse Button click or release event.
    ///
    /// The injector maintains internal state and only fires OS events when the
    /// requested `is_down` state differs from the currently held state, preventing
    /// API spam.
    pub fn set_left_button(&mut self, is_down: bool) {
        if is_down && !self.last_pressure_down {
            let _ = self.enigo.button(Button::Left, Direction::Press);
        } else if !is_down && self.last_pressure_down {
            let _ = self.enigo.button(Button::Left, Direction::Release);
        }
        self.last_pressure_down = is_down;
    }
}
