//! # Hand Speed Plugin
//!
//! **Creator**: nzbasic
//!
//! Tracks physical hand speed (in mm/s, km/h, m/s, or mph) and total distance traveled,
//! broadcasting real-time metrics over an embedded WebSocket server for stream overlays.

mod server;

use ntd_plugin_api::{
    NextTabletPlugin, PluginContext, PluginManifest, PluginPacket, PropertyDescriptor,
    PropertyKind, export_ntd_plugin,
};
use server::StatsServer;
use std::time::{Duration, Instant};

/// Minimum interval between WebSocket broadcast packets (cadence of ~60Hz).
const FLUSH_INTERVAL: Duration = Duration::from_millis(16);

/// Supported display units for hand speed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpeedUnit {
    #[default]
    MillimetersPerSecond = 0,
    MetersPerSecond = 1,
    KilometersPerHour = 2,
    MilesPerHour = 3,
}

impl SpeedUnit {
    #[must_use]
    pub const fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::MetersPerSecond,
            2 => Self::KilometersPerHour,
            3 => Self::MilesPerHour,
            _ => Self::MillimetersPerSecond,
        }
    }

    #[must_use]
    pub fn convert_from_mm_s(self, speed_mm_s: f32) -> f32 {
        match self {
            Self::MillimetersPerSecond => speed_mm_s,
            Self::MetersPerSecond => speed_mm_s / 1000.0_f32,
            Self::KilometersPerHour => (speed_mm_s / 1000.0_f32) * 3.6_f32,
            Self::MilesPerHour => (speed_mm_s / 1000.0_f32) * 2.236_94_f32,
        }
    }
}

/// Standalone plugin implementing Hand Speed and travel distance tracking.
pub struct HandSpeedPlugin {
    pub enabled: bool,
    pub ip: String,
    pub port: u16,
    pub unit: SpeedUnit,

    last_pos_mm: Option<(f32, f32)>,
    last_time: Instant,
    last_flush: Instant,
    smoothed_speed_mm_s: f32,
    total_distance_mm: f32,

    server: Option<StatsServer>,
    server_addr: Option<(String, u16)>,
}

impl Default for HandSpeedPlugin {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            enabled: true,
            ip: "127.0.0.1".to_string(),
            port: 50550,
            unit: SpeedUnit::MillimetersPerSecond,

            last_pos_mm: None,
            last_time: now,
            last_flush: now.checked_sub(Duration::from_secs(1)).unwrap_or(now),
            smoothed_speed_mm_s: 0.0,
            total_distance_mm: 0.0,

            server: None,
            server_addr: None,
        }
    }
}

impl HandSpeedPlugin {
    fn update_server(&mut self) {
        if !self.enabled {
            self.server = None;
            self.server_addr = None;
            return;
        }

        if let Some((ref cur_ip, cur_port)) = self.server_addr
            && cur_ip == &self.ip
            && cur_port == self.port
            && self.server.is_some()
        {
            return;
        }

        self.server = None;
        if let Ok(srv) = StatsServer::start(&self.ip, self.port) {
            self.server = Some(srv);
            self.server_addr = Some((self.ip.clone(), self.port));
        } else {
            self.server_addr = None;
        }
    }
}

impl NextTabletPlugin for HandSpeedPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "hand_speed".to_string(),
            name: "Hand Speed & Stats".to_string(),
            version: "1.0.0".to_string(),
            author: "nzbasic".to_string(),
            description: "Calculates real-time pen speed and cumulative travel distance with an embedded WebSocket server for stream overlays.".to_string(),
            icon: Some("SPEEDOMETER".to_string()),
            properties: vec![
                PropertyDescriptor {
                    id: "enabled".to_string(),
                    name: "WebSocket Server".to_string(),
                    tooltip: Some("Enables the background WebSocket broadcast server for streaming overlays.".to_string()),
                    kind: PropertyKind::Bool {
                        default: true,
                    },
                },
                PropertyDescriptor {
                    id: "ip".to_string(),
                    name: "Server IP".to_string(),
                    tooltip: Some("Bind IP address for the WebSocket server (e.g. 127.0.0.1).".to_string()),
                    kind: PropertyKind::String {
                        default: "127.0.0.1".to_string(),
                    },
                },
                PropertyDescriptor {
                    id: "port".to_string(),
                    name: "Port".to_string(),
                    tooltip: Some("TCP port number for incoming overlay WebSocket connections.".to_string()),
                    kind: PropertyKind::Int {
                        min: 1024,
                        max: 65535,
                        step: 1,
                        unit: "".to_string(),
                        default: 50550,
                    },
                },
                PropertyDescriptor {
                    id: "unit".to_string(),
                    name: "Speed Unit".to_string(),
                    tooltip: Some("Unit for speed reporting in the broadcast data.".to_string()),
                    kind: PropertyKind::Choice {
                        options: vec![
                            "Millimeters / second (mm/s)".to_string(),
                            "Meters / second (m/s)".to_string(),
                            "Kilometers / hour (km/h)".to_string(),
                            "Miles / hour (mph)".to_string(),
                        ],
                        default_index: 0,
                    },
                },
            ],
        }
    }

    fn process(&mut self, packet: &mut PluginPacket, context: &PluginContext) {
        let width_mm = context.active_area_width_mm.max(0.1_f32);
        let height_mm = context.active_area_height_mm.max(0.1_f32);

        let curr_x = packet.u * width_mm;
        let curr_y = packet.v * height_mm;

        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32();

        if let Some((last_x, last_y)) = self.last_pos_mm
            && dt > 0.0001_f32
            && dt < 1.0_f32
        {
            let dx = curr_x - last_x;
            let dy = curr_y - last_y;
            let distance_mm = dx.hypot(dy);

            let instant_speed = distance_mm / dt;
            self.smoothed_speed_mm_s =
                self.smoothed_speed_mm_s * 0.85_f32 + instant_speed * 0.15_f32;
            self.total_distance_mm += distance_mm;

            if now.duration_since(self.last_flush) >= FLUSH_INTERVAL {
                self.last_flush = now;
                self.update_server();

                let converted_speed = self.unit.convert_from_mm_s(self.smoothed_speed_mm_s);
                if let Some(ref srv) = self.server {
                    srv.send_stats(converted_speed, self.total_distance_mm);
                }
            }
        }

        self.last_pos_mm = Some((curr_x, curr_y));
        self.last_time = now;
        // Packet coordinates pass through completely unmodified
    }

    fn set_property(&mut self, key: &str, value_json: &str) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(value_json) {
            match key {
                "enabled" => {
                    if let Some(v) = val.as_bool() {
                        self.enabled = v;
                        self.update_server();
                    }
                }
                "ip" => {
                    if let Some(v) = val.as_str() {
                        self.ip = v.to_string();
                        self.update_server();
                    }
                }
                "port" => {
                    if let Some(v) = val.as_i64() {
                        self.port = (v as u16).clamp(1024, 65535);
                        self.update_server();
                    }
                }
                "unit" => {
                    if let Some(v) = val.as_u64() {
                        self.unit = SpeedUnit::from_index(v as usize);
                    }
                }
                _ => {}
            }
        }
    }

    fn reset(&mut self) {
        self.last_pos_mm = None;
        self.last_time = Instant::now();
    }
}

export_ntd_plugin!(HandSpeedPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_unit_conversions() {
        let speed_mm_s = 1000.0_f32;
        assert!(
            (SpeedUnit::MillimetersPerSecond.convert_from_mm_s(speed_mm_s) - 1000.0).abs() < 0.001
        );
        assert!((SpeedUnit::MetersPerSecond.convert_from_mm_s(speed_mm_s) - 1.0).abs() < 0.001);
        assert!((SpeedUnit::KilometersPerHour.convert_from_mm_s(speed_mm_s) - 3.6).abs() < 0.001);
        assert!((SpeedUnit::MilesPerHour.convert_from_mm_s(speed_mm_s) - 2.23694).abs() < 0.01);
    }

    #[test]
    fn test_hand_speed_passthrough() {
        let mut plugin = HandSpeedPlugin {
            enabled: false, // Don't bind port in unit test
            ..Default::default()
        };
        let mut packet = PluginPacket {
            u: 0.25,
            v: 0.75,
            ..Default::default()
        };
        let ctx = PluginContext::default();

        plugin.process(&mut packet, &ctx);
        assert!((packet.u - 0.25).abs() < f32::EPSILON);
        assert!((packet.v - 0.75).abs() < f32::EPSILON);
    }
}
