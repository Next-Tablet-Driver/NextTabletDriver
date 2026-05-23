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
}
