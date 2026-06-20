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

    if let Some(user_profile) = user_profile
        && let Some(username) = std::path::Path::new(user_profile)
            .file_name()
            .and_then(|n| n.to_str())
        && !username.is_empty()
    {
        cleaned = cleaned.replace(username, "<HIDDEN>");
    }

    if let Some(home) = home
        && let Some(username) = std::path::Path::new(home)
            .file_name()
            .and_then(|n| n.to_str())
        && !username.is_empty()
    {
        cleaned = cleaned.replace(username, "<HIDDEN>");
    }

    if let Some(user) = user
        && !user.is_empty()
    {
        cleaned = cleaned.replace(user, "<HIDDEN>");
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
