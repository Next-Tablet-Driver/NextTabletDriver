//! # Stage 1: Transformation
//!
//! Converts raw hardware coordinates (0..max) into physical millimeters (0..phys).

/// Handles the first stage of the pipeline: converting raw hardware coordinates
/// into physical millimeters using pre-calculated multipliers.
#[derive(Default)]
pub struct Transformer {
    x_multiplier: f32,
    y_multiplier: f32,
    last_max_w: f32,
    last_max_h: f32,
    last_phys_w: f32,
    last_phys_h: f32,
}

impl Transformer {
    /// Converts raw hardware coordinates `(x, y)` to physical dimensions in millimeters.
    ///
    /// The multipliers are automatically recomputed if the hardware resolution (`max_w`/`max_h`)
    /// or physical area specs (`phys_w`/`phys_h`) change by more than a tiny epsilon.
    ///
    /// # Arguments
    /// * `x` - Raw tablet X coordinate.
    /// * `y` - Raw tablet Y coordinate.
    /// * `max_w` - Maximum hardware X limit.
    /// * `max_h` - Maximum hardware Y limit.
    /// * `phys_w` - Physical width of the tablet active surface (mm).
    /// * `phys_h` - Physical height of the tablet active surface (mm).
    ///
    /// # Returns
    /// A tuple `(x_mm, y_mm)` representing coordinates in millimeters.
    pub fn execute(
        &mut self,
        x: u16,
        y: u16,
        max_w: f32,
        max_h: f32,
        phys_w: f32,
        phys_h: f32,
    ) -> (f32, f32) {
        {
            const EPS: f32 = 1e-6;
            if (max_w - self.last_max_w).abs() > EPS
                || (max_h - self.last_max_h).abs() > EPS
                || (phys_w - self.last_phys_w).abs() > EPS
                || (phys_h - self.last_phys_h).abs() > EPS
            {
                self.x_multiplier = if max_w > 0.0 { phys_w / max_w } else { 0.0 };
                self.y_multiplier = if max_h > 0.0 { phys_h / max_h } else { 0.0 };

                self.last_max_w = max_w;
                self.last_max_h = max_h;
                self.last_phys_w = phys_w;
                self.last_phys_h = phys_h;
            }
        }

        (
            f32::from(x) * self.x_multiplier,
            f32::from(y) * self.y_multiplier,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformer_basic_conversion() {
        let mut t = Transformer::default();

        // Simple conversion where multipliers are computed from specs
        let (x_mm, y_mm) = t.execute(100, 200, 1000.0, 2000.0, 100.0, 200.0);

        let expected_x = 100.0_f32 * (100.0 / 1000.0);
        let expected_y = 200.0_f32 * (200.0 / 2000.0);

        assert!((x_mm - expected_x).abs() < 1e-6);
        assert!((y_mm - expected_y).abs() < 1e-6);
    }

    #[test]
    fn test_transformer_handles_zero_dimensions() {
        let mut t = Transformer::default();

        // If max width is zero, x multiplier should become 0 and produce 0 coordinates
        let (x_mm, y_mm) = t.execute(50, 75, 0.0, 1000.0, 100.0, 200.0);
        assert_eq!(x_mm, 0.0);
        // y dimension is still valid
        assert!((y_mm - (75.0_f32 * (200.0 / 1000.0))).abs() < 1e-6);
    }

    #[test]
    fn test_transformer_small_changes_do_not_recompute() {
        let mut t = Transformer::default();

        // First compute baseline multipliers
        let baseline = t.execute(10, 20, 1000.0, 1000.0, 100.0, 100.0);

        // Values that differ by less than EPS should not trigger recompute
        let a = t.execute(
            10,
            20,
            1000.0 + 1e-7,
            1000.0 + 1e-7,
            100.0 + 1e-7,
            100.0 + 1e-7,
        );
        let b = t.execute(10, 20, 1000.0, 1000.0, 100.0, 100.0);

        assert!((a.0 - b.0).abs() < 1e-6);
        assert!((a.1 - b.1).abs() < 1e-6);
        // baseline should be close to both
        assert!((baseline.0 - a.0).abs() < 1e-3);
        assert!((baseline.1 - a.1).abs() < 1e-3);
    }
}
