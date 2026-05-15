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
    pub const fn reset(&mut self) {
        self.abs_screen = None;
        self.rel_mm = None;
    }

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
