use super::models::{Asset, Release, UpdateStatus};
use hex;
use sha2::{Digest, Sha256};
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
    release: &Release,
    status_sender: &crossbeam_channel::Sender<UpdateStatus>,
) -> Result<(), Box<dyn std::error::Error>> {
    let asset = find_platform_asset(release)?;
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

    let _total_size = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut temp_path = crate::settings::get_settings_dir().join("updates");
    fs::create_dir_all(&temp_path)?;
    temp_path.push(&asset.name);

    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];
    let mut reader = response.into_reader();

    let mut last_update_time = std::time::Instant::now();
    let mut last_downloaded: u64 = 0;

    {
        let mut file = fs::File::create(&temp_path)?;
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            if let Some(chunk) = buffer.get(..bytes_read) {
                file.write_all(chunk)?;
            }
            downloaded += bytes_read as u64;

            let now = std::time::Instant::now();
            let elapsed_ms = now.duration_since(last_update_time).as_millis();
            if elapsed_ms >= 250 {
                let elapsed_secs = elapsed_ms as f64 / 1000.0;
                let speed = ((downloaded - last_downloaded) as f64 / elapsed_secs) as u64;
                last_update_time = now;
                last_downloaded = downloaded;

                let _ = status_sender.send(UpdateStatus::Downloading(
                    crate::app::autoupdate::models::DownloadProgress { downloaded, speed },
                ));
            }
        }

        let _ = status_sender.send(UpdateStatus::Downloading(
            crate::app::autoupdate::models::DownloadProgress {
                downloaded,
                speed: 0,
            },
        ));
    }

    // Verify SHA256 mandatory - read the file back from disk to prevent TOCTOU attacks
    let actual = {
        let mut file = fs::File::open(&temp_path)?;
        let mut file_hasher = Sha256::new();
        let mut file_buffer = [0u8; 8192];
        loop {
            let bytes_read = file.read(&mut file_buffer)?;
            if bytes_read == 0 {
                break;
            }
            if let Some(chunk) = file_buffer.get(..bytes_read) {
                file_hasher.update(chunk);
            }
        }
        hex::encode(file_hasher.finalize())
    };

    if actual.to_lowercase() != expected_hash.to_lowercase() {
        let _ = fs::remove_file(&temp_path);
        return Err(
            format!("Checksum mismatch! Expected: {expected_hash}, Actual: {actual}").into(),
        );
    }
    log::info!(target: "Update::Verify", "SHA256 integrity verified successfully.");

    log::info!(target: "Update::Download", "Download complete, saved to {}", temp_path.display());

    // Make the file executable on Linux (or extract if it is a tar.gz archive)
    #[cfg(target_os = "linux")]
    let launch_path = if asset.name.ends_with(".tar.gz") {
        let updates_dir = temp_path
            .parent()
            .ok_or("Failed to get parent updates directory")?;
        log::info!(target: "Update::Extract", "Extracting tar.gz archive {} to {}", temp_path.display(), updates_dir.display());

        let extract_status = Command::new("tar")
            .arg("-xzf")
            .arg(&temp_path)
            .arg("-C")
            .arg(updates_dir)
            .status();

        match extract_status {
            Ok(s) if s.success() => {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::remove_file(&temp_path);
                let extracted_bin = updates_dir.join("next_tablet_driver");
                let _ = fs::set_permissions(&extracted_bin, fs::Permissions::from_mode(0o755));
                extracted_bin
            }
            Ok(s) => {
                let _ = fs::remove_file(&temp_path);
                return Err(format!("Failed to extract tar.gz archive: exit code {s}").into());
            }
            Err(e) => {
                let _ = fs::remove_file(&temp_path);
                return Err(format!("Failed to run tar command: {e}").into());
            }
        }
    } else {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o755));
        temp_path.clone()
    };

    #[cfg(target_os = "linux")]
    let status = Command::new(&launch_path).spawn();

    #[cfg(not(target_os = "linux"))]
    let status = Command::new(&temp_path)
        .args([
            "/SP-",
            "/SILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/CLOSEAPPLICATIONS",
            "/TASKS=desktopicon",
        ])
        .spawn();

    match status {
        Ok(_) => {
            log::info!(target: "Update::Process", "Installer launched, exiting...");
            crate::app::telemetry::capture_event(
                "update_installed",
                Some(serde_json::json!({ "new_version": release.tag_name })),
            );
            // This is a hard process exit: nothing else runs afterwards, so the
            // telemetry worker must be given a chance to flush before we terminate.
            crate::app::telemetry::TelemetryService::shutdown(std::time::Duration::from_millis(
                1500,
            ));
            #[allow(clippy::exit)]
            std::process::exit(0);
        }
        Err(e) => {
            #[cfg(target_os = "linux")]
            let _ = fs::remove_file(&launch_path);
            #[cfg(not(target_os = "linux"))]
            let _ = fs::remove_file(&temp_path);

            log::error!(target: "Update::Process", "Failed to launch installer: {e}");
            crate::app::telemetry::capture_event(
                "update_failed",
                Some(serde_json::json!({
                    "error_message": e.to_string(),
                    "context": "Launch Installer"
                })),
            );
            Err("Installer launch failed".into())
        }
    }
}

/// Finds the appropriate release asset for the current platform.
fn find_platform_asset(release: &Release) -> Result<&Asset, Box<dyn std::error::Error>> {
    #[cfg(windows)]
    return release
        .assets
        .iter()
        .find(|a| {
            std::path::Path::new(&a.name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        })
        .ok_or_else(|| "No suitable installer (.exe) asset found in release".into());

    #[cfg(target_os = "linux")]
    return release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".AppImage"))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".tar.gz")))
        .ok_or_else(|| "No suitable Linux asset found in release".into());

    #[cfg(not(any(windows, target_os = "linux")))]
    return Err("Unsupported platform for auto-update".into());
}
