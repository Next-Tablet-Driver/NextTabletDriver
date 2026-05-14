//! # Input Processing Pipeline
//!
//! This module defines the `Pipeline` struct, which is responsible for taking raw
//! decoded hardware packets (`TabletData`) from a specific vendor driver and
//! pushing them through the mathematical and filtering transformations required
//! to produce OS-ready cursor coordinates.

use crate::core::config::models::{DriverMode, MappingConfig};
use crate::drivers::TabletData;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

/// A structure to hold the intermediate results of the pipeline processing.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessedFrame {
    pub u: f32,
    pub v: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub is_down: bool,
}

/// The core processing pipeline for tablet input events.
///
/// It maintains internal state across frames (such as previous coordinates for
/// relative mode or filter history) and orchestrates the flow from raw data ->
/// filters -> transformation -> OS injection.
pub struct Pipeline {
    /// The last known absolute screen position (pixels), used for relative mode fallback.
    abs_screen: Option<(f32, f32)>,
    /// The last known physical position (mm), used for calculating relative deltas.
    rel_mm: Option<(f32, f32)>,
    /// The timestamp of the previous packet, used to reset relative tracking after inactivity.
    packet_time: Instant,

    // Pre-calculated coefficients
    x_multiplier: f32,
    y_multiplier: f32,
    tip_threshold_raw: f32,

    // Cached specs to detect changes
    last_max_w: f32,
    last_max_h: f32,
    last_phys_w: f32,
    last_phys_h: f32,
    last_max_p: f32,
    last_threshold: u16,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            abs_screen: None,
            rel_mm: None,
            packet_time: Instant::now(),
            x_multiplier: 1.0,
            y_multiplier: 1.0,
            tip_threshold_raw: 0.0,
            last_max_w: 0.0,
            last_max_h: 0.0,
            last_phys_w: 0.0,
            last_phys_h: 0.0,
            last_max_p: 0.0,
            last_threshold: 0,
        }
    }

    /// Resets the internal tracking for relative mode.
    /// This prevents massive cursor jumps when the pen is lifted and placed back
    /// down on a different part of the tablet.
    pub const fn reset_relative(&mut self) {
        self.abs_screen = None;
        self.rel_mm = None;
    }

    /// Processes a single hardware packet through the entire stack.
    pub fn process(
        &mut self,
        data: &TabletData,
        driver: &dyn crate::drivers::NextTabletDriver,
        config: &MappingConfig,
        injector: &mut crate::engine::injector::Injector,
        filters: &mut crate::filters::FilterPipeline,
        #[allow(unused_variables)] shared: &Arc<crate::engine::state::SharedState>,
    ) {
        if !data.is_connected {
            injector.set_left_button(false);
            // self.reset_relative();
            filters.reset();
            return;
        }

        // Skip non-positional reports (aux, tool ID, out-of-range)
        if !matches!(
            data.status,
            crate::drivers::TabletStatus::Contact
                | crate::drivers::TabletStatus::Hover
                | crate::drivers::TabletStatus::Active
        ) {
            return;
        }

        let (max_w, max_h, max_p) = driver.get_specs();
        let (phys_w, phys_h) = driver.get_physical_specs();

        // Update coefficients if specs changed (e.g. tablet reconnected)
        if max_w != self.last_max_w
            || max_h != self.last_max_h
            || phys_w != self.last_phys_w
            || phys_h != self.last_phys_h
        {
            self.x_multiplier = if max_w > 0.0 { phys_w / max_w } else { 0.0 };
            self.y_multiplier = if max_h > 0.0 { phys_h / max_h } else { 0.0 };

            self.last_max_w = max_w;
            self.last_max_h = max_h;
            self.last_phys_w = phys_w;
            self.last_phys_h = phys_h;
        }

        // Update pressure threshold if config or driver specs changed
        if config.tip_threshold != self.last_threshold || max_p != self.last_max_p {
            self.tip_threshold_raw = (f32::from(config.tip_threshold) * 0.01) * max_p;
            self.last_threshold = config.tip_threshold;
            self.last_max_p = max_p;
        }

        let x_mm = f32::from(data.x) * self.x_multiplier;
        let y_mm = f32::from(data.y) * self.y_multiplier;

        // Normalize
        let (u, v) = Self::normalize(x_mm, y_mm, config, shared);

        // Filter
        let (u, v) = Self::filter(u, v, config, filters, shared);

        // Project
        let mut frame = ProcessedFrame {
            u,
            v,
            ..Default::default()
        };

        match config.mode {
            DriverMode::Absolute => {
                let (sx, sy) = Self::project_absolute(u, v, config, shared);
                frame.screen_x = sx;
                frame.screen_y = sy;
                injector.move_absolute(sx, sy, u, v);

                self.abs_screen = Some((sx, sy));
            }
            DriverMode::Relative => {
                let (dx, dy) = self.project_relative(x_mm, y_mm, config);
                injector.move_relative(dx, dy);
            }
        }

        // Pressure & Injection
        frame.is_down = self.evaluate_pressure(data.pressure, max_p, config);
        injector.set_left_button(frame.is_down);
    }

    fn normalize(
        x_mm: f32,
        y_mm: f32,
        config: &MappingConfig,
        #[allow(unused_variables)] shared: &Arc<crate::engine::state::SharedState>,
    ) -> (f32, f32) {
        let (u, v) = crate::core::math::transform::physical_to_normalized(
            x_mm,
            y_mm,
            config.active_area.x,
            config.active_area.y,
            config.active_area.w,
            config.active_area.h,
            config.active_area.rotation,
        );

        (u, v)
    }

    fn filter(
        u: f32,
        v: f32,
        config: &MappingConfig,
        filters: &mut crate::filters::FilterPipeline,
        #[allow(unused_variables)] shared: &Arc<crate::engine::state::SharedState>,
    ) -> (f32, f32) {
        let (nu, nv) = filters.process(u, v, config);

        (nu, nv)
    }

    fn project_absolute(
        u: f32,
        v: f32,
        config: &MappingConfig,
        #[allow(unused_variables)] shared: &Arc<crate::engine::state::SharedState>,
    ) -> (f32, f32) {
        let (sx, sy) = crate::core::math::transform::normalized_to_screen(
            u,
            v,
            config.target_area.x,
            config.target_area.y,
            config.target_area.w,
            config.target_area.h,
        );

        (sx, sy)
    }

    fn project_relative(&mut self, x_mm: f32, y_mm: f32, config: &MappingConfig) -> (f32, f32) {
        let now = Instant::now();
        if now.duration_since(self.packet_time)
            > Duration::from_millis(u64::from(config.relative_config.reset_time_ms))
        {
            self.reset_relative();
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

    pub fn evaluate_pressure(
        &mut self,
        pressure_raw: u16,
        max_p: f32,
        config: &MappingConfig,
    ) -> bool {
        // Update pressure threshold if config or driver specs changed
        if config.tip_threshold != self.last_threshold || max_p != self.last_max_p {
            self.tip_threshold_raw = (f32::from(config.tip_threshold) * 0.01) * max_p;
            self.last_threshold = config.tip_threshold;
            self.last_max_p = max_p;
        }

        let pressure = if config.disable_pressure {
            max_p as u16
        } else {
            pressure_raw
        };
        f32::from(pressure) > self.tip_threshold_raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::models::MappingConfig;
    use crate::drivers::TabletData;
    use crate::engine::injector::Injector;
    use crate::engine::state::SharedState;
    use crate::filters::FilterPipeline;

    struct MockDriver;
    impl crate::drivers::NextTabletDriver for MockDriver {
        fn get_name(&self) -> &str {
            "Mock Driver"
        }
        fn get_specs(&self) -> (f32, f32, f32) {
            (1000.0, 1000.0, 1000.0)
        }
        fn get_physical_specs(&self) -> (f32, f32) {
            (100.0, 100.0)
        }
        fn get_vid_pid(&self) -> (u16, u16) {
            (0x0000, 0x0000)
        }
        fn parse(&self, _buf: &[u8]) -> Option<TabletData> {
            None
        }
    }

    #[test]
    fn test_pipeline_absolute_normalization() {
        let mut pipeline = Pipeline::new();
        let mut config = MappingConfig::default();
        config.active_area.x = 50.0;
        config.active_area.y = 50.0;
        config.active_area.w = 100.0;
        config.active_area.h = 100.0;

        let shared = Arc::new(SharedState::test_default());
        let mut injector = Injector::new();
        let mut filters = FilterPipeline::new();
        let driver = MockDriver;

        let data = TabletData {
            is_connected: true,
            status: crate::drivers::TabletStatus::Contact,
            x: 500, // Center (50mm)
            y: 500, // Center (50mm)
            ..Default::default()
        };

        pipeline.process(
            &data,
            &driver,
            &config,
            &mut injector,
            &mut filters,
            &shared,
        );
    }

    #[test]
    fn test_pipeline_pressure_threshold() {
        let mut pipeline = Pipeline::new();
        let mut config = MappingConfig::default();
        config.tip_threshold = 50; // 50%

        // max_p = 1000.0, so threshold = 500.0
        assert!(pipeline.evaluate_pressure(501, 1000.0, &config));
        assert!(!pipeline.evaluate_pressure(499, 1000.0, &config));
    }

    #[test]
    fn test_pipeline_disable_pressure() {
        let mut pipeline = Pipeline::new();
        let mut config = MappingConfig::default();
        config.disable_pressure = true;
        config.tip_threshold = 50;

        // Should always be true (down) regardless of raw pressure
        assert!(pipeline.evaluate_pressure(0, 1000.0, &config));
    }
}
