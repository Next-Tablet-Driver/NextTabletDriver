use crate::drivers::config_loader::INDEXED_CONFIGS;
use ntd_core::hid::{HidBackend, HidDevice, HidDeviceInfo, HidError, OpenOptions};
use ntd_tablets::NextTabletDriver;
use ntd_tablets::config::{DigitizerIdentifier, TabletConfiguration};
use ntd_tablets::generic::GenericNextTabletDriver;
use std::collections::HashMap;
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
/// When the backend can tell it (on Windows the OS-reported `InputReportByteLength` via
/// `HidP_GetCaps`), prefers that static device capability to a data sample, so it works even for
/// tablets that stay silent until the pen approaches.
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
    backend: &mut dyn HidBackend,
    info: &HidDeviceInfo,
    device: &dyn HidDevice,
    input_report_length: usize,
    config_name: &str,
) -> bool {
    if let Some(len) = backend.input_report_length(info) {
        log::debug!(
            target: "Detect",
            "{config_name} | Static report length = {len} (expected {input_report_length})"
        );
        return len == input_report_length;
    }

    log::debug!(target: "Detect", "{config_name} | No static report length, falling back to live \
        sample");

    let expected_wire_len = expected_wire_report_len(input_report_length);
    let mut sample = [0u8; 64];
    let result = device.read_timeout(&mut sample, Duration::from_millis(750));
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
    device: &dyn HidDevice,
    device_strings: &HashMap<u8, String>,
    config_name: &str,
) -> bool {
    for (&index, pattern) in device_strings {
        let value = match device.indexed_string(index) {
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
                    "{config_name} | Invalid DeviceStrings pattern {pattern:?} for index {index}: \
                        {e}"
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
    device: &dyn HidDevice,
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
                        log::debug!(target: "Detect", "{config_name} | Init Error (Feature \
                            Report): {e}");
                        return false;
                    }
                }
                Err(e) => {
                    log::debug!(target: "Detect", "{config_name} | Base64 Decode Error (Feature): \
                        {e}");
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
                        log::debug!(target: "Detect", "{config_name} | Init Error (Output Report): \
                            {e}");
                        return false;
                    }
                }
                Err(e) => {
                    log::debug!(target: "Detect", "{config_name} | Base64 Decode Error (Output): \
                        {e}");
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

/// A detected tablet: the open device, its driver, and its vendor and product IDs.
pub type DetectedTablet = (Box<dyn HidDevice>, Box<dyn NextTabletDriver>, u16, u16);

/// Looks for a supported tablet among the connected HID devices and opens it. Returns the device,
/// its driver and its vendor and product IDs, or `None` when no supported tablet is connected.
#[must_use]
pub fn detect_tablet(backend: &mut dyn HidBackend) -> Option<DetectedTablet> {
    let global_start = Instant::now();
    let devices = match backend.enumerate() {
        Ok(devices) => devices,
        Err(e) => {
            log::warn!(target: "Detect", "HID enumeration failed: {e}");
            return None;
        }
    };
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
        let vid = device_info.vendor_id;
        let pid = device_info.product_id;

        if !index.contains_key(&(vid, pid)) {
            let m_str = device_info
                .manufacturer
                .as_deref()
                .unwrap_or("")
                .to_lowercase();
            let p_str = device_info.product.as_deref().unwrap_or("").to_lowercase();
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
                    let manufacturer = device_info.manufacturer.as_deref().unwrap_or("<Unknown>");
                    let product = device_info.product.as_deref().unwrap_or("<Unknown>");

                    log::warn!(
                        target: "Detect",
                        "Found unrecognized potential tablet device: [{vid:04x}:{pid:04x}] \
                        '{manufacturer}' - '{product}'.\n\
                        No configuration file was found for this model. You can create a \
                        custom JSON configuration \
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
                let interface = device_info.interface_number;
                let path = &device_info.path;

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

                match backend.open(&device_info, OpenOptions::default()) {
                    Ok(device) => {
                        let open_duration = open_start.elapsed();
                        let init_start = Instant::now();

                        if !send_init_reports(device.as_ref(), digitizer, &config.name) {
                            log::debug!(target: "Detect", "Initialization failed for {}, skipping \
                                device", config.name);
                            continue;
                        }

                        if disambiguate {
                            let strings_confirmed =
                                digitizer.device_strings.as_ref().is_none_or(|ds| {
                                    confirm_device_strings(device.as_ref(), ds, &config.name)
                                });

                            let length_confirmed = strings_confirmed
                                && digitizer.input_report_length.is_none_or(|expected_len| {
                                    confirm_report_length(
                                        backend,
                                        &device_info,
                                        device.as_ref(),
                                        expected_len,
                                        &config.name,
                                    )
                                });

                            if !strings_confirmed || !length_confirmed {
                                log::debug!(
                                    target: "Detect",
                                    "{} | Candidate not confirmed (DeviceStrings: {}, \
                                        InputReportLength: {}), trying next candidate",
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
                            "TABLET DETAILS: Name: '{}' | Width: {}mm, Height: {}mm | MaxX: {}, \
                                MaxY: {} | MaxPressure: {} | Parser: {}",
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
                        if e == HidError::PermissionDenied {
                            log::error!(
                                target: "Detect",
                                "PERMISSION DENIED: Could not open tablet '{}' at path {:?} \
                                (Interface {}) due to insufficient privileges.\n\
                                \n\
                                TO FIX THIS (Linux):\n\
                                1. Create a custom udev rules file:\n\
                                   echo 'KERNEL==\"hidraw*\", SUBSYSTEM==\"hidraw\", \
                                   ATTRS{{idVendor}}==\"{:04x}\", ATTRS{{idProduct}}==\"{:04x}\", \
                                   MODE=\"0666\"' \
                                   | sudo tee /etc/udev/rules.d/99-next-tablet.rules\n\
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
                            log::debug!(target: "Detect", "Could not open {} interface {}: \
                                {e}", config.name, interface);
                        }
                    }
                }
            }

            // No candidate's report length could be confirmed (e.g. the
            // tablet stays silent until the pen approaches). Fall back to
            // the first candidate that opened and initialized successfully
            // rather than failing detection outright.
            if let Some((config, digitizer)) = fallback
                && let Ok(device) = backend.open(&device_info, OpenOptions::default())
                && send_init_reports(device.as_ref(), digitizer, &config.name)
            {
                log::warn!(
                    target: "Detect",
                    "{:04x}:{:04x} | No candidate confirmed via report length, falling back to \
                        '{}'",
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
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntd_tablets::config::{
        Attributes, DigitizerIdentifier, DigitizerSpecs, PenSpecs, Specifications,
        TabletConfiguration,
    };
    use ntd_test_support::{FakeHidBackend, FakeHidHandle, fake_tablet_info};

    fn make_test_config(attr: Option<Attributes>) -> TabletConfiguration {
        TabletConfiguration {
            name: "Test Tablet".to_string(),
            specifications: Specifications {
                auxiliary_buttons: None,
                digitizer: DigitizerSpecs {
                    width: 100.0,
                    height: 100.0,
                    max_x: 1000.0,
                    max_y: 1000.0,
                },
                pen: PenSpecs {
                    max_pressure: 8191,
                    button_count: Some(2),
                },
            },
            digitizer_identifiers: vec![],
            attributes: attr,
        }
    }

    fn make_test_digitizer(attr: Option<Attributes>) -> DigitizerIdentifier {
        DigitizerIdentifier {
            vendor_id: 0x1234,
            product_id: 0x5678,
            input_report_length: Some(10),
            output_report_length: None,
            device_strings: None,
            report_parser: "FallbackParser".to_string(),
            output_init_report: None,
            feature_init_report: None,
            attributes: attr,
        }
    }

    fn make_attributes(interface_val: serde_json::Value) -> Attributes {
        Attributes {
            libinputoverride: None,
            interface: Some(interface_val),
        }
    }

    #[test]
    fn test_get_expected_interface_digitizer_number_and_string() {
        let config = make_test_config(None);

        // Number value
        let dig_num = make_test_digitizer(Some(make_attributes(serde_json::json!(2))));
        assert_eq!(get_expected_interface(&config, &dig_num), Some(2));

        // String value (e.g. "3")
        let dig_str = make_test_digitizer(Some(make_attributes(serde_json::json!("3"))));
        assert_eq!(get_expected_interface(&config, &dig_str), Some(3));
    }

    #[test]
    fn test_get_expected_interface_fallback_to_config() {
        let config = make_test_config(Some(make_attributes(serde_json::json!(1))));
        let digitizer_no_attr = make_test_digitizer(None);

        assert_eq!(get_expected_interface(&config, &digitizer_no_attr), Some(1));
    }

    #[test]
    fn test_get_expected_interface_digitizer_precedence_over_config() {
        // Digitizer specifies interface 2, Config specifies interface 1
        let config = make_test_config(Some(make_attributes(serde_json::json!(1))));
        let digitizer = make_test_digitizer(Some(make_attributes(serde_json::json!(2))));

        assert_eq!(get_expected_interface(&config, &digitizer), Some(2));
    }

    #[test]
    fn test_get_expected_interface_invalid_and_missing() {
        let config = make_test_config(None);

        // Non-numeric string
        let dig_str_err =
            make_test_digitizer(Some(make_attributes(serde_json::json!("invalid_num"))));
        assert_eq!(get_expected_interface(&config, &dig_str_err), None);

        // JSON Object
        let dig_obj = make_test_digitizer(Some(make_attributes(serde_json::json!({"foo": "bar"}))));
        assert_eq!(get_expected_interface(&config, &dig_obj), None);

        // JSON Boolean
        let dig_bool = make_test_digitizer(Some(make_attributes(serde_json::json!(true))));
        assert_eq!(get_expected_interface(&config, &dig_bool), None);

        // Both None
        let dig_none = make_test_digitizer(None);
        assert_eq!(get_expected_interface(&config, &dig_none), None);
    }

    #[test]
    fn test_expected_wire_report_len() {
        #[cfg(target_os = "linux")]
        assert_eq!(expected_wire_report_len(10), 10);

        #[cfg(not(target_os = "linux"))]
        assert_eq!(expected_wire_report_len(10), 11);
    }

    // The XP-Pen Star G640 and its second variant share 28bd:0914 and differ by the length of
    // their input reports (14 and 12).
    const G640: (u16, u16) = (0x28bd, 0x0914);

    fn plugged_g640(report_length: Option<usize>) -> (FakeHidBackend, FakeHidHandle) {
        let backend = FakeHidBackend::new();
        let tablet = backend.plug(fake_tablet_info(G640.0, G640.1));
        tablet.set_input_report_length(report_length);
        (backend, tablet)
    }

    #[test]
    fn the_candidate_whose_report_length_matches_the_device_is_chosen() {
        for (length, name) in [
            (12, "XP-Pen Star G640 (Variant 2)"),
            (14, "XP-Pen Star G640"),
        ] {
            let (mut backend, tablet) = plugged_g640(Some(length));
            let (_, driver, vid, pid) = detect_tablet(&mut backend).unwrap();
            assert_eq!((vid, pid), G640);
            assert_eq!(driver.get_name(), name);
            // The tablet was woken up with its init report.
            assert!(!tablet.written().is_empty());
        }
    }

    #[test]
    fn without_a_confirmed_length_the_first_candidate_that_initialised_is_used() {
        // No static length and nothing to read: neither candidate can be confirmed.
        let (mut backend, _tablet) = plugged_g640(None);
        let (_, driver, ..) = detect_tablet(&mut backend).unwrap();
        assert!(driver.get_name().starts_with("XP-Pen Star G640"));
    }

    #[test]
    fn a_device_that_no_definition_names_is_ignored_and_never_opened() {
        let mut backend = FakeHidBackend::new();
        let _unknown = backend.plug(fake_tablet_info(0xdead, 0xbeef));
        assert!(detect_tablet(&mut backend).is_none());
        assert!(backend.opens().is_empty());
    }

    #[test]
    fn a_refused_open_gives_no_tablet() {
        let (mut backend, _tablet) = plugged_g640(Some(12));
        backend.deny_permission(true);
        assert!(detect_tablet(&mut backend).is_none());
        assert!(!backend.opens().is_empty());
    }

    #[test]
    fn nothing_plugged_gives_no_tablet() {
        assert!(detect_tablet(&mut FakeHidBackend::new()).is_none());
    }
}
