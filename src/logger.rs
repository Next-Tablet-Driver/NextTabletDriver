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
}

pub struct GlobalLogger {
    pub sender: Sender<LogEntry>,
}

pub const MAX_LOGS: usize = 1000;

pub static LOG_BUFFER: LazyLock<Arc<RwLock<VecDeque<LogEntry>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(VecDeque::with_capacity(MAX_LOGS))));

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

            let entry = LogEntry {
                time: Local::now().format("%H:%M:%S").to_string(),
                level: format!("{:?}", record.level()),
                group: target.to_string(),
                message: format!("{}", record.args()),
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
            while let Ok(entry) = receiver.recv() {
                // 1. Console & File Output (Debug only)
                if cfg!(debug_assertions) {
                    let log_line = format!(
                        "[{}] {} [{}] {}",
                        entry.time, entry.level, entry.group, entry.message
                    );
                    println!("{log_line}");

                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("debug.log")
                    {
                        use std::io::Write;
                        let _ = writeln!(file, "{log_line}");
                    }
                }

                // 2. Update UI Buffer
                if let Ok(mut entries) = LOG_BUFFER.write() {
                    if entries.len() >= MAX_LOGS {
                        entries.pop_front();
                    }
                    entries.push_back(entry);
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
