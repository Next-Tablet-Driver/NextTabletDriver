//! Background thread that batches captured events and POSTs them to `PostHog`.

use super::TelemetryMessage;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::time::Duration;

pub(super) struct TelemetryWorker {
    receiver: crossbeam_channel::Receiver<TelemetryMessage>,
    api_key: String,
    distinct_id: String,
    batch_queue: Vec<Value>,
    sent_dedup_keys: HashSet<String>,
}

impl TelemetryWorker {
    pub(super) fn new(
        receiver: crossbeam_channel::Receiver<TelemetryMessage>,
        api_key: String,
        distinct_id: String,
    ) -> Self {
        Self {
            receiver,
            api_key,
            distinct_id,
            batch_queue: Vec::new(),
            sent_dedup_keys: HashSet::new(),
        }
    }

    pub(super) fn run(mut self) {
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
            "session_id": super::SESSION_ID.as_str(),
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
            .post(super::POSTHOG_BATCH_URL)
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;

    fn worker() -> (crossbeam_channel::Sender<TelemetryMessage>, TelemetryWorker) {
        let (sender, receiver) = bounded(8);
        (
            sender,
            TelemetryWorker::new(receiver, "test-key".into(), "install-1".into()),
        )
    }

    #[test]
    fn events_carry_the_install_and_platform_properties() {
        let (_tx, worker) = worker();
        let payload = worker.build_event_payload("tablet_connected", None, None);
        assert_eq!(payload["event"], "tablet_connected");
        let props = &payload["properties"];
        assert_eq!(props["distinct_id"], "install-1");
        assert_eq!(props["os"], std::env::consts::OS);
        assert_eq!(props["arch"], std::env::consts::ARCH);
        assert_eq!(props["version"], crate::VERSION);
        assert_eq!(props["$set_once"]["initial_os"], std::env::consts::OS);
        assert!(props.get("$set").is_none());
        assert!(!props["session_id"].as_str().unwrap().is_empty());
    }

    #[test]
    fn the_timestamp_is_rfc3339() {
        let (_tx, worker) = worker();
        let payload = worker.build_event_payload("x", None, None);
        let stamp = payload["timestamp"].as_str().unwrap();
        assert!(
            chrono::DateTime::parse_from_rfc3339(stamp).is_ok(),
            "{stamp}"
        );
    }

    #[test]
    fn custom_properties_are_merged_and_set_properties_are_nested() {
        let (_tx, worker) = worker();
        let custom = json!({ "tablet": "CTL-472", "packets": 12 });
        let set = json!({ "plan": "free" });
        let payload = worker.build_event_payload("e", Some(&custom), Some(&set));
        let props = &payload["properties"];
        assert_eq!(props["tablet"], "CTL-472");
        assert_eq!(props["packets"], 12);
        assert_eq!(props["$set"]["plan"], "free");
        assert_eq!(props["distinct_id"], "install-1");
    }

    #[test]
    fn custom_properties_that_are_not_an_object_are_ignored() {
        let (_tx, worker) = worker();
        let payload = worker.build_event_payload("e", Some(&json!([1, 2, 3])), None);
        assert_eq!(payload["properties"]["distinct_id"], "install-1");
        assert!(payload["properties"].get("0").is_none());
    }

    #[test]
    fn the_worker_acknowledges_a_shutdown_and_stops() {
        let (tx, worker) = worker();
        let (ack_tx, ack_rx) = bounded(1);
        tx.send(TelemetryMessage::Shutdown { ack: ack_tx }).unwrap();
        // Nothing is queued, so nothing is sent over the network.
        worker.run();
        assert!(ack_rx.try_recv().is_ok());
    }

    #[test]
    fn the_worker_stops_when_every_sender_is_gone() {
        let (tx, worker) = worker();
        drop(tx);
        worker.run();
    }
}
