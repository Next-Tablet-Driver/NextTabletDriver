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

    use crate::settings::set_test_settings_dir;

    fn temp_settings(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ntd_crash_{name}_{nanos}"));
        std::fs::create_dir_all(&path).unwrap();
        set_test_settings_dir(path.clone());
        path
    }

    #[test]
    fn a_pending_report_is_consumed_once_it_is_sent() {
        let dir = temp_settings("pending");
        let file = dir.join("crash_report.json");
        std::fs::write(&file, r#"{ "panic_message": "boom" }"#).unwrap();
        send_pending_crash_reports();
        assert!(!file.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_report_is_discarded_rather_than_retried_forever() {
        let dir = temp_settings("garbage");
        let file = dir.join("crash_report.json");
        std::fs::write(&file, "not json at all").unwrap();
        send_pending_crash_reports();
        assert!(!file.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn nothing_happens_without_a_pending_report() {
        let dir = temp_settings("none");
        send_pending_crash_reports();
        assert!(!dir.join("crash_report.json").exists());
        let _ = std::fs::remove_dir_all(dir);
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
        let dir = temp_settings("hook");
        setup_panic_hook();
        let result = std::panic::catch_unwind(|| {
            panic!("boom in C:\\Users\\Nobody\\src");
        });
        // Put the default hook back for whatever runs next in this process.
        let _ = std::panic::take_hook();
        assert!(result.is_err());

        let report = std::fs::read_to_string(dir.join("crash_report.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert!(json["panic_message"].as_str().unwrap().contains("boom"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
