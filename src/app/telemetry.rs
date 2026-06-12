use serde_json::json;
use std::thread;
use std::time::Duration;

// Use an environment variable at compile time to avoid hardcoding the key in open-source code.
const POSTHOG_API_KEY: Option<&str> = option_env!("POSTHOG_API_KEY");
const POSTHOG_URL: &str = "https://eu.i.posthog.com/capture/";

/// Sends an anonymous telemetry event to `PostHog` in the background.
///
/// If telemetry is disabled, this function does nothing.
pub fn capture_event(
    event_name: &str,
    properties: Option<serde_json::Value>,
    app_prefs: &crate::settings::app_preferences::AppPreferences,
) {
    let Some(api_key) = POSTHOG_API_KEY else {
        return; // No telemetry without an API key
    };

    if !app_prefs.telemetry_enabled {
        return;
    }

    let distinct_id = app_prefs.telemetry_id.clone();
    let event = event_name.to_string();
    let mut props = properties.unwrap_or_else(|| json!({}));

    // Inject global properties
    if let Some(obj) = props.as_object_mut() {
        obj.insert("os".to_string(), json!(std::env::consts::OS));
        obj.insert("arch".to_string(), json!(std::env::consts::ARCH));
        obj.insert("version".to_string(), json!(crate::VERSION));
    }

    let payload = json!({
        "api_key": api_key,
        "event": event,
        "properties": {
            "distinct_id": distinct_id,
            "$set_once": {
                "initial_os": std::env::consts::OS,
                "initial_arch": std::env::consts::ARCH,
            },
            "$lib": "ureq",
            "$lib_version": "2.12.1",
        },
    });

    // Merge custom properties into the payload's properties object
    let mut final_payload = payload;
    if let Some(payload_props) = final_payload
        .get_mut("properties")
        .and_then(|p| p.as_object_mut())
        && let Some(custom_props) = props.as_object()
    {
        for (k, v) in custom_props {
            payload_props.insert(k.clone(), v.clone());
        }
    }

    // Fire and forget on a separate thread to not block the UI
    thread::spawn(move || {
        let agent = ureq::builder().timeout(Duration::from_secs(5)).build();

        let res = agent
            .post(POSTHOG_URL)
            .set("Content-Type", "application/json")
            .send_json(final_payload);

        if let Err(e) = res {
            log::trace!(target: "Telemetry", "Failed to send telemetry event: {e}");
        }
    });
}
