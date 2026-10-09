use std::time::{Duration, Instant};

/// The operational status of a tablet input tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TabletStatus {
    #[default]
    Disconnected,
    OutOfRange,
    Hover,
    Contact,
    Active,
    Eraser,
    Pen,
    Touch,
    Aux,
    Rotation,
    Tool,
    Mouse,
}

impl TabletStatus {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Disconnected => "Disconnected",
            Self::OutOfRange => "Out of Range",
            Self::Hover => "Hover",
            Self::Contact => "Contact",
            Self::Active => "Active",
            Self::Eraser => "Eraser",
            Self::Pen => "Pen",
            Self::Touch => "Touch",
            Self::Aux => "Aux",
            Self::Rotation => "Rotation",
            Self::Tool => "Tool",
            Self::Mouse => "Mouse",
        }
    }
}

impl std::fmt::Display for TabletStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Standardized representation of pen input.
///
/// This structure is the common language used by the engine to process input
/// regardless of the physical tablet hardware being used.
#[derive(Debug, Clone, Default)]
pub struct TabletData {
    /// Identifies the current tool status.
    pub status: TabletStatus,
    /// Raw X coordinate from the tablet sensor.
    pub x: u32,
    /// Raw Y coordinate from the tablet sensor.
    pub y: u32,
    /// Absolute pressure applied to the nib.
    pub pressure: u16,
    /// Horizontal pen tilt in degrees.
    pub tilt_x: i8,
    /// Vertical pen tilt in degrees.
    pub tilt_y: i8,
    /// Bitmask of pressed pen buttons.
    pub buttons: u8,
    /// Boolean indicating if the physical eraser end is being used.
    pub eraser: bool,
    /// Proximity of the pen to the surface.
    pub hover_distance: u8,
    /// Raw bytes of the USB packet for debugging.
    pub raw_data: [u8; 32],
    /// Length of the valid data in `raw_data`.
    pub raw_len: u8,
    /// Connection status of the device.
    pub is_connected: bool,
    /// Timestamp when the packet was received.
    pub receive_time: Option<Instant>,
    /// Time taken to parse this specific packet.
    pub parser_time: Duration,
}

impl TabletData {
    /// Sets the raw data bytes, truncating if necessary.
    pub fn set_raw(&mut self, data: &[u8]) {
        let len = data.len().min(32);
        if let (Some(dest), Some(src)) = (self.raw_data.get_mut(..len), data.get(..len)) {
            dest.copy_from_slice(src);
        }
        self.raw_len = len as u8;
    }

    /// Formats the raw data as a hexadecimal string for debugging.
    #[must_use]
    pub fn raw_hex(&self) -> String {
        self.raw_data
            .get(..self.raw_len as usize)
            .unwrap_or(&[])
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
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
    /// Last recorded time to inject the processed frame into the OS (ms).
    pub inject_ms: f32,
    pub min_inject_ms: f32,
    pub max_inject_ms: f32,
    pub avg_inject_ms: f32,
    /// Total number of packets processed since start.
    pub total_packets: u64,
}

impl DriverStats {
    /// Resets all statistics to their default values.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Resets only the latency-related statistics.
    pub const fn reset_latency(&mut self) {
        self.min_hid_read_ms = f32::MAX;
        self.max_hid_read_ms = 0.0;
        self.avg_hid_read_ms = 0.0;
        self.min_parser_ms = f32::MAX;
        self.max_parser_ms = 0.0;
        self.avg_parser_ms = 0.0;
        self.min_inject_ms = f32::MAX;
        self.max_inject_ms = 0.0;
        self.avg_inject_ms = 0.0;
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
        } else if dist < 1_000_000.0 {
            (format!("{:.3}", dist / 1_000.0), "m")
        } else {
            (format!("{:.3}", dist / 1_000_000.0), "km")
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
            inject_ms: 0.0,
            min_inject_ms: f32::MAX,
            max_inject_ms: 0.0,
            avg_inject_ms: 0.0,
            total_packets: 0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_stats_format_distance() {
        // Under 1 meter (millimeters)
        let mut stats = DriverStats {
            total_distance_mm: 750.4,
            ..Default::default()
        };
        let (val, unit) = stats.format_distance();
        assert_eq!(val, "750.4");
        assert_eq!(unit, "mm");

        // Between 1 meter and 1 kilometer (meters)
        stats.total_distance_mm = 1500.0;
        let (val_m, unit_m) = stats.format_distance();
        assert_eq!(val_m, "1.500");
        assert_eq!(unit_m, "m");

        // Over 1 kilometer (kilometers)
        stats.total_distance_mm = 2_500_000.0;
        let (val_km, unit_km) = stats.format_distance();
        assert_eq!(val_km, "2.500");
        assert_eq!(unit_km, "km");
    }

    #[test]
    fn test_driver_stats_reset_latency() {
        let mut stats = DriverStats {
            min_hid_read_ms: 0.25,
            max_hid_read_ms: 3.50,
            avg_hid_read_ms: 1.10,
            min_parser_ms: 0.05,
            max_parser_ms: 0.90,
            avg_parser_ms: 0.20,
            min_inject_ms: 0.10,
            max_inject_ms: 2.10,
            avg_inject_ms: 0.60,
            total_packets: 1000,
            total_distance_mm: 500.0,
            handspeed: 120.0,
            hid_read_ms: 1.0,
            parser_ms: 0.2,
            inject_ms: 0.5,
        };

        stats.reset_latency();

        // Latency statistics must be reset
        assert_eq!(stats.min_hid_read_ms, f32::MAX);
        assert_eq!(stats.max_hid_read_ms, 0.0);
        assert_eq!(stats.avg_hid_read_ms, 0.0);
        assert_eq!(stats.min_parser_ms, f32::MAX);
        assert_eq!(stats.max_parser_ms, 0.0);
        assert_eq!(stats.avg_parser_ms, 0.0);
        assert_eq!(stats.min_inject_ms, f32::MAX);
        assert_eq!(stats.max_inject_ms, 0.0);
        assert_eq!(stats.avg_inject_ms, 0.0);

        // Distance and total packets must not be modified by reset_latency
        assert_eq!(stats.total_distance_mm, 500.0);
        assert_eq!(stats.total_packets, 1000);
    }

    #[test]
    fn test_driver_stats_reset_distance() {
        let mut stats = DriverStats {
            total_distance_mm: 9876.5,
            ..Default::default()
        };

        stats.reset_distance();
        assert_eq!(stats.total_distance_mm, 0.0);
    }

    #[test]
    fn test_tablet_data_raw_buffer_and_hex() {
        let mut data = TabletData::default();
        let sample = [0xAA, 0xBB, 0x01, 0x02, 0xFF];
        data.set_raw(&sample);

        assert_eq!(data.raw_len, 5);
        assert_eq!(data.raw_hex(), "AA BB 01 02 FF");

        // Verify buffer truncation at 32 bytes
        let long_sample = [0x55; 48];
        data.set_raw(&long_sample);
        assert_eq!(data.raw_len, 32);
        assert_eq!(data.raw_data.len(), 32);
    }

    #[test]
    fn test_tablet_status_strings() {
        assert_eq!(TabletStatus::Contact.as_str(), "Contact");
        assert_eq!(TabletStatus::Hover.as_str(), "Hover");
        assert_eq!(TabletStatus::OutOfRange.as_str(), "Out of Range");
        assert_eq!(format!("{}", TabletStatus::Contact), "Contact");
    }

    #[test]
    fn every_status_has_a_display_name() {
        use TabletStatus::*;
        for (status, name) in [
            (Disconnected, "Disconnected"),
            (OutOfRange, "Out of Range"),
            (Hover, "Hover"),
            (Contact, "Contact"),
            (Active, "Active"),
            (Eraser, "Eraser"),
            (Pen, "Pen"),
            (Touch, "Touch"),
            (Aux, "Aux"),
            (Rotation, "Rotation"),
            (Tool, "Tool"),
            (Mouse, "Mouse"),
        ] {
            assert_eq!(status.as_str(), name);
        }
    }

    #[test]
    fn resetting_the_statistics_restores_every_default() {
        let mut stats = DriverStats {
            handspeed: 3.0,
            total_distance_mm: 99.0,
            total_packets: 5,
            max_hid_read_ms: 7.0,
            min_parser_ms: 0.1,
            ..DriverStats::default()
        };
        stats.reset();
        assert_eq!(stats.total_packets, 0);
        assert_eq!(stats.handspeed, 0.0);
        assert_eq!(stats.total_distance_mm, 0.0);
        assert_eq!(stats.max_hid_read_ms, 0.0);
        assert_eq!(stats.min_parser_ms, f32::MAX);
    }
}
