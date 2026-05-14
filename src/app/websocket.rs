//! # WebSocket Server
//!
//! This module provides an embedded WebSocket server that broadcasts real-time
//! tablet data (position, pressure, status) to external clients. This is primarily
//! designed for streamer overlays, custom UI integrations, or third-party plugins.

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tungstenite::protocol::WebSocket;
use tungstenite::{Message, accept};

use crate::engine::state::{LockRecoveryExt, SharedState};

/// The JSON payload broadcasted to all connected WebSocket clients.
///
/// Fields are dynamically omitted (via `skip_serializing_if = "Option::is_none"`)
/// depending on the user's privacy/performance settings in the Settings tab.
#[derive(Serialize)]
struct WsPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    x: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pressure: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_connected: Option<bool>,
}

/// Runs the embedded WebSocket server in a dedicated background thread.
///
/// # Behavior
/// 1. Reads the current configuration (`WebSocketConfig`) from `SharedState`.
/// 2. Manages the lifecycle of a `TcpListener` based on the user-configured port.
/// 3. Accepts incoming WebSocket handshakes and stores active connections.
/// 4. Reads the most recent `TabletData` and broadcasts it to all connected clients
///    at the user-defined polling rate (Hz).
///
/// # Networking
/// - Binds to `127.0.0.1` (localhost only) for security.
/// - Uses non-blocking sockets to allow for graceful client disconnection and reconnects.
pub fn websocket_loop(shared: &Arc<SharedState>) {
    let mut current_port = 0;
    let mut listener: Option<TcpListener> = None;
    let mut clients: HashMap<usize, WebSocket<std::net::TcpStream>> = HashMap::new();
    let mut next_client_id = 0;

    loop {
        if shared.shutdown_requested.load(Ordering::Relaxed) {
            log::info!(target: "WebSocket", "Shutdown requested, exiting WebSocket loop");
            break;
        }

        let frame_start = Instant::now();

        let (enabled, port, hz, send_coords, send_pressure, _send_tilt, send_status) = {
            let config = shared.config.read().unwrap_or_log("config");
            let ws = &config.websocket;
            let res = (
                ws.enabled,
                ws.port,
                ws.polling_rate_hz.max(1),
                ws.send_coordinates,
                ws.send_pressure,
                ws.send_tilt,
                ws.send_status,
            );
            drop(config);
            res
        };

        if !enabled {
            if listener.is_some() {
                log::info!(target: "WebSocket", "WebSocket Server disabled, shutting down...");
                clients.clear();
                listener = None;
            }
        } else if listener.is_none() || current_port != port {
            log::info!(target: "WebSocket", "Starting WebSocket Server on 127.0.0.1:{port}");
            clients.clear();

            match TcpListener::bind(format!("127.0.0.1:{port}")) {
                Ok(l) => match l.set_nonblocking(true) {
                    Ok(()) => {
                        listener = Some(l);
                        current_port = port;
                    }
                    Err(e) => {
                        log::error!(target: "WebSocket", "Failed to set WebSocket listener to non-blocking: {e}");
                        listener = None;
                    }
                },
                Err(e) => {
                    log::error!(target: "WebSocket", "Failed to bind to port {port}: {e}");
                    listener = None;
                }
            }
        }

        if let Some(l) = &listener {
            match l.accept() {
                Ok((stream, addr)) => {
                    if clients.len() >= 10 {
                        log::warn!(target: "WebSocket", "Max clients reached (10), rejecting {addr}");
                        continue;
                    }

                    log::info!(target: "WebSocket", "New connection from {addr}");
                    if stream.set_nonblocking(false).is_ok() {
                        match accept(stream) {
                            Ok(mut websocket) => {
                                if websocket.get_mut().set_nonblocking(true).is_ok() {
                                    clients.insert(next_client_id, websocket);
                                    next_client_id += 1;
                                }
                            }
                            Err(e) => {
                                log::warn!(target: "WebSocket", "Error during WebSocket handshake: {e}");
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    log::error!(target: "WebSocket", "Listener error: {e}");
                }
            }

            if !clients.is_empty() {
                let data = shared
                    .tablet_data
                    .read()
                    .map(|d| d.clone())
                    .unwrap_or_default();

                let payload = WsPayload {
                    x: if send_coords { Some(data.x) } else { None },
                    y: if send_coords { Some(data.y) } else { None },
                    pressure: if send_pressure {
                        Some(data.pressure)
                    } else {
                        None
                    },
                    status: if send_status {
                        Some(data.status.to_string())
                    } else {
                        None
                    },
                    is_connected: if send_status {
                        Some(data.is_connected)
                    } else {
                        None
                    },
                };

                if let Ok(json) = serde_json::to_string(&payload) {
                    let mut dead_clients = vec![];

                    for (id, client) in &mut clients {
                        if client.send(Message::Text(json.clone().into())).is_err() {
                            dead_clients.push(*id);
                        }
                    }

                    for id in dead_clients {
                        clients.remove(&id);
                        log::info!(target: "WebSocket", "Client disconnected.");
                    }
                }
            }
        }

        let target_duration = Duration::from_micros(1_000_000 / u64::from(hz));
        let elapsed = frame_start.elapsed();

        if elapsed < target_duration {
            thread::sleep(target_duration.checked_sub(elapsed).unwrap_or_default());
        } else {
            log::trace!(target: "WebSocket", "Broadcast too slow, frame took {elapsed:?}");
        }
    }
}
