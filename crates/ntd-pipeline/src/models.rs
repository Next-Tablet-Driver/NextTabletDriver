//! # Pipeline Data Models
//!
//! Shared structures used across different stages of the processing pipeline.

/// A structure to hold the intermediate results of the pipeline processing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ProcessedFrame {
    /// Horizontal position inside the active area, from 0 to 1.
    pub u: f32,
    /// Vertical position inside the active area, from 0 to 1.
    pub v: f32,
    /// Horizontal position on screen in pixels, or movement since the last packet in relative mode.
    pub screen_x: f32,
    /// Vertical position on screen in pixels, or movement since the last packet in relative mode.
    pub screen_y: f32,
    /// The tip counts as touching.
    pub is_down: bool,
    /// Pressure scaled to the injector's 0-8191 range, independent of driver mode.
    pub pressure: i32,
    /// Horizontal tilt, in degrees.
    pub tilt_x: i32,
    /// Vertical tilt, in degrees.
    pub tilt_y: i32,
}
