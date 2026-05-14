use crate::engine::state::WriteRecoverExt;
use crossbeam_channel::{Sender, select, unbounded};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tungstenite::{Message, WebSocket, accept};

pub struct StatsServer {
    shutdown_flag: Arc<AtomicBool>,
    sender: Sender<(f32, f32)>,
    thread_handle: Option<JoinHandle<()>>,
}

impl StatsServer {
    pub fn start(ip: String, port: u16) -> Result<Self, String> {
        let addr = format!("{ip}:{port}");
        let listener = TcpListener::bind(&addr).map_err(|e| format!("Failed to bind: {e}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {e}"))?;

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let (tx, rx) = unbounded::<(f32, f32)>();

        let shutdown_server = Arc::clone(&shutdown_flag);
        let handle = thread::spawn(move || {
            let clients = Arc::new(Mutex::new(Vec::new()));

            // Broadcast loop
            let clients_broadcast = Arc::clone(&clients);
            let shutdown_broadcast = Arc::clone(&shutdown_server);
            let rx_broadcast = rx.clone();
            thread::spawn(move || {
                while !shutdown_broadcast.load(Ordering::SeqCst) {
                    select! {
                        recv(rx_broadcast) -> msg => {
                            if let Ok((speed, total_dist)) = msg {
                                let mut clients = clients_broadcast.lock().unwrap_or_reset("stats_clients");
                                let json = serde_json::json!({
                                    "handspeed": speed,
                                    "total_distance": total_dist,
                                    "timestamp": std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis()
                                }).to_string();

                                clients.retain_mut(|client: &mut WebSocket<TcpStream>| {
                                    client.send(Message::Text(json.clone().into())).is_ok()
                                });
                            }
                        },
                        default(std::time::Duration::from_millis(100)) => {}
                    }
                }
                log::info!(target: "Stats", "Broadcast thread shut down");
            });

            // Accept loop
            while !shutdown_server.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => match accept(stream) {
                        Ok(ws) => {
                            let mut clients = clients.lock().unwrap_or_reset("stats_clients");
                            clients.push(ws);
                            log::debug!(target: "Stats", "New WebSocket client connected");
                        }
                        Err(e) => log::error!(target: "Stats", "WebSocket accept error: {e}"),
                    },
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => log::error!(target: "Stats", "TCP accept error: {e}"),
                }
            }
            log::info!(target: "Stats", "Server accept loop shut down");
        });

        Ok(Self {
            shutdown_flag,
            sender: tx,
            thread_handle: Some(handle),
        })
    }

    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn send_stats(&self, speed: f32, total_dist: f32) {
        let _ = self.sender.try_send((speed, total_dist));
    }
}

impl Drop for StatsServer {
    fn drop(&mut self) {
        self.stop();
    }
}
