//! # Speed Statistics Filter
//!
//! A passive "filter" that observes the coordinate stream without mutating it.
//! It calculates the physical hand speed of the user (e.g., in mm/s or km/h)
//! and broadcasts this data via a dedicated WebSocket server for streaming overlays.

use crate::core::config::models::{MappingConfig, SpeedUnit};
use crate::engine::state::SharedState;
use crate::filters::Filter;
use crate::filters::stats_server::StatsServer;
use std::sync::Arc;
use std::time::Instant;

/// Analyzes coordinate deltas over time to calculate physical hand speed.
pub struct SpeedStatsFilter {
    last_pos: Option<(f32, f32)>,
    last_time: Instant,
    server: Option<StatsServer>,
    current_config: Option<(String, u16)>,
    shared: Arc<SharedState>,
}

impl SpeedStatsFilter {
    pub fn new(shared: Arc<SharedState>) -> Self {
        Self {
            last_pos: None,
            last_time: Instant::now(),
            server: None,
            current_config: None,
            shared,
        }
    }

    fn update_server(&mut self, enabled: bool, ip: &str, port: u16) {
        if !enabled {
            if self.server.is_some() {
                log::info!(target: "Stats", "Stopping WebSocket stats server");
                self.server = None;
                self.current_config = None;
            }
            return;
        }

        if let Some((current_ip, current_port)) = &self.current_config
            && current_ip == ip
            && *current_port == port
        {
            return;
        }

        // Configuration changed or server not started
        log::info!(target: "Stats", "Configuring WebSocket stats server on {ip}:{port}");
        self.server = None; // Drop old server (triggering shutdown)

        match StatsServer::start(ip, port) {
            Ok(server) => {
                self.server = Some(server);
                self.current_config = Some((ip.to_string(), port));
            }
            Err(e) => {
                log::error!(target: "Stats", "Failed to start stats server: {e}");
                self.current_config = None;
            }
        }
    }
}

impl Filter for SpeedStatsFilter {
    fn name(&self) -> &'static str {
        "HandSpeed WebSocket"
    }

    fn process(&mut self, u: f32, v: f32, config: &MappingConfig) -> (f32, f32) {
        let conf = &config.speed_stats;

        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32();

        // Convert normalized to physical mm
        let curr_x_mm = u * config.active_area.w;
        let curr_y_mm = v * config.active_area.h;

        if let Some((last_x_mm, last_y_mm)) = self.last_pos
            && dt > 0.0001
        {
            let dx = curr_x_mm - last_x_mm;
            let dy = curr_y_mm - last_y_mm;
            let distance_mm = dx.hypot(dy);

            let mut speed = distance_mm / dt; // mm/s

            speed = match conf.unit {
                SpeedUnit::MillimetersPerSecond => speed,
                SpeedUnit::MetersPerSecond => speed / 1000.0,
                SpeedUnit::KilometersPerHour => (speed / 1000.0) * 3.6,
                SpeedUnit::MilesPerHour => (speed / 1000.0) * 2.23694,
            };

            let current_total_dist = self.shared.stats.write().map_or(0.0, |mut stats| {
                stats.handspeed = speed;
                stats.total_distance_mm += distance_mm;
                stats.total_distance_mm
            });

            if let Some(server) = &self.server {
                server.send_stats(speed, current_total_dist);
            }
        }

        self.last_pos = Some((curr_x_mm, curr_y_mm));
        self.last_time = now;

        (u, v)
    }

    fn update_config(&mut self, config: &MappingConfig) {
        let conf = &config.speed_stats;
        self.update_server(conf.enabled, &conf.ip, conf.port);
    }

    fn reset(&mut self) {
        self.last_pos = None;
        self.last_time = Instant::now();
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;
    use crate::core::config::models::MappingConfig;
    use crate::engine::state::SharedState;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_speed_calculation() {
        let shared = Arc::new(SharedState::test_default());
        let mut filter = SpeedStatsFilter::new(shared.clone());
        let mut config = MappingConfig::default();
        config.active_area.w = 100.0;
        config.active_area.h = 100.0;
        config.speed_stats.unit = SpeedUnit::MillimetersPerSecond;

        // First point
        filter.process(0.0, 0.0, &config);

        // Move 10mm in 0.1s => 100mm/s
        filter.last_time = Instant::now()
            .checked_sub(Duration::from_millis(100))
            .unwrap();
        filter.process(0.1, 0.0, &config);

        let speed = shared.stats.read().unwrap().handspeed;
        // Allow some float tolerance
        assert!((speed - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_speed_unit_conversion() {
        let shared = Arc::new(SharedState::test_default());
        let mut filter = SpeedStatsFilter::new(shared.clone());
        let mut config = MappingConfig::default();
        config.active_area.w = 1000.0;
        config.active_area.h = 1000.0;
        config.speed_stats.unit = SpeedUnit::MetersPerSecond;

        // First point
        filter.process(0.0, 0.0, &config);

        // Move 1000mm (1m) in 1s => 1m/s
        filter.last_time = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        filter.process(1.0, 0.0, &config);

        let speed = shared.stats.read().unwrap().handspeed;
        assert!((speed - 1.0).abs() < 0.1);
    }
}
