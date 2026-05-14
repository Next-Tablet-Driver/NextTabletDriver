use crate::drivers::config_loader::INDEXED_CONFIGS;
use crate::drivers::generic::GenericNextTabletDriver;
use crate::drivers::NextTabletDriver;
use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};

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

        if let Some(matches) = index.get(&(vid, pid)) {
            for (config, digitizer) in matches {
                let interface = device_info.interface_number();
                let path = device_info.path();

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
                        use base64::{Engine as _, engine::general_purpose};

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
                                vid,
                                pid,
                            )),
                            vid,
                            pid,
                        ));
                    }
                    Err(e) => {
                        log::debug!(target: "Detect", "Could not open {} interface {}: {e}", config.name, interface);
                    }
                }
            }
        }
    }
    None
}

