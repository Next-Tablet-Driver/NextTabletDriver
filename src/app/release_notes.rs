//! # Release Notes Fetching
//!
//! Fetches the release history from the GitHub Releases API for the "Release"
//! tab. Network calls retry with exponential backoff and jitter, and results
//! are cached to disk so the tab still shows the last known notes when
//! offline.

use super::autoupdate::models::Release;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const MAX_ATTEMPTS: u32 = 4;
const BASE_DELAY_MS: u64 = 500;
const MAX_DELAY_MS: u64 = 8000;

fn cache_path() -> PathBuf {
    crate::settings::get_settings_dir().join("cache_releases.json")
}

fn load_cache() -> Option<Vec<Release>> {
    let content = fs::read_to_string(cache_path()).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_cache(releases: &[Release]) {
    let path = cache_path();
    match serde_json::to_string_pretty(releases) {
        Ok(json) => {
            let tmp = path.with_extension("json.tmp");
            if let Err(e) = fs::write(&tmp, &json) {
                log::error!(target: "ReleaseNotes", "Failed to write temp release cache: {e}");
            } else if let Err(e) = fs::rename(&tmp, &path) {
                log::error!(target: "ReleaseNotes", "Failed to rename temp release cache: {e}");
            }
        }
        Err(e) => log::error!(target: "ReleaseNotes", "Failed to serialize release cache: {e}"),
    }
}

/// A pseudo-random fraction in `[0.0, 1.0)`, used for jitter. `RandomState`
/// draws from the OS RNG to seed its hasher, so this avoids pulling in a
/// dedicated `rand` dependency for a single call site.
fn jitter_fraction() -> f64 {
    use std::hash::{BuildHasher, Hasher};
    let hash = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    (hash % 1000) as f64 / 1000.0
}

/// Full-jitter exponential backoff: a random delay between 0 and
/// `min(MAX_DELAY_MS, BASE_DELAY_MS * 2^attempt)`.
fn backoff_delay(attempt: u32) -> Duration {
    let exp_ms = BASE_DELAY_MS.saturating_mul(1u64 << attempt.min(6));
    let capped_ms = exp_ms.min(MAX_DELAY_MS);
    let jittered_ms = (capped_ms as f64 * jitter_fraction()) as u64;
    Duration::from_millis(jittered_ms)
}

fn fetch_with_retry() -> Result<Vec<Release>, String> {
    let mut last_err = String::new();
    for attempt in 0..MAX_ATTEMPTS {
        match super::autoupdate::fetch_releases() {
            Ok(releases) => return Ok(releases),
            Err(e) => {
                log::warn!(
                    target: "ReleaseNotes",
                    "Fetch attempt {}/{MAX_ATTEMPTS} failed: {e}", attempt + 1
                );
                last_err = e;
                if attempt + 1 < MAX_ATTEMPTS {
                    std::thread::sleep(backoff_delay(attempt));
                }
            }
        }
    }
    Err(last_err)
}

/// Outcome of a release notes fetch, distinguishing fresh network data from
/// data served out of the local cache after all retries failed.
pub enum ReleaseNotesOutcome {
    Fresh(Vec<Release>),
    Cached(Vec<Release>),
    Unavailable,
}

/// Fetches the release list, retrying with exponential backoff and jitter.
///
/// Falls back to the on-disk cache when offline. Meant to be called from a
/// background thread since it blocks on I/O.
#[must_use]
pub fn get_releases() -> ReleaseNotesOutcome {
    match fetch_with_retry() {
        Ok(releases) => {
            save_cache(&releases);
            ReleaseNotesOutcome::Fresh(releases)
        }
        Err(e) => {
            log::warn!(
                target: "ReleaseNotes",
                "All fetch attempts failed ({e}), falling back to cache"
            );
            load_cache().map_or(ReleaseNotesOutcome::Unavailable, ReleaseNotesOutcome::Cached)
        }
    }
}
