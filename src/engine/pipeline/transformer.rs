//! # Stage 1: Transformation
//!
//! Converts raw hardware coordinates (0..max) into physical millimeters (0..phys).

/// Handles the first stage of the pipeline: converting raw hardware coordinates
/// into physical millimeters using pre-calculated multipliers.
#[derive(Default)]
pub struct Transformer {
    x_multiplier: f32,
    y_multiplier: f32,
    last_max_w: f32,
    last_max_h: f32,
    last_phys_w: f32,
    last_phys_h: f32,
}

impl Transformer {
    pub fn execute(
        &mut self,
        x: u16,
        y: u16,
        max_w: f32,
        max_h: f32,
        phys_w: f32,
        phys_h: f32,
    ) -> (f32, f32) {
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

        (
            f32::from(x) * self.x_multiplier,
            f32::from(y) * self.y_multiplier,
        )
    }
}
