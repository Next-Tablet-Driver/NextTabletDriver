//! # Pipeline Data Models
//!
//! Shared structures used across different stages of the processing pipeline.

/// A structure to hold the intermediate results of the pipeline processing.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessedFrame {
    pub u: f32,
    pub v: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub is_down: bool,
    /// Pressure scaled to the injector's 0-8191 range, independent of driver mode.
    pub pressure: i32,
    pub tilt_x: i32,
    pub tilt_y: i32,
}
