use super::models::Release;

const OWNER: &str = "Next-Tablet-Driver";
const REPO: &str = "NextTabletDriver";

#[cfg(test)]
thread_local! {
    static TEST_URL: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

fn github_releases_list_url() -> String {
    #[cfg(test)]
    {
        if let Some(url) = TEST_URL.with(|url| url.borrow().clone()) {
            return url;
        }
    }
    format!("https://api.github.com/repos/{OWNER}/{REPO}/releases?per_page=30")
}

/// Fetches the published release history from the GitHub API, most recent first.
///
/// # Errors
/// Returns an error if the network request fails, the GitHub API returns a
/// non-200 status, or the response body cannot be parsed.
pub fn fetch_releases() -> Result<Vec<Release>, String> {
    fetch_releases_from(&github_releases_list_url())
}

fn fetch_releases_from(url: &str) -> Result<Vec<Release>, String> {
    let response = ureq::get(url)
        .set("User-Agent", "NextTabletDriver-AutoUpdate")
        .call()
        .map_err(|e| format!("Network error: {e}"))?;

    if response.status() != 200 {
        return Err(format!("GitHub API error: {}", response.status()));
    }

    response
        .into_json::<Vec<Release>>()
        .map_err(|e| format!("Failed to parse releases: {e}"))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Serves one canned HTTP response on a free local port and returns the URL to ask for it.
    fn serve_once(status_line: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0u8; 2048];
                let _ = stream.read(&mut request);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{address}/releases")
    }

    const RELEASES: &str = r#"[
        {
            "tag_name": "v2.1.0",
            "name": "Two point one",
            "body": "Notes",
            "published_at": "2026-10-01T10:00:00Z",
            "assets": [ { "name": "setup.exe", "browser_download_url": "https://example.test/setup.exe" } ]
        },
        { "tag_name": "v2.0.0", "body": null }
    ]"#;

    #[test]
    fn the_releases_url_targets_the_project_repository() {
        assert_eq!(
            github_releases_list_url(),
            "https://api.github.com/repos/Next-Tablet-Driver/NextTabletDriver/releases?per_page=30"
        );
    }

    #[test]
    fn a_release_list_is_parsed() {
        let releases = fetch_releases_from(&serve_once("200 OK", RELEASES)).unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag_name, "v2.1.0");
        assert_eq!(releases[0].assets[0].name, "setup.exe");
        assert_eq!(releases[1].tag_name, "v2.0.0");
        assert!(releases[1].assets.is_empty());
    }

    #[test]
    fn a_reply_that_is_not_a_release_list_is_reported() {
        let error = fetch_releases_from(&serve_once("200 OK", "{ not a list"))
            .err()
            .unwrap();
        assert!(error.starts_with("Failed to parse releases"), "{error}");
    }

    #[test]
    fn a_success_status_other_than_200_is_reported() {
        let error = fetch_releases_from(&serve_once("204 No Content", ""))
            .err()
            .unwrap();
        assert_eq!(error, "GitHub API error: 204");
    }

    #[test]
    fn an_error_status_is_a_network_error() {
        let error = fetch_releases_from(&serve_once("500 Internal Server Error", "{}"))
            .err()
            .unwrap();
        assert!(error.starts_with("Network error"), "{error}");
    }

    #[test]
    fn an_unreachable_server_is_a_network_error() {
        let free_port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let error = fetch_releases_from(&format!("http://127.0.0.1:{free_port}/"))
            .err()
            .unwrap();
        assert!(error.starts_with("Network error"), "{error}");
    }

    #[test]
    fn the_public_entry_point_uses_the_configured_endpoint() {
        TEST_URL.with(|url| *url.borrow_mut() = Some(serve_once("200 OK", RELEASES)));
        let releases = fetch_releases().unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag_name, "v2.1.0");
    }
}
