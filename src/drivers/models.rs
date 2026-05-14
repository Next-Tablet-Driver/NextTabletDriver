use std::time::{Duration, Instant};

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
    pub const fn reset_latency(&mut self) {
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
