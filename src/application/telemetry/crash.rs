//! Panic hook and pending crash-report replay, with user-path anonymization.

use super::capture_event;
use ntd_settings::Paths;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Installs a panic hook that writes an anonymized crash report to the settings folder, then calls
/// the previous hook.
pub fn setup_panic_hook(paths: &Paths) {
    let crash_file = crash_file(paths.settings_dir());
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let payload = panic_info.to_string();
        let anonymized = anonymize_path(&payload);

        if let Ok(json) = serde_json::to_string(&json!({ "panic_message": anonymized })) {
            let _ = std::fs::write(&crash_file, json);
        }

        default_hook(panic_info);
    }));
}

/// Sends the crash report left by the previous run, if there is one, as a telemetry event, then
/// deletes it.
pub fn send_pending_crash_reports(paths: &Paths) {
    send_pending_crash_reports_with(paths.settings_dir(), &|name, properties| {
        capture_event(name, Some(properties));
    });
}

/// Where the report of a panic is left, in the settings folder `dir`.
fn crash_file(dir: &Path) -> PathBuf {
    dir.join("crash_report.json")
}

/// Replays the report left by the previous run through `send`, then deletes it; a report that
/// cannot be read is deleted without being sent so it is not retried forever.
fn send_pending_crash_reports_with(dir: &Path, send: &dyn Fn(&str, Value)) {
    let crash_file = crash_file(dir);
    if crash_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&crash_file)
            && let Ok(json) = serde_json::from_str::<Value>(&content)
        {
            send("app_panicked", json);
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

    /// Collects what a replay would hand to the telemetry worker.
    fn replay(dir: &Path) -> Vec<(String, Value)> {
        let sent = std::cell::RefCell::new(Vec::new());
        send_pending_crash_reports_with(dir, &|name, properties| {
            sent.borrow_mut().push((name.to_string(), properties));
        });
        sent.into_inner()
    }

    #[test]
    fn a_pending_report_is_sent_once_and_then_deleted() {
        let paths = crate::test_support::test_paths();
        let dir = paths.settings_dir().to_path_buf();
        let file = dir.join("crash_report.json");
        std::fs::write(&file, r#"{ "panic_message": "boom" }"#).unwrap();

        let sent = replay(&dir);
        assert_eq!(
            sent,
            [(
                "app_panicked".to_string(),
                json!({ "panic_message": "boom" })
            )]
        );
        assert!(!file.exists());
        assert!(replay(&dir).is_empty(), "the report must not be sent twice");
    }

    #[test]
    fn an_unreadable_report_is_discarded_rather_than_retried_forever() {
        let paths = crate::test_support::test_paths();
        let dir = paths.settings_dir().to_path_buf();
        let file = dir.join("crash_report.json");
        std::fs::write(&file, "not json at all").unwrap();
        assert!(replay(&dir).is_empty(), "nothing readable, nothing sent");
        assert!(!file.exists());
    }

    #[test]
    fn smoke_the_public_replay_does_not_fail_without_a_worker() {
        // Smoke test: with no telemetry worker the event is dropped, the file is still removed.
        let paths = crate::test_support::test_paths();
        let dir = paths.settings_dir().to_path_buf();
        let file = dir.join("crash_report.json");
        std::fs::write(&file, r#"{ "panic_message": "boom" }"#).unwrap();
        send_pending_crash_reports(&paths);
        assert!(!file.exists());
    }

    #[test]
    fn nothing_happens_without_a_pending_report() {
        let paths = crate::test_support::test_paths();
        let dir = paths.settings_dir().to_path_buf();
        assert!(replay(&dir).is_empty());
        assert!(!dir.join("crash_report.json").exists());
    }

    #[test]
    fn anonymizing_hides_every_known_user_name() {
        let message = r"panic at C:\Users\alice\src\main.rs and /home/bob/x (carol)";
        let cleaned = anonymize_path_impl(
            message,
            Some(r"C:\Users\alice"),
            Some("/home/bob"),
            Some("carol"),
        );
        assert!(!cleaned.contains("alice"));
        assert!(!cleaned.contains("bob"));
        assert!(!cleaned.contains("carol"));
        assert_eq!(cleaned.matches("<HIDDEN>").count(), 3);
    }

    #[test]
    fn anonymizing_leaves_messages_alone_without_user_information() {
        let message = "index out of bounds";
        assert_eq!(anonymize_path_impl(message, None, None, None), message);
        assert_eq!(
            anonymize_path_impl(message, Some(""), Some("/"), Some("")),
            message
        );
    }

    #[test]
    fn anonymizing_with_the_real_environment_hides_the_current_user() {
        for variable in ["USERNAME", "USER"] {
            if let Ok(user) = std::env::var(variable)
                && user.len() >= 3
            {
                let cleaned = anonymize_path(&format!("panic in /home/{user}/src/main.rs"));
                assert!(!cleaned.contains(&user), "{cleaned}");
            }
        }
        // Without any user information the message is returned as it was.
        assert_eq!(anonymize_path("index out of bounds"), "index out of bounds");
    }

    #[test]
    fn the_panic_hook_writes_an_anonymized_report_for_the_next_launch() {
        let paths = crate::test_support::test_paths();
        let dir = paths.settings_dir().to_path_buf();
        // Process-global state: installs a panic hook and restores the default one afterwards.
        // Safe under nextest (one process per test); see `.github/CONTRIBUTING.md`.
        setup_panic_hook(&paths);
        let result = std::panic::catch_unwind(|| {
            panic!("boom in C:\\Users\\Nobody\\src");
        });
        // Put the default hook back for whatever runs next in this process.
        let _ = std::panic::take_hook();
        assert!(result.is_err());

        let report = std::fs::read_to_string(dir.join("crash_report.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert!(json["panic_message"].as_str().unwrap().contains("boom"));
    }
}
