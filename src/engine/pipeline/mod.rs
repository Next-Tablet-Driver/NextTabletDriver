//! # Input Processing Pipeline
//!
//! Orchestrates the flow of raw data through various stages to produce OS-ready input.

pub mod models;
pub mod normalizer;
pub mod projector;
pub mod transformer;

pub use models::ProcessedFrame;
pub use normalizer::Normalizer;
pub use projector::Projector;
pub use transformer::Transformer;

use crate::core::config::models::{DriverMode, MappingConfig};
use crate::drivers::TabletData;
use std::sync::Arc;

/// The core processing pipeline for tablet input events.
///
/// Refactored into a component-based architecture for better modularity.
pub struct Pipeline {
    pub transformer: Transformer,
    pub normalizer: Normalizer,
    pub projector: Projector,

    // Pressure evaluation state
    pub tip_threshold_raw: f32,
    pub last_max_p: f32,
    pub last_threshold: u16,
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
            transformer: Transformer::default(),
            normalizer: Normalizer,
            projector: Projector::default(),
            tip_threshold_raw: 0.0,
            last_max_p: 0.0,
            last_threshold: 0,
        }
    }

    /// Resets the internal tracking for relative mode.
    pub const fn reset_relative(&mut self) {
        self.projector.reset();
    }

    /// Processes a single hardware packet through the entire stack, returning the
    /// resulting frame. Callers that need to drive OS input injection (the desktop
    /// app) are responsible for feeding the returned frame into `engine::injector`
    /// themselves. This method never touches the OS, so it's safe to call from
    /// contexts (like the SDK's embedded engine) that must never inject input.
    pub fn process(
        &mut self,
        data: &TabletData,
        driver: &dyn crate::drivers::NextTabletDriver,
        config: &MappingConfig,
        filters: &mut crate::filters::FilterPipeline,
        shared: &Arc<crate::engine::state::SharedState>,
    ) -> ProcessedFrame {
        if !data.is_connected {
            filters.reset();
            return ProcessedFrame::default();
        }

        // Skip non-positional reports (aux, tool ID, out-of-range)
        if !matches!(
            data.status,
            crate::drivers::TabletStatus::Contact
                | crate::drivers::TabletStatus::Hover
                | crate::drivers::TabletStatus::Active
        ) {
            return ProcessedFrame::default();
        }

        let (max_w, max_h, max_p) = driver.get_specs();
        let (phys_w, phys_h) = driver.get_physical_specs();

        // Stage 1: Transformation (Raw -> MM)
        let (x_mm, y_mm) = self
            .transformer
            .execute(data.x, data.y, max_w, max_h, phys_w, phys_h);

        // Stage 2: Normalization (MM -> UV)
        let (u, v) = self.normalizer.execute(x_mm, y_mm, config, shared);

        // Stage 3: Filtering (UV -> UV)
        let (u, v) = filters.process(u, v, config);

        let pressure_ratio = if max_p > 0.0 {
            f32::from(data.pressure) / max_p
        } else {
            0.0
        };

        // Stage 4: Projection (UV/MM -> Screen)
        let mut frame = ProcessedFrame {
            u,
            v,
            pressure: (pressure_ratio.clamp(0.0, 1.0) * 8191.0) as i32,
            tilt_x: i32::from(data.tilt_x),
            tilt_y: i32::from(data.tilt_y),
            ..Default::default()
        };

        match config.mode {
            DriverMode::Absolute => {
                let (sx, sy) = self.projector.project_absolute(u, v, config, shared);
                frame.screen_x = sx;
                frame.screen_y = sy;
            }
            DriverMode::Relative => {
                let (dx, dy) = self.projector.project_relative(x_mm, y_mm, config);
                frame.screen_x = dx;
                frame.screen_y = dy;
            }
        }

        frame.is_down = self.evaluate_pressure(data.pressure, max_p, config);
        frame
    }

    pub fn evaluate_pressure(
        &mut self,
        pressure_raw: u16,
        max_p: f32,
        config: &MappingConfig,
    ) -> bool {
        // Update pressure threshold if config or driver specs changed
        const EPS: f32 = 1e-6;
        if config.tip_threshold != self.last_threshold || (max_p - self.last_max_p).abs() > EPS {
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;
    use crate::core::config::models::MappingConfig;
    use crate::drivers::TabletData;
    use crate::engine::state::SharedState;
    use crate::filters::FilterPipeline;

    struct MockDriver;
    impl crate::drivers::NextTabletDriver for MockDriver {
        fn get_name(&self) -> &'static str {
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
        let mut filters = FilterPipeline::new();
        let driver = MockDriver;

        let data = TabletData {
            is_connected: true,
            status: crate::drivers::TabletStatus::Contact,
            x: 500, // Center (50mm)
            y: 500, // Center (50mm)
            ..Default::default()
        };

        let frame = pipeline.process(&data, &driver, &config, &mut filters, &shared);
        assert!((frame.u - 0.5).abs() < f32::EPSILON);
        assert!((frame.v - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pipeline_pressure_threshold() {
        let mut pipeline = Pipeline::new();
        let config = MappingConfig {
            tip_threshold: 50,
            ..Default::default()
        };

        // max_p = 1000.0, so threshold = 500.0
        assert!(pipeline.evaluate_pressure(501, 1000.0, &config));
        assert!(!pipeline.evaluate_pressure(499, 1000.0, &config));
    }

    #[test]
    fn test_pipeline_disable_pressure() {
        let mut pipeline = Pipeline::new();
        let config = MappingConfig {
            disable_pressure: true,
            tip_threshold: 50,
            ..Default::default()
        };

        // Should always be true (down) regardless of raw pressure
        assert!(pipeline.evaluate_pressure(0, 1000.0, &config));
    }
}
