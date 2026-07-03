use serde::Deserialize;
use std::path::PathBuf;

/// Represents a GitHub Release object returned from the API.
#[derive(Deserialize, Clone)]
pub struct Release {
    pub tag_name: String,
    pub body: Option<String>,
    pub assets: Vec<Asset>,
}

/// Represents an individual file (asset) attached to a GitHub Release.
#[derive(Deserialize, Clone)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Clone, Default)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub speed: u64,
}

/// Represents the current phase/status of the update process.
/// This enum is passed via channels from the update thread to the main UI thread.
#[derive(Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(Release),
    Downloading(DownloadProgress),
    ReadyToInstall(PathBuf),
    Error(String),
}

impl UpdateStatus {
    #[must_use]
    pub const fn as_release(&self) -> Option<&Release> {
        if let Self::Available(release) = self {
            Some(release)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Version {
    pub major: u32,
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub patch: u32,
}

impl Version {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('v');
        let parts: Vec<&str> = s.split('.').collect();
        match parts.as_slice() {
            [major_s, year_s, ddmm, patch_s] => {
                let major = major_s.parse().ok()?;
                let year = year_s.parse().ok()?;

                if ddmm.len() != 4 {
                    return None;
                }
                let day = ddmm.get(0..2)?.parse().ok()?;
                let month = ddmm.get(2..4)?.parse().ok()?;

                let patch = patch_s.parse().ok()?;

                Some(Self {
                    major,
                    year,
                    month,
                    day,
                    patch,
                })
            }
            _ => None,
        }
    }
}
