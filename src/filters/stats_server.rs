use crate::engine::state::WriteRecoverExt;
use crossbeam_channel::{Sender, select, unbounded};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tungstenite::{Message, WebSocket, accept};

/// An embedded WebSocket server that broadcasts pen statistics (such as hand speed
/// and cumulative distance traveled) to connected clients in real-time.
pub struct StatsServer {
    shutdown_flag: Arc<AtomicBool>,
    sender: Sender<(f32, f32)>,
    thread_handle: Option<JoinHandle<()>>,
}

impl StatsServer {
    /// Starts the stats server.
    ///
    /// # Errors
    /// Returns an error if the TCP listener fails to bind to the specified address
    /// or if setting the listener to non-blocking mode fails.
    pub fn start(ip: &str, port: u16) -> Result<Self, String> {
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
                                    match client.send(Message::Text(json.clone().into())) {
                                        Ok(()) => true,
                                        Err(tungstenite::Error::Io(ref io_err)) if io_err.kind() == std::io::ErrorKind::WouldBlock => true,
                                        Err(_) => false,
                                    }
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
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let _ =
                            stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));
                        let _ =
                            stream.set_write_timeout(Some(std::time::Duration::from_millis(100)));
                        match accept(stream) {
                            Ok(mut ws) => {
                                let _ = ws.get_mut().set_nonblocking(true);
                                let mut clients = clients.lock().unwrap_or_reset("stats_clients");
                                clients.push(ws);
                                log::debug!(target: "Stats", "New WebSocket client connected");
                            }
                            Err(e) => log::error!(target: "Stats", "WebSocket accept error: {e}"),
                        }
                    }
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

    /// Signals the broadcast and acceptor threads to terminate and blocks until they join.
    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Queues a new stats packet `(handspeed_mm_s, total_distance_mm)` to be broadcasted to all active clients.
    pub fn send_stats(&self, speed: f32, total_dist: f32) {
        let _ = self.sender.try_send((speed, total_dist));
    }
}

impl Drop for StatsServer {
    fn drop(&mut self) {
        self.stop();
    }
}
