//! Panic hook and pending crash-report replay, with user-path anonymization.

use super::capture_event;
use serde_json::{Value, json};

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
