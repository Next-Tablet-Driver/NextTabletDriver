use crate::drivers::NextTabletDriver;
use crate::drivers::config::{DigitizerIdentifier, TabletConfiguration};
use crate::drivers::config_loader::INDEXED_CONFIGS;
use crate::drivers::generic::GenericNextTabletDriver;
use hidapi::{HidApi, HidDevice};
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

use std::collections::HashSet;
use std::sync::Mutex;

static WARNED_UNSUPPORTED: Mutex<Option<HashSet<(u16, u16)>>> = Mutex::new(None);

#[must_use]
pub fn detect_tablet(api: &HidApi) -> Option<(HidDevice, Box<dyn NextTabletDriver>, u16, u16)> {
    use base64::{Engine as _, engine::general_purpose};
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
            let is_tablet_brand = matches!(vid, 0x056a | 0x256c | 0x28bd | 0x5543 | 0x0b57)
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
                        let mut init_success = true;

                        let init_start = Instant::now();

                        // Feature Reports
                        if let Some(reports) = &digitizer.feature_init_report {
                            for report_str in reports {
                                match general_purpose::STANDARD.decode(report_str) {
                                    Ok(data) => {
                                        log::trace!(target: "Detect", "Sending Feature Report: {data:02x?}");
                                        if let Err(e) = device.send_feature_report(&data) {
                                            log::error!(target: "Detect", "{} | Init Error (Feature Report): {e}", config.name);
                                            init_success = false;
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        log::error!(target: "Detect", "{} | Base64 Decode Error (Feature): {e}", config.name);
                                        init_success = false;
                                        break;
                                    }
                                }
                            }
                        }

                        // Output Reports
                        if init_success && let Some(reports) = &digitizer.output_init_report {
                            for report_str in reports {
                                match general_purpose::STANDARD.decode(report_str) {
                                    Ok(data) => {
                                        log::trace!(target: "Detect", "Sending Output Report: {data:02x?}");
                                        if let Err(e) = device.write(&data) {
                                            log::error!(target: "Detect", "{} | Init Error (Output Report): {e}", config.name);
                                            init_success = false;
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        log::error!(target: "Detect", "{} | Base64 Decode Error (Output): {e}", config.name);
                                        init_success = false;
                                        break;
                                    }
                                }
                            }
                        }

                        if !init_success {
                            log::warn!(target: "Detect", "Initialization failed for {}, skipping device", config.name);
                            continue;
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
        }
    }
    None
}
