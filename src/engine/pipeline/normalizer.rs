//! # Stage 2: Normalization
//!
//! Converts physical millimeters into normalized UV coordinates (0..1).

use crate::core::config::models::MappingConfig;
use crate::engine::state::SharedState;
use std::sync::Arc;

/// Handles the second stage: converting physical millimeters into normalized
/// UV coordinates (0..1) based on the active area configuration.
#[derive(Default)]
pub struct Normalizer;

impl Normalizer {
    /// Converts physical coordinates `(x_mm, y_mm)` to normalized UV coordinates `[0.0, 1.0]`
    /// relative to the configured active area offsets, rotation, and dimensions.
    ///
    /// # Arguments
    /// * `x_mm` - Physical X coordinate in millimeters.
    /// * `y_mm` - Physical Y coordinate in millimeters.
    /// * `config` - The current global application configuration defining the active area.
    /// * `_shared` - A reference to the thread-safe shared application state.
    ///
    /// # Returns
    /// A tuple `(u, v)` representing normalized UV coordinates.
    #[allow(clippy::unused_self)]
    pub fn execute(
        &self,
        x_mm: f32,
        y_mm: f32,
        config: &MappingConfig,
        _shared: &Arc<SharedState>,
    ) -> (f32, f32) {
        crate::core::math::transform::physical_to_normalized(
            x_mm,
            y_mm,
            config.active_area.x,
            config.active_area.y,
            config.active_area.w,
            config.active_area.h,
            config.active_area.rotation,
        )
    }
}
