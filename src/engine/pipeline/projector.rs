//! # Stage 4: Projection
//!
//! Converts UV or MM coordinates into final OS-ready output (pixels or deltas).

use crate::core::config::models::MappingConfig;
use crate::engine::state::SharedState;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Handles the third stage: converting normalized or physical coordinates into
/// final output (screen pixels or relative deltas).
pub struct Projector {
    /// The last known absolute screen position (pixels), used for relative mode fallback.
    pub abs_screen: Option<(f32, f32)>,
    /// The last known physical position (mm), used for calculating relative deltas.
    pub rel_mm: Option<(f32, f32)>,
    /// The timestamp of the previous packet, used to reset relative tracking after inactivity.
    pub packet_time: Instant,
}

impl Default for Projector {
    fn default() -> Self {
        Self {
            abs_screen: None,
            rel_mm: None,
            packet_time: Instant::now(),
        }
    }
}

impl Projector {
    /// Resets all accumulated tracking states for screen and physical coordinates.
    ///
    /// Typically called when the pen leaves tablet proximity or connection is lost.
    pub const fn reset(&mut self) {
        self.abs_screen = None;
        self.rel_mm = None;
    }

    /// Maps normalized UV coordinates `(u, v)` to absolute virtual screen coordinates (in pixels).
    ///
    /// # Arguments
    /// * `u` - Normalized X coordinate in `[0.0, 1.0]`.
    /// * `v` - Normalized Y coordinate in `[0.0, 1.0]`.
    /// * `config` - The current global configuration containing target screen area dimensions.
    /// * `_shared` - A reference to the thread-safe shared application state.
    ///
    /// # Returns
    /// A tuple `(screen_x, screen_y)` representing the absolute target pixel position.
    pub fn project_absolute(
        &mut self,
        u: f32,
        v: f32,
        config: &MappingConfig,
        _shared: &Arc<SharedState>,
    ) -> (f32, f32) {
        let (sx, sy) = crate::core::math::transform::normalized_to_screen(
            u,
            v,
            config.target_area.x,
            config.target_area.y,
            config.target_area.w,
            config.target_area.h,
        );
        self.abs_screen = Some((sx, sy));
        (sx, sy)
    }

    /// Calculates relative cursor movement deltas based on physical coordinate offsets.
    ///
    /// Automatically resets the tracking origin if the time delta between the previous
    /// packet and the current packet exceeds `reset_time_ms`.
    ///
    /// # Arguments
    /// * `x_mm` - The current physical X coordinate (mm).
    /// * `y_mm` - The current physical Y coordinate (mm).
    /// * `config` - The current global configuration containing relative movement limits and sensitivities.
    ///
    /// # Returns
    /// A tuple `(delta_x, delta_y)` representing the relative movement offset.
    pub fn project_relative(&mut self, x_mm: f32, y_mm: f32, config: &MappingConfig) -> (f32, f32) {
        let now = Instant::now();
        if now.duration_since(self.packet_time)
            > Duration::from_millis(u64::from(config.relative_config.reset_time_ms))
        {
            self.reset();
        }
        self.packet_time = now;

        let delta = if let Some((lx, ly)) = self.rel_mm {
            crate::core::math::transform::apply_relative_delta(
                x_mm,
                y_mm,
                lx,
                ly,
                config.relative_config.rotation,
                config.relative_config.x_sensitivity,
                config.relative_config.y_sensitivity,
            )
        } else {
            (0.0, 0.0)
        };

        self.rel_mm = Some((x_mm, y_mm));
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::models::MappingConfig;
    use crate::engine::state::SharedState;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[test]
    fn test_project_absolute_center() {
        let mut p = Projector::default();
        let cfg = MappingConfig::default();
        let shared = Arc::new(SharedState::new());

        let (sx, sy) = p.project_absolute(0.5, 0.5, &cfg, &shared);
        assert!((sx - (cfg.target_area.w * 0.5)).abs() < 1e-6);
        assert!((sy - (cfg.target_area.h * 0.5)).abs() < 1e-6);
        assert_eq!(p.abs_screen, Some((sx, sy)));
    }

    #[test]
    fn test_project_relative_basic_and_rotation() {
        let mut p = Projector::default();
        let mut cfg = MappingConfig::default();
        cfg.relative_config.x_sensitivity = 2.0;
        cfg.relative_config.y_sensitivity = 3.0;
        cfg.relative_config.rotation = 0.0;

        // First report: no previous position -> zero delta
        let d1 = p.project_relative(10.0, 5.0, &cfg);
        assert_eq!(d1, (0.0, 0.0));

        // Second report: delta should be (2mm * 2, 3mm * 3)
        let d2 = p.project_relative(12.0, 8.0, &cfg);
        assert!((d2.0 - 4.0).abs() < 1e-6);
        assert!((d2.1 - 9.0).abs() < 1e-6);
    }

    #[test]
    fn test_project_relative_resets_after_inactivity() {
        let mut p = Projector::default();
        let mut cfg = MappingConfig::default();
        cfg.relative_config.reset_time_ms = 1; // very small

        // Prime the previous position
        let _ = p.project_relative(0.0, 0.0, &cfg);
        // Simulate inactivity by setting packet_time far in the past
        p.packet_time = Instant::now() - Duration::from_millis(10);
        p.rel_mm = Some((1.0, 1.0));

        // Now project_relative should detect the timeout and reset to None -> produce zero delta
        let delta = p.project_relative(2.0, 2.0, &cfg);
        assert_eq!(delta, (0.0, 0.0));
        // rel_mm should have been updated to the new coords
        assert_eq!(p.rel_mm, Some((2.0, 2.0)));
    }
}
