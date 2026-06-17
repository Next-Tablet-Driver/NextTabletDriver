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

fn anonymize_path(message: &str) -> String {
    // A simple regex or string replacement to hide local usernames.
    // e.g. C:\Users\Username\Documents -> C:\Users\<HIDDEN>\Documents
    let mut cleaned = message.to_string();
    if let Ok(user_profile) = std::env::var("USERPROFILE")
        && let Some(username) = std::path::Path::new(&user_profile)
            .file_name()
            .and_then(|n| n.to_str())
    {
        cleaned = cleaned.replace(username, "<HIDDEN>");
    }
    cleaned
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

pub fn send_pending_crash_reports(app_prefs: &crate::settings::app_preferences::AppPreferences) {
    let crash_file = crate::settings::get_settings_dir().join("crash_report.json");
    if crash_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&crash_file)
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
        {
            capture_event("app_panicked", Some(json), app_prefs);
        }
        let _ = std::fs::remove_file(crash_file);
    }
}
