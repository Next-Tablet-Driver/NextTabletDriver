use crate::drivers::NextTabletDriver;
use crate::drivers::config::{DigitizerIdentifier, TabletConfiguration};
use crate::drivers::config_loader::INDEXED_CONFIGS;
use crate::drivers::generic::GenericNextTabletDriver;
use hidapi::{HidApi, HidDevice};
use std::collections::HashMap;
use std::ffi::CStr;
use std::time::{Duration, Instant};

fn get_expected_interface(
    config: &TabletConfiguration,
    digitizer: &DigitizerIdentifier,
) -> Option<i32> {
    let value = digitizer
        .attributes
        .as_ref()
        .and_then(|a| a.interface.as_ref())
        .or_else(|| {
            config
                .attributes
                .as_ref()
                .and_then(|a| a.interface.as_ref())
        })?;

    match value {
        serde_json::Value::Number(num) => num.as_i64().map(|n| n as i32),
        serde_json::Value::String(s) => s.parse::<i32>().ok(),
        _ => None,
    }
}

/// The byte length a raw HID read actually produces for a config's declared
/// `InputReportLength`. On Linux, hidraw reads exclude the report-ID byte, so
/// the wire length matches the JSON value exactly (see the matching quirk in
/// `GenericNextTabletDriver::parse`). On Windows/macOS, hidapi includes the
/// leading report-ID byte in every read, so the wire length is one longer.
const fn expected_wire_report_len(input_report_length: usize) -> usize {
    #[cfg(target_os = "linux")]
    {
        input_report_length
    }
    #[cfg(not(target_os = "linux"))]
    {
        input_report_length + 1
    }
}

/// Confirms a candidate's declared report length against the real device.
///
/// On Windows, prefers the OS-reported `InputReportByteLength` via
/// `HidP_GetCaps`, which is a static device capability, not a data sample,
/// so it works even for tablets that stay silent until the pen approaches.
/// This mirrors the static `device.InputReportLength` match that
/// `OpenTabletDriver` performs in `Driver.cs`: a direct equality against the
/// JSON-declared `input_report_length`, with no report-ID-byte adjustment,
/// since the OS-reported capability already accounts for it the same way
/// `OpenTabletDriver`'s own HID backend does. Falls back to a live sample
/// read when the static query is unavailable (e.g. it fails to open the
/// device), and unconditionally on platforms other than Windows; the live
/// read path goes through `expected_wire_report_len` instead, since it
/// compares against actual bytes read off the wire rather than an OS-level
/// capability.
fn confirm_report_length(
    device: &HidDevice,
    path: &CStr,
    input_report_length: usize,
    config_name: &str,
) -> bool {
    #[cfg(windows)]
    if let Some(len) = crate::drivers::hid_caps::query_input_report_byte_length(path) {
        log::debug!(
            target: "Detect",
            "{config_name} | HidP_GetCaps: InputReportByteLength = {len} (expected {input_report_length})"
        );
        return len == input_report_length;
    }

    #[cfg(windows)]
    log::debug!(target: "Detect", "{config_name} | HidP_GetCaps unavailable, falling back to live sample");

    let expected_wire_len = expected_wire_report_len(input_report_length);
    let mut sample = [0u8; 64];
    let result = device.read_timeout(&mut sample, 750);
    log::debug!(
        target: "Detect",
        "{config_name} | Live sample: {result:?} (expected {expected_wire_len} bytes)"
    );
    matches!(result, Ok(n) if n == expected_wire_len)
}

/// Confirms a candidate's declared `DeviceStrings` regex patterns against
/// the real device's USB string descriptors.
///
/// Some tablet firmware revisions share the exact same `VendorID`,
/// `ProductID`, and `InputReportLength` (e.g. Gaomon S620 vs. Gaomon M106K
/// Pro, both `256c:006f` with `InputReportLength: 12`), so
/// `confirm_report_length` alone cannot tell them apart. Their configs
/// instead each declare a `DeviceStrings` regex (typically matched against
/// the firmware-version string at index `201`/`0xC9`) that only one
/// candidate's actual hardware satisfies. Mirrors `OpenTabletDriver`'s
/// `Driver.DeviceMatchesStrings` in `Driver.cs`.
///
/// `get_indexed_string` issues a plain `GET_DESCRIPTOR` control transfer, so
/// unlike the report-length live-sample fallback, it works immediately and
/// does not require the pen to be in proximity.
fn confirm_device_strings(
    device: &HidDevice,
    device_strings: &HashMap<u8, String>,
    config_name: &str,
) -> bool {
    for (&index, pattern) in device_strings {
        let value = match device.get_indexed_string(i32::from(index)) {
            Ok(Some(s)) => s,
            Ok(None) => {
                log::debug!(
                    target: "Detect",
                    "{config_name} | String index {index} not present on device"
                );
                return false;
            }
            Err(e) => {
                log::debug!(
                    target: "Detect",
                    "{config_name} | Failed to read string index {index}: {e}"
                );
                return false;
            }
        };

        let regex = match regex::Regex::new(pattern) {
            Ok(r) => r,
            Err(e) => {
                log::warn!(
                    target: "Detect",
                    "{config_name} | Invalid DeviceStrings pattern {pattern:?} for index {index}: {e}"
                );
                return false;
            }
        };

        if !regex.is_match(&value) {
            log::debug!(
                target: "Detect",
                "{config_name} | String index {index} = {value:?} did not match pattern {pattern:?}"
            );
            return false;
        }
    }
    true
}

/// Sends a candidate's feature/output init reports to wake it into the
/// expected mode. Returns `false` on the first failure.
fn send_init_reports(
    device: &HidDevice,
    digitizer: &DigitizerIdentifier,
    config_name: &str,
) -> bool {
    use base64::{Engine as _, engine::general_purpose};

    if let Some(reports) = &digitizer.feature_init_report {
        for report_str in reports {
            match general_purpose::STANDARD.decode(report_str) {
                Ok(data) => {
                    log::trace!(target: "Detect", "Sending Feature Report: {data:02x?}");
                    if let Err(e) = device.send_feature_report(&data) {
                        log::debug!(target: "Detect", "{config_name} | Init Error (Feature Report): {e}");
                        return false;
                    }
                }
                Err(e) => {
                    log::debug!(target: "Detect", "{config_name} | Base64 Decode Error (Feature): {e}");
                    return false;
                }
            }
        }
    }

    if let Some(reports) = &digitizer.output_init_report {
        for report_str in reports {
            match general_purpose::STANDARD.decode(report_str) {
                Ok(data) => {
                    log::trace!(target: "Detect", "Sending Output Report: {data:02x?}");
                    if let Err(e) = device.write(&data) {
                        log::debug!(target: "Detect", "{config_name} | Init Error (Output Report): {e}");
                        return false;
                    }
                }
                Err(e) => {
                    log::debug!(target: "Detect", "{config_name} | Base64 Decode Error (Output): {e}");
                    return false;
                }
            }
        }
    }

    true
}

use std::collections::HashSet;
use std::sync::Mutex;

static WARNED_UNSUPPORTED: Mutex<Option<HashSet<(u16, u16)>>> = Mutex::new(None);

#[must_use]
pub fn detect_tablet(api: &HidApi) -> Option<(HidDevice, Box<dyn NextTabletDriver>, u16, u16)> {
    let global_start = Instant::now();
    let devices: Vec<_> = api.device_list().collect();
    let enum_duration = global_start.elapsed();

    if enum_duration > Duration::from_millis(500) {
        log::warn!(target: "Detect", "HID Enumeration SLOW: {enum_duration:.2?}");
    }

    log::debug!(
        target: "Detect",
        "Starting scan of {} HID devices...",
        devices.len()
    );

    let index = &*INDEXED_CONFIGS;

    for device_info in devices {
        let vid = device_info.vendor_id();
        let pid = device_info.product_id();

        if !index.contains_key(&(vid, pid)) {
            let m_str = device_info
                .manufacturer_string()
                .unwrap_or("")
                .to_lowercase();
            let p_str = device_info.product_string().unwrap_or("").to_lowercase();
            let is_known_vid_or_pid = INDEXED_CONFIGS.keys().any(|&(v, p)| v == vid || p == pid);
            let is_tablet_brand = is_known_vid_or_pid
                || m_str.contains("tablet")
                || m_str.contains("digitizer")
                || m_str.contains("xp-pen")
                || m_str.contains("wacom")
                || m_str.contains("huion")
                || m_str.contains("gaomon")
                || m_str.contains("ugee")
                || m_str.contains("veikk")
                || p_str.contains("tablet")
                || p_str.contains("digitizer")
                || p_str.contains("pen display")
                || p_str.contains("drawing monitor");

            if is_tablet_brand {
                let is_new_entry = match WARNED_UNSUPPORTED.lock() {
                    Ok(mut guard) => guard.get_or_insert_with(HashSet::new).insert((vid, pid)),
                    Err(poisoned) => {
                        let mut guard = poisoned.into_inner();
                        guard.get_or_insert_with(HashSet::new).insert((vid, pid))
                    }
                };

                if is_new_entry {
                    let manufacturer = device_info.manufacturer_string().unwrap_or("<Unknown>");
                    let product = device_info.product_string().unwrap_or("<Unknown>");

                    log::warn!(
                        target: "Detect",
                        "Found unrecognized potential tablet device: [{vid:04x}:{pid:04x}] '{manufacturer}' - '{product}'.\n\
                        No configuration file was found for this model. You can create a custom JSON configuration \
                        in the 'tablets/' directory to add support for it."
                    );
                }
            }
            continue;
        }

        if let Some(matches) = index.get(&(vid, pid)) {
            // Several tablet firmware revisions share the same VID:PID, and
            // some (e.g. Gaomon S620 vs. M106K Pro) even share the same
            // declared InputReportLength. When more than one candidate is in
            // play, a successfully-initialized candidate is only accepted
            // once its declared DeviceStrings regexes (if any) match the
            // device's USB string descriptors and its report length (if
            // declared) is confirmed against an actual sample read,
            // mirroring OpenTabletDriver's `Driver.DeviceMatchesStrings` and
            // `device.InputReportLength` match filters in `Driver.cs`.
            let disambiguate = matches.len() > 1;
            let mut fallback: Option<(&TabletConfiguration, &DigitizerIdentifier)> = None;

            for (config, digitizer) in matches {
                let interface = device_info.interface_number();
                let path = device_info.path();

                if let Some(expected) = get_expected_interface(config, digitizer)
                    && expected != interface
                {
                    continue;
                }

                log::debug!(
                    target: "Detect",
                    "Found candidate for {}: {:04x}:{:04x} (Interface {}, Path: {:?})",
                    config.name,
                    vid,
                    pid,
                    interface,
                    path
                );

                let open_start = Instant::now();

                match api.open_path(path) {
                    Ok(device) => {
                        let open_duration = open_start.elapsed();
                        let init_start = Instant::now();

                        if !send_init_reports(&device, digitizer, &config.name) {
                            log::debug!(target: "Detect", "Initialization failed for {}, skipping device", config.name);
                            continue;
                        }

                        if disambiguate {
                            let strings_confirmed = digitizer
                                .device_strings
                                .as_ref()
                                .is_none_or(|ds| confirm_device_strings(&device, ds, &config.name));

                            let length_confirmed = strings_confirmed
                                && digitizer.input_report_length.is_none_or(|expected_len| {
                                    confirm_report_length(&device, path, expected_len, &config.name)
                                });

                            if !strings_confirmed || !length_confirmed {
                                log::debug!(
                                    target: "Detect",
                                    "{} | Candidate not confirmed (DeviceStrings: {}, InputReportLength: {}), trying next candidate",
                                    config.name,
                                    strings_confirmed,
                                    length_confirmed
                                );
                                if fallback.is_none() {
                                    fallback = Some((config, digitizer));
                                }
                                continue;
                            }
                        }

                        log::info!(
                            target: "Detect",
                            "Connected: {} ({:04x}:{:04x}) | Interface: {} | Init: {:.2?}",
                            config.name,
                            vid,
                            pid,
                            interface,
                            init_start.elapsed(),
                        );

                        log::info!(
                            target: "Tracking",
                            "TABLET DETAILS: Name: '{}' | Width: {}mm, Height: {}mm | MaxX: {}, MaxY: {} | MaxPressure: {} | Parser: {}",
                            config.name,
                            config.specifications.digitizer.width,
                            config.specifications.digitizer.height,
                            config.specifications.digitizer.max_x,
                            config.specifications.digitizer.max_y,
                            config.specifications.pen.max_pressure,
                            digitizer.report_parser
                        );

                        log::debug!(
                            target: "Detect",
                            "Timings -> Enum: {:.2?} | Open: {:.2?} | Total: {:.2?}",
                            enum_duration,
                            open_duration,
                            global_start.elapsed()
                        );

                        return Some((
                            device,
                            Box::new(GenericNextTabletDriver::new(
                                config.clone(),
                                digitizer,
                                vid,
                                pid,
                            )),
                            vid,
                            pid,
                        ));
                    }
                    Err(e) => {
                        let err_str = e.to_string().to_lowercase();
                        if err_str.contains("permission")
                            || err_str.contains("denied")
                            || err_str.contains("access")
                        {
                            log::error!(
                                target: "Detect",
                                "PERMISSION DENIED: Could not open tablet '{}' at path {:?} (Interface {}) due to insufficient privileges.\n\
                                \n\
                                TO FIX THIS (Linux):\n\
                                1. Create a custom udev rules file:\n\
                                   echo 'KERNEL==\"hidraw*\", SUBSYSTEM==\"hidraw\", ATTRS{{idVendor}}==\"{:04x}\", ATTRS{{idProduct}}==\"{:04x}\", MODE=\"0666\"' | sudo tee /etc/udev/rules.d/99-next-tablet.rules\n\
                                2. Reload and trigger udev rules:\n\
                                   sudo udevadm control --reload-rules && sudo udevadm trigger\n\
                                3. Re-plug your tablet device.",
                                config.name,
                                path,
                                interface,
                                vid,
                                pid
                            );
                        } else {
                            log::debug!(target: "Detect", "Could not open {} interface {}: {e}", config.name, interface);
                        }
                    }
                }
            }

            // No candidate's report length could be confirmed (e.g. the
            // tablet stays silent until the pen approaches). Fall back to
            // the first candidate that opened and initialized successfully
            // rather than failing detection outright.
            if let Some((config, digitizer)) = fallback {
                let path = device_info.path();
                if let Ok(device) = api.open_path(path)
                    && send_init_reports(&device, digitizer, &config.name)
                {
                    log::warn!(
                        target: "Detect",
                        "{:04x}:{:04x} | No candidate confirmed via report length, falling back to '{}'",
                        vid,
                        pid,
                        config.name
                    );
                    return Some((
                        device,
                        Box::new(GenericNextTabletDriver::new(
                            config.clone(),
                            digitizer,
                            vid,
                            pid,
                        )),
                        vid,
                        pid,
                    ));
                }
            }
        }
    }
    None
}
