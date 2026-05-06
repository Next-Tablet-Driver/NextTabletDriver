//! # Tab Panels
//!
//! Each module within this directory encapsulates the layout and interactivity
//! for a specific tab in the main application view (e.g., Output Mapping, Filters, Settings).

pub mod console;
pub mod debugger;

pub mod filters;
pub mod output;
pub mod pen_settings;
pub mod performance;
pub mod release;
pub mod settings;

pub use debugger::render_debugger_panel;

pub use performance::render_performance_panel;
pub use release::render_release_panel;
