//! # Application State and Lifecycle
//!
//! This module contains the core state and lifecycle management for the GUI application.
//! It handles the main application loop, state transitions, tab navigation, and the
//! integration with auto-updating and WebSocket systems.

pub mod autoupdate;
pub mod lifecycle;
pub mod state;
pub mod update;
pub mod websocket;

pub use state::{AppTab, TabletMapperApp};
