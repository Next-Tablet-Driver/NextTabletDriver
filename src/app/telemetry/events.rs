//! Public event-capture API used throughout the app to record anonymous usage events.

use super::{TelemetryMessage, TelemetryService, send_message};
use crate::engine::state::{EngineStatus, LockRecoveryExt, SharedState};
use serde_json::{Value, json};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Helper function to send an event to the background worker.
pub fn capture_event(event_name: &str, properties: Option<Value>) {
    send_message(TelemetryMessage::Event {
        event_name: event_name.to_string(),
        properties,
        set_properties: None,
        dedup_key: None,
    });
}

/// Capture an event and update user profile ($set) simultaneously.
pub fn capture_event_with_set(
    event_name: &str,
    properties: Option<Value>,
    set_properties: Option<Value>,
) {
    send_message(TelemetryMessage::Event {
        event_name: event_name.to_string(),
        properties,
        set_properties,
        dedup_key: None,
    });
}

/// Capture an event but deduplicate it for the current session using the given key.
/// Helpful to avoid spamming `tablet_connected` 20 times if there is a USB bug.
pub fn capture_event_dedup(
    event_name: &str,
    properties: Option<Value>,
    set_properties: Option<Value>,
    dedup_key: &str,
) {
    send_message(TelemetryMessage::Event {
        event_name: event_name.to_string(),
        properties,
        set_properties,
        dedup_key: Some(dedup_key.to_string()),
    });
}

/// Captures a final `app_closed` summary event and blocks briefly to flush it.
///
/// There are multiple exit paths (graceful window close in `main.rs`, and the two
/// tray "Exit" menu handlers which call `std::process::exit` directly), and none of
/// them wait for background threads to finish. Call this at every one of those exit
/// points, right before the process actually terminates, so the event isn't dropped.
pub fn capture_app_closed(shared: &SharedState) {
    let session_duration_secs = super::SESSION_START.elapsed().as_secs();
    let total_packets_processed = shared.packet_count.load(Ordering::Relaxed);
    // A `vid` of 0 means the default/disconnected `DeviceState` (see `engine::state`),
    // which is more robust than comparing against its display name.
    let tablet_connected = shared.device_state.read().unwrap_or_log("device_state").vid != 0;
    let engine_failed = matches!(
        *shared.engine_status.read().unwrap_or_log("engine_status"),
        EngineStatus::Failed(_)
    );

    capture_event(
        "app_closed",
        Some(json!({
            "session_duration_secs": session_duration_secs,
            "total_packets_processed": total_packets_processed,
            "tablet_connected_at_exit": tablet_connected,
            "engine_failed_at_exit": engine_failed,
        })),
    );

    TelemetryService::shutdown(Duration::from_millis(1500));
}
