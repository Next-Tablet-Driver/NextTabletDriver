//! # Visual Theme and Custom Widgets
//!
//! This module configures the egui global styling to create a clean, modern
//! Light theme that visually aligns with the aesthetics of `OpenTabletDriver` (OTD).
//! It also provides reusable helper functions for consistent layout paradigms
//! (like section headers and standardized input boxes) across panels.
//!
//! - [`colors`] holds the semantic color palette and global style application.
//! - [`widgets`] holds reusable styled egui widget helpers built on top of it.

mod colors;
mod widgets;

pub use colors::{
    SemanticColors, accent_bg, apply_theme, label_color, panel_bg, panel_border, semantic_colors,
};
pub use widgets::{
    ui_card, ui_input_box, ui_input_box_range, ui_input_box_string, ui_input_box_u16,
    ui_input_box_u16_range, ui_input_box_u32, ui_input_box_u32_range, ui_labeled_box,
    ui_section_header, ui_setting_row, ui_setting_row_range,
};
