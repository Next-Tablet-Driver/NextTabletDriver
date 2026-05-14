//! # Data Filters
//!
//! This module defines the plugin-like `Filter` architecture. Filters are applied
//! sequentially in a pipeline to manipulate the `(u, v)` normalized coordinate stream
//! before it gets projected onto the screen. This allows for features like smoothing,
//! latency prediction, and telemetry extraction.

pub mod antichatter;
pub mod stats;

use crate::core::config::models::MappingConfig;

/// A trait defining a standardized interface for coordinate mutations.
pub trait Filter: Send + Sync {
    /// Returns the UI-friendly name of the filter.
    fn name(&self) -> &'static str;
    /// Applies the mathematical filter to the current coordinate.
    ///
    /// # Arguments
    /// * `x`, `y` - The incoming coordinate (usually in `[0.0, 1.0]` UV space).
    /// * `config` - The global `MappingConfig` containing user preferences.
    ///
    /// # Returns
    /// The mutated `(x, y)` coordinate.
    fn process(&mut self, x: f32, y: f32, config: &MappingConfig) -> (f32, f32);

    /// Optional hook called whenever the global configuration changes.
    fn update_config(&mut self, _config: &MappingConfig) {}

    /// Clears any internal state bounds (e.g., previous positions, moving averages).
    /// Called when the pen leaves proximity or the tablet disconnects.
    fn reset(&mut self);
}

/// A collection of filters that execute in sequence.
pub struct FilterPipeline {
    /// The ordered list of active filters.
    pub filters: Vec<Box<dyn Filter>>,
}

impl FilterPipeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add(&mut self, filter: Box<dyn Filter>) {
        log::info!(target: "Pipeline", "Adding filter: {}", filter.name());
        self.filters.push(filter);
    }

    pub fn process(&mut self, mut x: f32, mut y: f32, config: &MappingConfig) -> (f32, f32) {
        for filter in &mut self.filters {
            let (nx, ny) = filter.process(x, y, config);
            x = nx;
            y = ny;
        }
        (x, y)
    }

    pub fn update_config(&mut self, config: &MappingConfig) {
        for filter in &mut self.filters {
            filter.update_config(config);
        }
    }

    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
    }
}

impl Default for FilterPipeline {
    fn default() -> Self {
        Self::new()
    }
}
