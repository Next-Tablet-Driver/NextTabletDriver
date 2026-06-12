//! # Application State and Lifecycle
//!
//! This module contains the core state and lifecycle management for the GUI application.

pub mod autoupdate;
pub mod events;
pub mod layout;
pub mod lifecycle;
pub mod services;
pub mod state;
pub mod telemetry;
pub mod update;
pub mod websocket;

pub use state::{AppTab, TabletMapperApp};
