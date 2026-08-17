//! Telemetry: anonymous usage analytics sent to `PostHog`, plus crash-report capture.
//!
//! - [`events`] holds the public event-capture API (`capture_event*`, `capture_app_closed`).
//! - [`worker`] is the background thread that batches events and posts them to `PostHog`.
//! - [`crash`] is the panic hook and pending crash-report replay, with path anonymization.

mod crash;
mod events;
mod worker;

pub use crash::{send_pending_crash_reports, setup_panic_hook};
pub use events::{capture_app_closed, capture_event, capture_event_dedup, capture_event_with_set};

use crossbeam_channel::{Sender, bounded};
use serde_json::Value;
use std::sync::{LazyLock, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use worker::TelemetryWorker;

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
            let worker = TelemetryWorker::new(receiver, api_key.to_string(), telemetry_id);

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

fn send_message(msg: TelemetryMessage) {
    if let Some(sender) = TELEMETRY_SENDER.get() {
        // try_send is completely non-blocking. If the queue is full (1000 events backlogged),
        // we just drop the event to prioritize driver performance over telemetry.
        if let Err(e) = sender.try_send(msg) {
            log::trace!(target: "Telemetry", "Dropped telemetry event: {e}");
        }
    }
}
