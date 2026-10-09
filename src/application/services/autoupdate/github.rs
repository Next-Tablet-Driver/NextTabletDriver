use super::models::Release;

const OWNER: &str = "Next-Tablet-Driver";
const REPO: &str = "NextTabletDriver";

fn github_releases_list_url() -> String {
    format!("https://api.github.com/repos/{OWNER}/{REPO}/releases?per_page=30")
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
