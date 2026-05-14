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
            normalizer: Normalizer::default(),
            projector: Projector::default(),
            tip_threshold_raw: 0.0,
            last_max_p: 0.0,
            last_threshold: 0,
        }
    }

    /// Resets the internal tracking for relative mode.
    pub fn reset_relative(&mut self) {
        self.projector.reset();
    }

    /// Processes a single hardware packet through the entire stack.
    pub fn process(
        &mut self,
        data: &TabletData,
        driver: &dyn crate::drivers::NextTabletDriver,
        config: &MappingConfig,
        injector: &mut crate::engine::injector::Injector,
        filters: &mut crate::filters::FilterPipeline,
        shared: &Arc<crate::engine::state::SharedState>,
    ) {
        if !data.is_connected {
            injector.set_left_button(false);
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

        // Stage 1: Transformation (Raw -> MM)
        let (x_mm, y_mm) = self
            .transformer
            .execute(data.x, data.y, max_w, max_h, phys_w, phys_h);

        // Stage 2: Normalization (MM -> UV)
        let (u, v) = self.normalizer.execute(x_mm, y_mm, config, shared);

        // Stage 3: Filtering (UV -> UV)
        let (u, v) = filters.process(u, v, config);

        // Stage 4: Projection & Injection (UV/MM -> Screen/Events)
        let mut frame = ProcessedFrame {
            u,
            v,
            ..Default::default()
        };

        match config.mode {
            DriverMode::Absolute => {
                let (sx, sy) = self.projector.project_absolute(u, v, config, shared);
                frame.screen_x = sx;
                frame.screen_y = sy;
                injector.move_absolute(sx, sy, u, v);
            }
            DriverMode::Relative => {
                let (dx, dy) = self.projector.project_relative(x_mm, y_mm, config);
                injector.move_relative(dx, dy);
            }
        }

        // Pressure & Injection
        frame.is_down = self.evaluate_pressure(data.pressure, max_p, config);
        injector.set_left_button(frame.is_down);
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
