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
    ///
    /// # Panics
    /// Panics if the Enigo mouse injection backend fails to initialize. This is usually
    /// due to missing OS permissions or an unsupported display environment.
    #[must_use]
    pub fn new() -> Self {
        let settings = Settings::default();
        let enigo = Enigo::new(&settings).unwrap_or_else(|e| {
            log::error!(target: "Injector", "CRITICAL: Failed to initialize Enigo: {e}");
            #[allow(clippy::panic)]
            {
                panic!("Failed to initialize Enigo mouse injection backend: {e}");
            }
        });

        log::info!(target: "Injector", "Windows Injector initialized successfully.");

        Self {
            enigo,
            last_pressure_down: false,
            remainder_x: 0.0,
            remainder_y: 0.0,
        }
    }

    /// Updates stylus proximity state. Unused on Windows since proximity
    /// tracking is handled at the system level.
    pub const fn set_proximity(&mut self, _in_proximity: bool) {}

    /// Injects an absolute cursor position on the screen.
    /// Used by `Absolute` driver mode.
    ///
    /// Directs mouse coordinate injection via Enigo using absolute pixel
    /// coordinates (`Coordinate::Abs`).
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
        let target_px = target_x.round() as i32;
        let target_py = target_y.round() as i32;

        let _ = self.enigo.move_mouse(target_px, target_py, Coordinate::Abs);

        // Reset accumulators so relative mode starts clean after a mode switch
        self.remainder_x = 0.0;
        self.remainder_y = 0.0;
    }

    /// Injects relative mouse movement on the screen.
    /// Used by `Relative` driver mode.
    ///
    /// Accumulates sub-pixel movement remainders and emits relative move
    /// events once they accumulate to at least a full integer pixel.
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
            log::debug!(target: "Injector", "Pen tip DOWN (Left Click Pressed)");
            let _ = self.enigo.button(Button::Left, Direction::Press);
        } else if !is_down && self.last_pressure_down {
            log::debug!(target: "Injector", "Pen tip UP (Left Click Released)");
            let _ = self.enigo.button(Button::Left, Direction::Release);
        }
        self.last_pressure_down = is_down;
    }
}
