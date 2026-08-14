use crate::engine::state::{EngineStatus, LockRecoveryExt, SharedState};
use crossbeam_channel::{Receiver, Sender, bounded};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::{LazyLock, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const POSTHOG_API_KEY: Option<&str> = option_env!("POSTHOG_API_KEY");
const POSTHOG_BATCH_URL: &str = "https://eu.i.posthog.com/batch/";

static TELEMETRY_SENDER: OnceLock<Sender<TelemetryMessage>> = OnceLock::new();

/// A random identifier generated once per process run, attached to every event.
/// Lets `PostHog` group/replay all events emitted by a single launch of the app,
/// independent of the stable per-install `distinct_id`.
static SESSION_ID: LazyLock<String> = LazyLock::new(|| uuid::Uuid::new_v4().to_string());

/// Marks (approximately) when this run started, so `app_closed` can report a session
/// duration. Forced to initialize as early as possible from `TelemetryService::init`.
static SESSION_START: LazyLock<Instant> = LazyLock::new(Instant::now);

#[derive(Debug)]
enum TelemetryMessage {
    Event {
        event_name: String,
        properties: Option<Value>,
        set_properties: Option<Value>,
        dedup_key: Option<String>,
    },
    /// Requests an immediate flush of any queued/batched events, acknowledging
    /// via `ack` once done so the caller can bound how long it waits before exit.
    Shutdown { ack: Sender<()> },
}

pub struct TelemetryService;

impl TelemetryService {
    /// Initializes the global telemetry worker. Should be called once at startup.
    pub fn init(telemetry_id: String, enabled: bool) {
        LazyLock::force(&SESSION_START);

        if !enabled {
            return;
        }

        let Some(api_key) = POSTHOG_API_KEY else {
            return; // No API key, no telemetry
        };

        // Create a bounded channel with a large enough capacity to handle bursts,
        // but not so large that it consumes too much memory.
        let (sender, receiver) = bounded(1000);

        if TELEMETRY_SENDER.set(sender).is_ok() {
            let worker = TelemetryWorker {
                receiver,
                api_key: api_key.to_string(),
                distinct_id: telemetry_id,
                batch_queue: Vec::new(),
                sent_dedup_keys: HashSet::new(),
            };

            if let Err(e) = thread::Builder::new()
                .name("TelemetryWorker".into())
                .spawn(move || worker.run())
            {
                log::error!(target: "Telemetry", "Failed to spawn TelemetryWorker: {e}");
            }
        }
    }

    /// Blocks (up to `timeout`) until the worker has flushed any queued events.
    ///
    /// The worker thread is detached and the app has no `Drop` hook that runs before
    /// `std::process::exit`/the end of `main`, so without this call any events queued
    /// in the last few seconds before shutdown (including a final `app_closed` event)
    /// would be silently discarded when the process exits.
    pub fn shutdown(timeout: Duration) {
        let Some(sender) = TELEMETRY_SENDER.get() else {
            return;
        };

        let (ack_tx, ack_rx) = bounded(1);
        if sender
            .send_timeout(TelemetryMessage::Shutdown { ack: ack_tx }, timeout)
            .is_err()
        {
            log::trace!(target: "Telemetry", "Could not queue telemetry shutdown flush in time");
            return;
        }

        if ack_rx.recv_timeout(timeout).is_err() {
            log::trace!(target: "Telemetry", "Telemetry shutdown flush timed out");
        }
    }
}

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
    let session_duration_secs = SESSION_START.elapsed().as_secs();
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

fn send_message(msg: TelemetryMessage) {
    if let Some(sender) = TELEMETRY_SENDER.get() {
        // try_send is completely non-blocking. If the queue is full (1000 events backlogged),
        // we just drop the event to prioritize driver performance over telemetry.
        if let Err(e) = sender.try_send(msg) {
            log::trace!(target: "Telemetry", "Dropped telemetry event: {e}");
        }
    }
}

struct TelemetryWorker {
    receiver: Receiver<TelemetryMessage>,
    api_key: String,
    distinct_id: String,
    batch_queue: Vec<Value>,
    sent_dedup_keys: HashSet<String>,
}

impl TelemetryWorker {
    fn run(mut self) {
        let batch_size_limit = 50;
        let batch_timeout = Duration::from_secs(5);

        // Use a single ureq agent to reuse connections
        let agent = ureq::builder().timeout(Duration::from_secs(10)).build();

        loop {
            match self.receiver.recv_timeout(batch_timeout) {
                Ok(TelemetryMessage::Event {
                    event_name,
                    properties,
                    set_properties,
                    dedup_key,
                }) => {
                    if let Some(ref key) = dedup_key {
                        let full_key = format!("{event_name}_{key}");
                        if self.sent_dedup_keys.contains(&full_key) {
                            continue; // Deduplicated, ignore this event
                        }
                        self.sent_dedup_keys.insert(full_key);
                    }

                    let payload = self.build_event_payload(
                        &event_name,
                        properties.as_ref(),
                        set_properties.as_ref(),
                    );
                    self.batch_queue.push(payload);

                    if self.batch_queue.len() >= batch_size_limit {
                        self.flush_batch(&agent);
                    }
                }
                Ok(TelemetryMessage::Shutdown { ack }) => {
                    self.flush_batch(&agent);
                    let _ = ack.send(());
                    break;
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    if !self.batch_queue.is_empty() {
                        self.flush_batch(&agent);
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    self.flush_batch(&agent);
                    break;
                }
            }
        }
    }

    fn build_event_payload(
        &self,
        event_name: &str,
        custom_props: Option<&Value>,
        set_props: Option<&Value>,
    ) -> Value {
        let mut props = json!({
            "distinct_id": self.distinct_id,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "version": crate::VERSION,
            "session_id": SESSION_ID.as_str(),
            "$lib": "ureq_batch",
            "$lib_version": "2.12.1",
            "$set_once": {
                "initial_os": std::env::consts::OS,
                "initial_arch": std::env::consts::ARCH,
            },
        });

        if let Some(payload_props) = props.as_object_mut() {
            if let Some(custom_obj) = custom_props.and_then(|c| c.as_object()) {
                for (k, v) in custom_obj {
                    payload_props.insert(k.clone(), v.clone());
                }
            }
            if let Some(set_p) = set_props {
                payload_props.insert("$set".to_string(), set_p.clone());
            }
        }

        json!({
            "event": event_name,
            "properties": props,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    }

    fn flush_batch(&mut self, agent: &ureq::Agent) {
        if self.batch_queue.is_empty() {
            return;
        }

        let batch_payload = json!({
            "api_key": self.api_key,
            "batch": self.batch_queue,
        });

        let res = agent
            .post(POSTHOG_BATCH_URL)
            .set("Content-Type", "application/json")
            .send_json(&batch_payload);

        match res {
            Ok(_) => {
                log::trace!(target: "Telemetry", "Successfully sent batch of {} events", self.batch_queue.len());
                self.batch_queue.clear();
            }
            Err(e) => {
                log::trace!(target: "Telemetry", "Failed to send telemetry batch: {e}");
                // Simple offline resilience: if the batch fails, we keep the queue.
                // To avoid memory leak if offline forever, cap the queue at 500.
                if self.batch_queue.len() > 500 {
                    // Drain the oldest items to make room
                    self.batch_queue.drain(0..(self.batch_queue.len() - 500));
                }
            }
        }
    }
}

pub fn setup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let payload = panic_info.to_string();
        let anonymized = anonymize_path(&payload);

        let crash_file = crate::settings::get_settings_dir().join("crash_report.json");
        if let Ok(json) = serde_json::to_string(&json!({ "panic_message": anonymized })) {
            let _ = std::fs::write(crash_file, json);
        }

        default_hook(panic_info);
    }));
}

pub fn send_pending_crash_reports() {
    let crash_file = crate::settings::get_settings_dir().join("crash_report.json");
    if crash_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&crash_file)
            && let Ok(json) = serde_json::from_str::<Value>(&content)
        {
            capture_event("app_panicked", Some(json));
        }
        let _ = std::fs::remove_file(crash_file);
    }
}

fn anonymize_path(message: &str) -> String {
    let user_profile = std::env::var("USERPROFILE").ok();
    let home = std::env::var("HOME").ok();
    let user = std::env::var("USER").ok();
    let username_win = std::env::var("USERNAME").ok();

    anonymize_path_impl(
        message,
        user_profile.as_deref(),
        home.as_deref(),
        user.as_deref().or(username_win.as_deref()),
    )
}

fn anonymize_path_impl(
    message: &str,
    user_profile: Option<&str>,
    home: Option<&str>,
    user: Option<&str>,
) -> String {
    let mut cleaned = message.to_string();

    if let Some(user_profile) = user_profile {
        let username = user_profile.split(['/', '\\']).next_back().unwrap_or("");
        if !username.is_empty() {
            cleaned = cleaned.replace(username, "<HIDDEN>");
        }
    }

    if let Some(home) = home {
        let username = home.split(['/', '\\']).next_back().unwrap_or("");
        if !username.is_empty() {
            cleaned = cleaned.replace(username, "<HIDDEN>");
        }
    }

    if let Some(user) = user
        && !user.is_empty()
    {
        cleaned = cleaned.replace(user, "<HIDDEN>");
    }

    cleaned
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn test_anonymize_path_windows() {
        let msg = r"panic at C:\Users\JohnDoe\Projects\NextTabletDriver\src\main.rs";
        let cleaned = anonymize_path_impl(msg, Some(r"C:\Users\JohnDoe"), None, Some("JohnDoe"));
        assert_eq!(
            cleaned,
            r"panic at C:\Users\<HIDDEN>\Projects\NextTabletDriver\src\main.rs"
        );
    }

    #[test]
    fn test_anonymize_path_linux() {
        let msg = "panic at /home/johndoe/Projects/NextTabletDriver/src/main.rs";
        let cleaned = anonymize_path_impl(msg, None, Some("/home/johndoe"), Some("johndoe"));
        assert_eq!(
            cleaned,
            "panic at /home/<HIDDEN>/Projects/NextTabletDriver/src/main.rs"
        );
    }

    #[test]
    fn test_anonymize_path_no_env() {
        let msg = "panic at /home/johndoe/Projects/NextTabletDriver/src/main.rs";
        let cleaned = anonymize_path_impl(msg, None, None, None);
        assert_eq!(
            cleaned,
            "panic at /home/johndoe/Projects/NextTabletDriver/src/main.rs"
        );
    }
}
