use super::models::{Release, Version};

const OWNER: &str = "Next-Tablet-Driver";
const REPO: &str = "NextTabletDriver";

fn github_api_url() -> String {
    format!("https://api.github.com/repos/{OWNER}/{REPO}/releases/latest")
}

fn github_releases_list_url() -> String {
    format!("https://api.github.com/repos/{OWNER}/{REPO}/releases?per_page=30")
}

/// Queries the GitHub API to check if a newer version is available.
///
/// # Errors
/// Returns an error if the network request fails, the GitHub API returns a non-200 status,
/// or the response body cannot be parsed as a valid release JSON.
pub fn check_for_updates() -> Result<Option<Release>, Box<dyn std::error::Error>> {
    let url = github_api_url();
    let response = ureq::get(&url)
        .set("User-Agent", "NextTabletDriver-AutoUpdate")
        .call()?;

    if response.status() != 200 {
        return Err(format!("GitHub API error: {}", response.status()).into());
    }

    let release: Release = response.into_json()?;

    let remote_version_str = &release.tag_name;
    let local_version_str = crate::VERSION;

    let remote_v = Version::parse(remote_version_str);
    let local_v = Version::parse(local_version_str);

    match (remote_v, local_v) {
        (Some(remote), Some(local)) if remote > local => {
            log::info!(
                target: "Update::Check",
                "New version available: {remote_version_str} (local version: {local_version_str})"
            );
            Ok(Some(release))
        }
        _ => {
            log::info!(
                target: "Update::Check",
                "No new updates found or version format mismatch. (Remote: {remote_version_str}, Local: {local_version_str})"
            );
            Ok(None)
        }
    }
}

/// Fetches the published release history from the GitHub API, most recent first.
///
/// # Errors
/// Returns an error if the network request fails, the GitHub API returns a
/// non-200 status, or the response body cannot be parsed.
pub fn fetch_releases() -> Result<Vec<Release>, String> {
    let response = ureq::get(&github_releases_list_url())
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
