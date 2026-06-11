use chrono::Local;
use crossbeam_channel::Sender;
use log::{LevelFilter, Log, Metadata, Record};
use std::collections::VecDeque;
use std::sync::{Arc, LazyLock, RwLock};
use std::thread;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub group: String,
    pub message: String,
    pub search_text: String,
}

pub struct GlobalLogger {
    pub sender: Sender<LogEntry>,
}

pub const MAX_LOGS: usize = 2000;

pub static LOG_BUFFER: LazyLock<Arc<RwLock<VecDeque<LogEntry>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(VecDeque::with_capacity(MAX_LOGS))));

/// Global monotonic log sequence counter.
///
/// Incremented every time a log is successfully added to `LOG_BUFFER` or the buffer is cleared.
/// The UI uses this sequence number to invalidate its console cache instead of relying on
/// `LOG_BUFFER.len()`, which fails to trigger updates once the circular buffer reaches `MAX_LOGS` capacity.
pub static LOG_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Log for GlobalLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let target = record.target();

            // Whitelist: only these named targets appear in the in-app console
            let is_allowed = [
                "App",
                "UI",
                "HID",
                "TabletManager",
                "Pipeline",
                "Config",
                "Startup",
                "Update",
                "Stats",
                "Tray",
                "Timer",
                "WebSocket",
                "Telemetry",
                "Driver",
                "Detect",
            ]
            .iter()
            .any(|&t| target == t || target.starts_with(&format!("{t}::")))
                || target.starts_with("NextTabletDriver");

            if !is_allowed {
                return;
            }

            let level = format!("{:?}", record.level());
            let group = target.to_string();
            let message = format!("{}", record.args());
            let search_text = format!("{} {}", group.to_lowercase(), message.to_lowercase());

            let entry = LogEntry {
                time: Local::now().format("%H:%M:%S").to_string(),
                level,
                group,
                message,
                search_text,
            };

            // Send to worker thread (non-blocking)
            let _ = self.sender.send(entry);
        }
    }

    fn flush(&self) {}
}

/// Initializes the global logger.
///
/// # Errors
///
/// This function returns an error if:
/// - It fails to spawn the background logger worker thread.
/// - It fails to set the global logger instance.
pub fn init() -> Result<(), String> {
    let (sender, receiver) = crossbeam_channel::unbounded::<LogEntry>();

    // Spawn the logger worker thread
    let spawn_result = thread::Builder::new()
        .name("LoggerWorker".to_string())
        .spawn(move || {
            let log_dir = crate::settings::get_settings_dir();
            let session_log_path = log_dir.join("session.log");

            let mut session_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&session_log_path)
                .ok();

            while let Ok(entry) = receiver.recv() {
                let log_line = format!(
                    "[{}] {} [{}] {}",
                    entry.time, entry.level, entry.group, entry.message
                );

                // 1. Unbounded File Output
                if let Some(file) = &mut session_file {
                    use std::io::Write;
                    let _ = writeln!(file, "{log_line}");
                }

                if cfg!(debug_assertions) {
                    println!("{log_line}");
                }

                // 2. Update UI Buffer
                if let Ok(mut entries) = LOG_BUFFER.write() {
                    if entries.len() >= MAX_LOGS {
                        entries.pop_front();
                    }
                    entries.push_back(entry);
                    LOG_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }
        });

    if let Err(e) = spawn_result {
        return Err(format!("Failed to spawn logger worker thread: {e}"));
    }

    let logger = GlobalLogger { sender };

    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Debug))
        .map_err(|e| format!("Logger initialization failed: {e}"))
}
