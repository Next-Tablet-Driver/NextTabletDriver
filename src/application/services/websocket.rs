//! # WebSocket Server
//!
//! This module provides an embedded WebSocket server that broadcasts real-time
//! tablet data (position, pressure, status) to external clients. This is primarily
//! designed for streamer overlays, custom UI integrations, or third-party plugins.

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::protocol::WebSocket;
use tungstenite::{Message, accept_hdr};

use crate::engine::state::{LockRecoveryExt, SharedState};

/// The JSON payload broadcasted to all connected WebSocket clients.
///
/// Fields are dynamically omitted (via `skip_serializing_if = "Option::is_none"`)
/// depending on the user's privacy/performance settings in the Settings tab.
#[derive(Serialize)]
struct WsPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    x: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pressure: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_connected: Option<bool>,
}

impl WsPayload {
    fn from_tablet_data(
        data: &crate::drivers::TabletData,
        send_coords: bool,
        send_pressure: bool,
        send_status: bool,
    ) -> Self {
        Self {
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
        }
    }
}

/// Maximum number of simultaneously connected clients.
const MAX_CLIENTS: usize = 10;
/// Maximum number of handshakes allowed to be in flight at once.
const MAX_PENDING_HANDSHAKES: usize = 4;

/// Returns `true` if a browser `Origin` header may talk to this server.
///
/// The server only listens on loopback, but any web page the user visits can still open
/// `ws://127.0.0.1:<port>` from their browser and read live pen data. Browsers always send
/// an `Origin`, so only loopback pages and local files (`null` / `file://`, used by local
/// overlays such as OBS browser sources) are accepted. Non-browser clients send no `Origin`.
fn is_origin_allowed(origin: &str) -> bool {
    let origin = origin.trim();
    if origin.eq_ignore_ascii_case("null") || origin.to_ascii_lowercase().starts_with("file://") {
        return true;
    }
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    let authority = rest.split('/').next().unwrap_or("");
    let host = authority.strip_prefix('[').map_or_else(
        || authority.split(':').next().unwrap_or(""),
        |stripped| stripped.split(']').next().unwrap_or(""),
    );
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

#[allow(clippy::result_large_err)]
fn check_origin(request: &Request, response: Response) -> Result<Response, ErrorResponse> {
    let allowed = request
        .headers()
        .get("Origin")
        .is_none_or(|v| v.to_str().is_ok_and(is_origin_allowed));
    if allowed {
        Ok(response)
    } else {
        log::warn!(target: "WebSocket", "Rejected WebSocket connection from a disallowed Origin");
        let mut resp = ErrorResponse::new(Some("Forbidden origin".to_string()));
        *resp.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
        Err(resp)
    }
}

/// Performs the (blocking) WebSocket handshake on its own thread so that a slow or
/// malicious client can never stall the broadcast loop.
fn spawn_handshake(
    stream: std::net::TcpStream,
    pending: &Arc<AtomicUsize>,
    ready: &crossbeam_channel::Sender<WebSocket<std::net::TcpStream>>,
) {
    pending.fetch_add(1, Ordering::AcqRel);
    let pending_t = Arc::clone(pending);
    let ready_t = ready.clone();
    let spawned = thread::Builder::new()
        .name("WebSocketHandshake".into())
        .spawn(move || {
            let result = (|| {
                stream.set_nonblocking(false).ok()?;
                stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
                stream
                    .set_write_timeout(Some(Duration::from_millis(500)))
                    .ok()?;
                let ws = accept_hdr(stream, check_origin)
                    .map_err(|e| {
                        log::warn!(target: "WebSocket", "Error during WebSocket handshake: {e}");
                    })
                    .ok()?;
                ws.get_ref().set_read_timeout(None).ok()?;
                ws.get_ref().set_nonblocking(true).ok()?;
                Some(ws)
            })();
            if let Some(ws) = result {
                let _ = ready_t.send(ws);
            }
            pending_t.fetch_sub(1, Ordering::AcqRel);
        });
    if let Err(e) = spawned {
        log::error!(target: "WebSocket", "Failed to spawn handshake thread: {e}");
        pending.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Runs the embedded WebSocket server in a dedicated background thread.
///
/// # Behavior
/// Reads the current configuration (`WebSocketConfig`) from `SharedState`, manages the
/// lifecycle of a `TcpListener` based on the user-configured port, accepts incoming
/// WebSocket handshakes and stores active connections, then reads the most recent
/// `TabletData` and broadcasts it to all connected clients at the user-defined polling
/// rate (Hz).
///
/// # Networking
/// - Binds to `127.0.0.1` (localhost only) for security.
/// - Uses non-blocking sockets to allow for graceful client disconnection and reconnects.
pub fn websocket_loop(shared: &Arc<SharedState>) {
    let mut current_port = 0;
    let mut listener: Option<TcpListener> = None;
    let mut clients: HashMap<usize, WebSocket<std::net::TcpStream>> = HashMap::new();
    let mut next_client_id = 0;
    let mut last_bind_attempt: Option<Instant> = None;
    let pending_handshakes = Arc::new(AtomicUsize::new(0));
    let (ready_tx, ready_rx) = crossbeam_channel::unbounded::<WebSocket<std::net::TcpStream>>();

    loop {
        if shared.lifecycle.shutdown_requested.load(Ordering::Relaxed) {
            log::info!(target: "WebSocket", "Shutdown requested, exiting WebSocket loop");
            break;
        }

        let frame_start = Instant::now();

        let (enabled, port, hz, send_coords, send_pressure, _send_tilt, send_status) = {
            let config = shared.config.mapping.read().unwrap_or_log("config");
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
            last_bind_attempt = None;
        } else if listener.is_none() || current_port != port {
            let is_port_change = current_port != port;
            let should_bind = is_port_change
                || last_bind_attempt.is_none_or(|t| t.elapsed() >= Duration::from_secs(5));

            if should_bind {
                log::info!(target: "WebSocket", "Starting WebSocket Server on 127.0.0.1:{port}");
                clients.clear();
                last_bind_attempt = Some(Instant::now());
                current_port = port;

                match TcpListener::bind(format!("127.0.0.1:{port}")) {
                    Ok(l) => match l.set_nonblocking(true) {
                        Ok(()) => {
                            listener = Some(l);
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
        }

        if let Some(l) = &listener {
            match l.accept() {
                Ok((stream, addr)) => {
                    let pending = pending_handshakes.load(Ordering::Acquire);
                    if clients.len() + pending >= MAX_CLIENTS || pending >= MAX_PENDING_HANDSHAKES {
                        log::warn!(target: "WebSocket", "Too many clients or pending handshakes, rejecting {addr}");
                    } else {
                        log::info!(target: "WebSocket", "New connection from {addr}");
                        // The blocking handshake runs off-thread so it can never stall the broadcast.
                        spawn_handshake(stream, &pending_handshakes, &ready_tx);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    log::error!(target: "WebSocket", "Listener error: {e}");
                }
            }

            while let Ok(websocket) = ready_rx.try_recv() {
                if clients.len() < MAX_CLIENTS {
                    clients.insert(next_client_id, websocket);
                    next_client_id += 1;
                    crate::application::telemetry::capture_event(
                        "websocket_client_connected",
                        None,
                    );
                }
            }

            // Drain incoming frames so Close/Ping are processed and dead peers are noticed
            // promptly; payloads sent by clients are ignored.
            let mut dead_clients: Vec<usize> = Vec::new();
            for (id, client) in &mut clients {
                loop {
                    match client.read() {
                        Ok(Message::Close(_)) | Err(tungstenite::Error::ConnectionClosed) => {
                            dead_clients.push(*id);
                            break;
                        }
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(ref e))
                            if e.kind() == std::io::ErrorKind::WouldBlock =>
                        {
                            break;
                        }
                        Err(_) => {
                            dead_clients.push(*id);
                            break;
                        }
                    }
                }
            }

            if !clients.is_empty() {
                let data = shared
                    .pipeline
                    .tablet_data
                    .read()
                    .map(|d| d.clone())
                    .unwrap_or_default();

                let payload =
                    WsPayload::from_tablet_data(&data, send_coords, send_pressure, send_status);

                if let Ok(json) = serde_json::to_string(&payload) {
                    // `Utf8Bytes` clones are ref-counted, unlike cloning the `String` per client.
                    let message = Message::Text(json.into());

                    for (id, client) in &mut clients {
                        if dead_clients.contains(id) {
                            continue;
                        }
                        if let Err(e) = client.send(message.clone()) {
                            if let tungstenite::Error::Io(ref io_err) = e
                                && io_err.kind() == std::io::ErrorKind::WouldBlock
                            {
                                continue;
                            }
                            dead_clients.push(*id);
                        }
                    }
                }
            }

            for id in dead_clients {
                clients.remove(&id);
                log::info!(target: "WebSocket", "Client disconnected.");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::{TabletData, TabletStatus};

    #[test]
    fn test_origin_policy() {
        assert!(is_origin_allowed("null"));
        assert!(is_origin_allowed("file:///C:/overlay/index.html"));
        assert!(is_origin_allowed("http://localhost:3000"));
        assert!(is_origin_allowed("http://127.0.0.1"));
        assert!(is_origin_allowed("https://[::1]:8443"));
        assert!(!is_origin_allowed("https://evil.example"));
        assert!(!is_origin_allowed("http://localhost.evil.example"));
        assert!(!is_origin_allowed("http://127.0.0.1.evil.example"));
        assert!(!is_origin_allowed("ftp://localhost"));
        assert!(!is_origin_allowed("garbage"));
    }

    #[test]
    fn test_ws_payload_full_serialization_with_large_u32_coords() {
        let data = TabletData {
            x: 105_370, // Exceeds u16 max (65535)
            y: 75_200,  // Exceeds u16 max (65535)
            pressure: 8191,
            status: TabletStatus::Contact,
            is_connected: true,
            ..Default::default()
        };

        let payload = WsPayload::from_tablet_data(&data, true, true, true);
        let json_str = serde_json::to_string(&payload).expect("Failed to serialize payload");
        let v: serde_json::Value =
            serde_json::from_str(&json_str).expect("Failed to deserialize JSON");

        assert_eq!(v["x"], 105_370);
        assert_eq!(v["y"], 75_200);
        assert_eq!(v["pressure"], 8191);
        assert_eq!(v["status"], "Contact");
        assert_eq!(v["is_connected"], true);
    }

    #[test]
    fn test_ws_payload_privacy_omits_disabled_fields() {
        let data = TabletData {
            x: 500,
            y: 500,
            pressure: 1000,
            status: TabletStatus::Hover,
            is_connected: true,
            ..Default::default()
        };

        // Disable coordinates and pressure, keep status
        let payload = WsPayload::from_tablet_data(&data, false, false, true);
        let json_str = serde_json::to_string(&payload).expect("Failed to serialize payload");

        assert!(!json_str.contains("\"x\""));
        assert!(!json_str.contains("\"y\""));
        assert!(!json_str.contains("\"pressure\""));
        assert!(json_str.contains("\"status\":\"Hover\""));
        assert!(json_str.contains("\"is_connected\":true"));
    }

    #[test]
    fn test_ws_payload_all_disabled_produces_empty_json_object() {
        let data = TabletData {
            x: 500,
            y: 500,
            pressure: 1000,
            status: TabletStatus::Hover,
            is_connected: true,
            ..Default::default()
        };

        let payload = WsPayload::from_tablet_data(&data, false, false, false);
        let json_str = serde_json::to_string(&payload).expect("Failed to serialize payload");
        assert_eq!(json_str, "{}");
    }

    #[test]
    fn test_websocket_broadcast_when_app_hidden_in_tray() {
        use crate::engine::state::{SharedState, WriteRecoverExt};
        use std::sync::Arc;
        use std::sync::atomic::Ordering;
        use std::thread;
        use std::time::Duration;

        let shared = Arc::new(SharedState::new());
        // Explicitly simulate the application running in background / minimized in system tray
        shared.lifecycle.is_visible.store(false, Ordering::Relaxed);

        // Find an available port on loopback
        let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("probe bind failed");
        let port = probe.local_addr().expect("probe addr failed").port();
        drop(probe);

        {
            let mut cfg = shared.config.mapping.write().unwrap_or_reset("config");
            cfg.websocket.enabled = true;
            cfg.websocket.port = port;
            cfg.websocket.polling_rate_hz = 120;
            cfg.websocket.send_coordinates = true;
            cfg.websocket.send_pressure = true;
            cfg.websocket.send_status = true;
        }

        let shared_clone = Arc::clone(&shared);
        let server_thread = thread::spawn(move || {
            websocket_loop(&shared_clone);
        });

        // Retry connecting until the server listener is up
        let mut client = None;
        for _ in 0..50 {
            if let Ok(stream) = std::net::TcpStream::connect(format!("127.0.0.1:{port}"))
                && let Ok((ws, _)) = tungstenite::client(format!("ws://127.0.0.1:{port}"), stream)
            {
                client = Some(ws);
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }

        let mut client = client.expect("WebSocket client failed to connect to server");

        // Simulate high-frequency polling thread writing to shared.pipeline.tablet_data
        // while the UI is hidden (is_visible == false)
        *shared
            .pipeline
            .tablet_data
            .write()
            .unwrap_or_reset("tablet_data") = TabletData {
            x: 8888,
            y: 9999,
            pressure: 4096,
            status: TabletStatus::Contact,
            is_connected: true,
            ..Default::default()
        };

        // Read until we receive the message containing the new coordinates
        client
            .get_mut()
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set_read_timeout failed");

        let mut received_target = false;
        for _ in 0..50 {
            let msg = client.read().expect("Failed to read WebSocket message");
            if let tungstenite::Message::Text(text) = msg {
                let v: serde_json::Value = serde_json::from_str(&text).expect("Valid JSON");
                if v["x"] == 8888 && v["y"] == 9999 {
                    assert_eq!(v["pressure"], 4096);
                    assert_eq!(v["status"], "Contact");
                    received_target = true;
                    break;
                }
            }
        }

        assert!(
            received_target,
            "Did not receive updated coordinates over WebSocket while app was hidden in tray"
        );

        // Shut down the server gracefully
        shared
            .lifecycle
            .shutdown_requested
            .store(true, Ordering::Relaxed);
        let _ = server_thread.join();
    }
}
