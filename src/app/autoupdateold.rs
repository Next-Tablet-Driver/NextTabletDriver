//! # Automatic Updates
//!
//! This module handles checking for, downloading, and launching new releases of
//! the application from GitHub. It operates in a background thread to prevent pausing
//! the main UI or input processing.

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;

use hex;
use sha2::{Digest, Sha256};

use serde::Deserialize;

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

/// Represents the current phase/status of the update process.
/// This enum is passed via channels from the update thread to the main UI thread.
#[derive(Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(Release),
    Downloading(f32),
    ReadyToInstall(PathBuf),
    Error(String),
}

impl UpdateStatus {
    pub fn as_release(&self) -> Option<&Release> {
        if let Self::Available(release) = self {
            Some(release)
        } else {
            None
        }
    }
}

const OWNER: &str = "Next-Tablet-Driver";
const REPO: &str = "NextTabletDriver";

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Version {
    pub major: u32,
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub patch: u32,
}

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('v');
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let year = parts[1].parse().ok()?;

        let ddmm = parts[2];
        if ddmm.len() != 4 {
            return None;
        }
        let day = ddmm[0..2].parse().ok()?;
        let month = ddmm[2..4].parse().ok()?;

        let patch = parts[3].parse().ok()?;

        Some(Self {
            major,
            year,
            month,
            day,
            patch,
        })
    }
}

fn github_api_url() -> String {
    format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        OWNER, REPO
    )
}

/// Queries the GitHub API to check if a newer version is available.
///
/// # Returns
/// - `Ok(Some(Release))` if the remote tag name is different from the local `VERSION`.
/// - `Ok(None)` if the current version matches the latest remote version.
/// - `Err` if the network request or parsing fails.
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
                "New version available: {} (local version: {})",
                remote_version_str,
                local_version_str
            );
            Ok(Some(release))
        }
        _ => {
            log::info!(
                target: "Update::Check",
                "No new updates found or version format mismatch. (Remote: {}, Local: {})",
                remote_version_str,
                local_version_str
            );
            Ok(None)
        }
    }
}

/// Downloads the specified release installer and launches it.
///
/// # Process
/// 1. Finds a suitable asset for the current platform.
/// 2. Downloads the binary to a temporary location.
/// 3. Verifies SHA256 integrity if a checksum file is available.
/// 4. Spawns the installer/updater process.
/// 5. Exits the current application instance so the installer can overwrite files.
pub fn download_and_install(
    release: Release,
    status_sender: crossbeam_channel::Sender<UpdateStatus>,
) -> Result<(), Box<dyn std::error::Error>> {
    let asset = find_platform_asset(&release)?;
    let download_url = &asset.browser_download_url;

    // Mandatory: Look for a .sha256 file in release assets
    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name == format!("{}.sha256", asset.name))
        .ok_or_else(|| "Security Error: No .sha256 checksum file found for this asset. Installation aborted.")?;

    log::info!(target: "Update::Download", "Found checksum asset: {}", checksum_asset.name);
    let resp = ureq::get(&checksum_asset.browser_download_url)
        .set("User-Agent", "NextTabletDriver-AutoUpdate")
        .call()?;
    let hash_str = resp.into_string()?;
    // Take the first word (the hash)
    let expected_hash = hash_str.split_whitespace().next().unwrap_or("").to_string();

    log::info!(target: "Update::Download", "Downloading update from {}", download_url);

    let response = ureq::get(download_url)
        .set("User-Agent", "NextTabletDriver-AutoUpdate")
        .call()?;

    if response.status() != 200 {
        return Err(format!("Download failed: {}", response.status()).into());
    }

    let total_size = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut temp_path = env::temp_dir();
    temp_path.push(&asset.name);

    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];
    let mut hasher = Sha256::new();
    let mut reader = response.into_reader();

    {
        let mut file = fs::File::create(&temp_path)?;
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            file.write_all(&buffer[..bytes_read])?;
            hasher.update(&buffer[..bytes_read]);
            downloaded += bytes_read as u64;

            if total_size > 0 {
                let progress = downloaded as f32 / total_size as f32;
                let _ = status_sender.send(UpdateStatus::Downloading(progress));
            }
        }
    }

    // Verify SHA256 mandatory
    let actual = hex::encode(hasher.finalize());
    if actual.to_lowercase() != expected_hash.to_lowercase() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "Checksum mismatch! Expected: {}, Actual: {}",
            expected_hash, actual
        )
        .into());
    }
    log::info!(target: "Update::Verify", "SHA256 integrity verified successfully.");

    log::info!(target: "Update::Download", "Download complete, saved to {:?}", temp_path);

    // Make the file executable on Linux
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o755));
    }

    let status = Command::new(&temp_path).spawn();

    match status {
        Ok(_) => {
            log::info!(target: "Update::Process", "Installer launched, exiting...");
            std::process::exit(0);
        }
        Err(e) => {
            let _ = fs::remove_file(&temp_path);
            log::error!(target: "Update::Process", "Failed to launch installer: {}", e);
            Err(e.into())
        }
    }
}

/// Finds the appropriate release asset for the current platform.
fn find_platform_asset(release: &Release) -> Result<&Asset, Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".exe"))
            .ok_or_else(|| "No suitable installer (.exe) asset found in release".into())
    }

    #[cfg(target_os = "linux")]
    {
        release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".AppImage"))
            .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".tar.gz")))
            .ok_or_else(|| "No suitable Linux asset found in release".into())
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err("Unsupported platform for auto-update".into())
    }
}
