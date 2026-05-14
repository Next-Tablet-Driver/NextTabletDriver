//! # Application State and Lifecycle
//!
//! This module contains the core state and lifecycle management for the GUI application.

pub mod autoupdate;
pub mod lifecycle;
pub mod state;
pub mod update;
pub mod websocket;
pub mod events;
pub mod layout;
pub mod services;

pub use state::{AppTab, TabletMapperApp};
