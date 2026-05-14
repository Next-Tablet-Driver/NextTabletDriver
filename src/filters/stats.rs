//! # Speed Statistics Filter
//!
//! A passive "filter" that observes the coordinate stream without mutating it.
//! It calculates the physical hand speed of the user (e.g., in mm/s or km/h)
//! and broadcasts this data via a dedicated WebSocket server for streaming overlays.

use crate::core::config::models::{MappingConfig, SpeedUnit};
use crate::engine::state::{LockResultExt, SharedState};
use crate::filters::Filter;
use crossbeam_channel::{Sender, unbounded};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use tungstenite::accept;

/// Analyzes coordinate deltas over time to calculate physical hand speed.
pub struct SpeedStatsFilter {
    last_pos: Option<(f32, f32)>,
    last_time: Instant,
    tx: Option<Sender<(f32, f32)>>,
    current_config: Option<(String, u16)>,
    shared: Arc<SharedState>,
}

impl SpeedStatsFilter {
    pub fn new(shared: Arc<SharedState>) -> Self {
        Self {
            last_pos: None,
            last_time: Instant::now(),
            tx: None,
            current_config: None,
            shared,
        }
    }

    fn ensure_server_running(&mut self, ip: &str, port: u16) {
        if let Some((current_ip, current_port)) = &self.current_config
            && current_ip == ip
            && *current_port == port
        {
            return;
        }

        // Restart or Start server
        log::info!(target: "Stats", "Starting WebSocket stats server on {ip}:{port}");
        let (tx, rx) = unbounded::<(f32, f32)>();
        self.tx = Some(tx);
        self.current_config = Some((ip.to_string(), port));

        let addr = format!("{ip}:{port}");
        thread::spawn(move || {
            let listener = match TcpListener::bind(&addr) {
                Ok(l) => l,
                Err(e) => {
                    log::error!(target: "Stats", "Failed to bind WebSocket stats server: {e}");
                    return;
                }
            };

            let clients = Arc::new(Mutex::new(Vec::new()));

            // Broadcast thread
            let clients_clone = Arc::clone(&clients);
            thread::spawn(move || {
                while let Ok((speed, total_dist)) = rx.recv() {
                    let mut clients = clients_clone.lock().ignore_poison();
                    let msg = serde_json::json!({
                        "handspeed": speed,
                        "total_distance": total_dist,
                        "timestamp": Instant::now().elapsed().as_millis()
                    })
                    .to_string();

                    clients.retain_mut(
                        |client: &mut tungstenite::WebSocket<std::net::TcpStream>| {
                            client
                                .send(tungstenite::Message::Text(msg.clone().into()))
                                .is_ok()
                        },
                    );
                }
            });

            // Accept thread
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => match accept(s) {
                        Ok(ws) => {
                            let mut clients = clients.lock().ignore_poison();
                            clients.push(ws);
                            log::debug!(target: "Stats", "New WebSocket client connected");
                        }
                        Err(e) => {
                            log::error!(target: "Stats", "WebSocket accept error: {e}");
                        }
                    },
                    Err(e) => {
                        log::error!(target: "Stats", "TCP accept error: {e}");
                    }
                }
            }
        });
    }
}

impl Filter for SpeedStatsFilter {
    fn name(&self) -> &'static str {
        "HandSpeed WebSocket"
    }

    fn process(&mut self, u: f32, v: f32, config: &MappingConfig) -> (f32, f32) {
        let conf = &config.speed_stats;

        // We always update the local server if enabled
        if conf.enabled {
            self.ensure_server_running(&conf.ip, conf.port);
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32();

        // Convert normalized to physical mm
        let curr_x_mm = u * config.active_area.w;
        let curr_y_mm = v * config.active_area.h;

        if let Some((last_x_mm, last_y_mm)) = self.last_pos
            && dt > 0.0001
        {
            // Avoid division by zero
            let dx = curr_x_mm - last_x_mm;
            let dy = curr_y_mm - last_y_mm;
            let distance_mm = (dx * dx + dy * dy).sqrt();

            let mut speed = distance_mm / dt; // mm/s

            // Convert to requested unit
            speed = match conf.unit {
                SpeedUnit::MillimetersPerSecond => speed,
                SpeedUnit::MetersPerSecond => speed / 1000.0,
                SpeedUnit::KilometersPerHour => (speed / 1000.0) * 3.6,
                SpeedUnit::MilesPerHour => (speed / 1000.0) * 2.23694,
            };

            // Update shared stats for UI
            let mut current_total_dist = 0.0;
            if let Ok(mut stats) = self.shared.stats.write() {
                stats.handspeed = speed;
                stats.total_distance_mm += distance_mm;
                current_total_dist = stats.total_distance_mm;
            }

            if let Some(tx) = &self.tx {
                let _ = tx.try_send((speed, current_total_dist));
            }
        }

        self.last_pos = Some((curr_x_mm, curr_y_mm));
        self.last_time = now;

        (u, v)
    }

    fn update_config(&mut self, config: &MappingConfig) {
        let conf = &config.speed_stats;
        if conf.enabled {
            self.ensure_server_running(&conf.ip, conf.port);
        }
    }

    fn reset(&mut self) {
        self.last_pos = None;
        self.last_time = Instant::now();
    }
}
