//! # Tablet Drivers and Parsing
//!
//! This module provides the infrastructure for hardware abstraction. It handles:
//! 1.  **Detection**: Identifying supported HID devices on the USB bus.
//! 2.  **Initialization**: Sending vendor-specific "magic" packets to enable digitizer mode.
//! 3.  **Parsing**: Converting raw byte arrays from various protocols into a unified format.
//!
//! The system is designed to be extensible; adding support for a new tablet involves
//! adding a JSON configuration file to the `tablets/` directory.

use hidapi::{HidApi, HidDevice};
use include_dir::{Dir, DirEntry, include_dir};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

pub mod config;
pub mod generic;
pub mod parsers;

use config::TabletConfiguration;
use generic::GenericNextTabletDriver;

/// Standardized representation of pen input.
///
/// This structure is the common language used by the engine to process input
/// regardless of the physical tablet hardware being used.
#[derive(Debug, Clone, Default)]
pub struct TabletData {
    /// Identifies the current tool (e.g., "Pen", "Eraser", "Touch").
    pub status: String,
    /// Raw X coordinate from the tablet sensor. Range depends on hardware resolution.
    pub x: u16,
    /// Raw Y coordinate from the tablet sensor. Range depends on hardware resolution.
    pub y: u16,
    /// Absolute pressure applied to the nib. Normalized by the driver to a
    /// standard range (often 0 to 8191).
    pub pressure: u16,
    /// Horizontal pen tilt in degrees (if supported by hardware).
    pub tilt_x: i8,
    /// Vertical pen tilt in degrees (if supported by hardware).
    pub tilt_y: i8,
    /// Bitmask of pressed pen buttons (e.g., side buttons).
    pub buttons: u8,
    /// Boolean indicating if the physical eraser end of the pen is being used.
    pub eraser: bool,
    /// Proximity of the pen to the tablet surface in vendor-specific units.
    pub hover_distance: u8,
    /// Raw hexadecimal string of the USB packet (useful for debugging).
    pub raw_data: String,
    /// Connection status of the device.
    pub is_connected: bool,
    /// Timestamp when the packet was received by the driver.
    pub receive_time: Option<Instant>,
    /// Time taken to parse this specific packet.
    pub parser_time: Duration,
}

/// Statistics collected during a driver session.
#[derive(Clone, Copy, Debug)]
pub struct DriverStats {
    /// Calculated hand speed in millimeters per second.
    pub handspeed: f32,
    /// Aggregate distance traveled by the pen tip.
    pub total_distance_mm: f32,
    /// Last recorded time to read from the HID interface (ms).
    pub hid_read_ms: f32,
    pub min_hid_read_ms: f32,
    pub max_hid_read_ms: f32,
    pub avg_hid_read_ms: f32,
    /// Last recorded time to parse the packet (ms).
    pub parser_ms: f32,
    pub min_parser_ms: f32,
    pub max_parser_ms: f32,
    pub avg_parser_ms: f32,
    /// Total number of packets processed since start.
    pub total_packets: u64,
}

impl DriverStats {
    /// Resets all statistics to their default values.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Resets only the latency-related statistics.
    pub fn reset_latency(&mut self) {
        self.min_hid_read_ms = f32::MAX;
        self.max_hid_read_ms = 0.0;
        self.avg_hid_read_ms = 0.0;
        self.min_parser_ms = f32::MAX;
        self.max_parser_ms = 0.0;
        self.avg_parser_ms = 0.0;
    }

    /// Resets the accumulated distance.
    pub const fn reset_distance(&mut self) {
        self.total_distance_mm = 0.0;
    }

    /// Formats the total distance into a human-readable string and unit.
    #[must_use]
    pub fn format_distance(&self) -> (String, &'static str) {
        let dist = self.total_distance_mm;
        if dist < 1000.0 {
            (format!("{dist:.1}"), "mm")
        } else if dist < 1000000.0 {
            (format!("{:.3}", dist / 1000.0), "m")
        } else {
            (format!("{:.3}", dist / 1000000.0), "km")
        }
    }
}

impl Default for DriverStats {
    fn default() -> Self {
        Self {
            handspeed: 0.0,
            total_distance_mm: 0.0,
            hid_read_ms: 0.0,
            min_hid_read_ms: f32::MAX,
            max_hid_read_ms: 0.0,
            avg_hid_read_ms: 0.0,
            parser_ms: 0.0,
            min_parser_ms: f32::MAX,
            max_parser_ms: 0.0,
            avg_parser_ms: 0.0,
            total_packets: 0,
        }
    }
}

/// The trait that all tablet-specific driver implementations must satisfy.
///
/// It provides the interface for the Engine to query hardware limits and
/// decode incoming USB data.
pub trait NextTabletDriver {
    /// Returns the marketing name of the tablet.
    fn get_name(&self) -> &str;
    /// Returns hardware resolution and max pressure: `(MaxX, MaxY, MaxPressure)`.
    fn get_specs(&self) -> (f32, f32, f32);
    /// Returns physical tablet size in millimeters: `(Width, Height)`.
    fn get_physical_specs(&self) -> (f32, f32);
    /// Returns the USB identity of the device: `(VendorID, ProductID)`.
    fn get_vid_pid(&self) -> (u16, u16);
    /// Attempts to parse a raw USB packet into standard [`TabletData`].
    /// Returns `None` if the packet is malformed or empty.
    fn parse(&self, data: &[u8]) -> Option<TabletData>;
}

static TABLET_CONFIGS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/tablets");

static LOADED_CONFIGS: std::sync::LazyLock<Vec<TabletConfiguration>> =
    std::sync::LazyLock::new(load_configurations);

fn load_configurations() -> Vec<TabletConfiguration> {
    let global_start = Instant::now();
    let mut configs = Vec::new();
    let mut loaded_names = HashSet::new();

    let local_dir = Path::new("tablets");
    if local_dir.exists() {
        let disk_start = Instant::now();
        load_from_disk_recursive(local_dir, &mut configs, &mut loaded_names);
        log::debug!(
            target: "Driver",
            "Loaded {} configs from disk in {:.2?}",
            configs.len(),
            disk_start.elapsed()
        );
    }

    let embedded_start = Instant::now();
    let prev_len = configs.len();
    load_embedded_recursive(&TABLET_CONFIGS_DIR, &mut configs, &mut loaded_names);
    log::debug!(
        target: "Driver",
        "Loaded {} configs from embedded in {:.2?}",
        configs.len() - prev_len,
        embedded_start.elapsed()
    );

    log::info!(
        target: "Driver",
        "Total {} tablet configurations loaded in {:.2?}",
        configs.len(),
        global_start.elapsed()
    );
    configs
}

fn load_embedded_recursive(
    dir: &Dir,
    configs: &mut Vec<TabletConfiguration>,
    names: &mut HashSet<String>,
) {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(sub_dir) => {
                load_embedded_recursive(sub_dir, configs, names);
            }
            DirEntry::File(file) => {
                if file.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    match file.contents_utf8() {
                        Some(content_str) => {
                            match serde_json::from_str::<TabletConfiguration>(content_str) {
                                Ok(config) => {
                                    if !names.contains(&config.name) {
                                        configs.push(config);
                                    }
                                }
                                Err(e) => {
                                    log::error!(target: "Driver", "Failed to parse embedded config {:?}: {e}", file.path());
                                }
                            }
                        }
                        None => {
                            log::warn!(target: "Driver", "Embedded config file {:?} is not valid UTF-8", file.path());
                        }
                    }
                }
            }
        }
    }
}

fn load_from_disk_recursive(
    path: &Path,
    configs: &mut Vec<TabletConfiguration>,
    names: &mut HashSet<String>,
) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                load_from_disk_recursive(&p, configs, names);
            } else if p.extension().and_then(|s| s.to_str()) == Some("json") {
                match fs::read_to_string(&p) {
                    Ok(content) => match serde_json::from_str::<TabletConfiguration>(&content) {
                        Ok(config) => {
                            if !names.contains(&config.name) {
                                names.insert(config.name.clone());
                                configs.push(config);
                            }
                        }
                        Err(e) => {
                            log::error!(target: "Driver", "Failed to parse disk config {p:?}: {e}");
                        }
                    },
                    Err(e) => {
                        log::error!(target: "Driver", "Failed to read disk config {p:?}: {e}");
                    }
                }
            }
        }
    }
}

#[must_use]
pub fn detect_tablet(api: &HidApi) -> Option<(HidDevice, Box<dyn NextTabletDriver>, u16, u16)> {
    let global_start = Instant::now();
    let enum_start = Instant::now();
    let devices: Vec<_> = api.device_list().collect();
    let enum_duration = enum_start.elapsed();

    if enum_duration > Duration::from_millis(500) {
        log::warn!(target: "Detect", "HID Enumeration SLOW: {enum_duration:.2?}");
    }

    log::debug!(
        target: "Detect",
        "Starting scan of {} HID devices...",
        devices.len()
    );

    let configs = &*LOADED_CONFIGS;

    for config in configs {
        for digitizer in &config.digitizer_identifiers {
            for device_info in &devices {
                if device_info.vendor_id() == digitizer.vendor_id
                    && device_info.product_id() == digitizer.product_id
                {
                    let interface = device_info.interface_number();
                    let path = device_info.path();

                    log::debug!(
                        target: "Detect",
                        "Found candidate for {}: {:04x}:{:04x} (Interface {}, Path: {:?})",
                        config.name,
                        digitizer.vendor_id,
                        digitizer.product_id,
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
                                digitizer.vendor_id,
                                digitizer.product_id,
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
                                    digitizer.vendor_id,
                                    digitizer.product_id,
                                )),
                                digitizer.vendor_id,
                                digitizer.product_id,
                            ));
                        }
                        Err(e) => {
                            log::debug!(target: "Detect", "Could not open {} interface {}: {e}", config.name, interface);
                        }
                    }
                }
            }
        }
    }
    None
}
