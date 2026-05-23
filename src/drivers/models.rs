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
    pub x: u16,
    /// Raw Y coordinate from the tablet sensor.
    pub y: u16,
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
            total_packets: 0,
        }
    }
}
