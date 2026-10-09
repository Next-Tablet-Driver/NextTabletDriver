//! Embedded WebSocket broadcast server for real-time pen statistics.

use crossbeam_channel::{Sender, select, unbounded};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tungstenite::{Message, WebSocket, accept};

/// Embedded WebSocket server broadcasting pen hand speed and distance to connected overlays.
pub struct StatsServer {
    shutdown_flag: Arc<AtomicBool>,
    sender: Sender<(f32, f32)>,
    thread_handle: Option<JoinHandle<()>>,
}

impl StatsServer {
    /// Starts the stats WebSocket server on `ip:port`.
    pub fn start(ip: &str, port: u16) -> Result<Self, String> {
        let addr = format!("{ip}:{port}");
        let listener =
            TcpListener::bind(&addr).map_err(|e| format!("Failed to bind {addr}: {e}"))?;
        let _ = listener.set_nonblocking(true);

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let (tx, rx) = unbounded::<(f32, f32)>();

        let shutdown_server = Arc::clone(&shutdown_flag);
        let handle = thread::spawn(move || {
            let clients: Arc<Mutex<Vec<WebSocket<TcpStream>>>> = Arc::new(Mutex::new(Vec::new()));

            let clients_broadcast = Arc::clone(&clients);
            let shutdown_broadcast = Arc::clone(&shutdown_server);
            let rx_broadcast = rx.clone();

            let broadcast_handle = thread::spawn(move || {
                while !shutdown_broadcast.load(Ordering::Relaxed) {
                    select! {
                        recv(rx_broadcast) -> msg => {
                            if let Ok((speed, total_dist)) = msg {
                                let unix_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis();

                                let json = serde_json::json!({
                                    "handspeed": speed,
                                    "total_distance": total_dist,
                                    "timestamp": unix_ms
                                }).to_string();

                                if let Ok(mut client_list) = clients_broadcast.lock() {
                                    client_list.retain_mut(|client| {
                                        match client.send(Message::Text(json.clone().into())) {
                                            Ok(()) => true,
                                            Err(tungstenite::Error::Io(ref io_err))
                                                if io_err.kind() == std::io::ErrorKind::WouldBlock => true,
                                            Err(_) => false,
                                        }
                                    });
                                }
                            }
                        },
                        default(std::time::Duration::from_millis(50)) => {}
                    }
                }
            });

            while !shutdown_server.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let _ =
                            stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));
                        let _ =
                            stream.set_write_timeout(Some(std::time::Duration::from_millis(100)));
                        if let Ok(mut ws) = accept(stream) {
                            let _ = ws.get_mut().set_nonblocking(true);
                            if let Ok(mut client_list) = clients.lock() {
                                client_list.push(ws);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => {
                        thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }

            let _ = broadcast_handle.join();
        });

        Ok(Self {
            shutdown_flag,
            sender: tx,
            thread_handle: Some(handle),
        })
    }

    /// Queues a stats payload to broadcast without blocking the caller.
    pub fn send_stats(&self, speed: f32, total_dist: f32) {
        let _ = self.sender.try_send((speed, total_dist));
    }

    /// Signals threads to stop and joins the worker handle.
    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for StatsServer {
    fn drop(&mut self) {
        self.stop();
    }
}
