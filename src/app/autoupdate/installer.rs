use super::models::{Asset, Release, UpdateStatus};
use hex;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::process::Command;

/// Downloads the specified release installer and launches it.
///
/// # Errors
/// Returns an error if:
/// - No suitable installer asset or checksum file is found in the release.
/// - The download request fails or returns a non-200 status.
/// - File I/O operations (creation, writing, renaming) fail.
/// - The downloaded file's SHA256 checksum does not match the expected hash.
/// - The installer process fails to launch.
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
        .ok_or(
            "Security Error: No .sha256 checksum file found for this asset. Installation aborted.",
        )?;

    log::info!(target: "Update::Download", "Found checksum asset: {}", checksum_asset.name);
    let resp = ureq::get(&checksum_asset.browser_download_url)
        .set("User-Agent", "NextTabletDriver-AutoUpdate")
        .call()?;
    let hash_str = resp.into_string()?;
    let expected_hash = hash_str.split_whitespace().next().unwrap_or("").to_string();

    log::info!(target: "Update::Download", "Downloading update from {download_url}");

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
            if let Some(chunk) = buffer.get(..bytes_read) {
                file.write_all(chunk)?;
                hasher.update(chunk);
            }
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
            "Checksum mismatch! Expected: {expected_hash}, Actual: {actual}"
        )
        .into());
    }
    log::info!(target: "Update::Verify", "SHA256 integrity verified successfully.");

    log::info!(target: "Update::Download", "Download complete, saved to {temp_path:?}");

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
            #[allow(clippy::exit)]
            std::process::exit(0);
        }
        Err(e) => {
            let _ = fs::remove_file(&temp_path);
            log::error!(target: "Update::Process", "Failed to launch installer: {e}");
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
