use chrono::Local;
use log::{LevelFilter, Log, Metadata, Record};
use std::collections::VecDeque;
use std::sync::{Arc, LazyLock, RwLock};

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub group: String,
    pub message: String,
}

pub struct GlobalLogger {
    pub entries: Arc<RwLock<VecDeque<LogEntry>>>,
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
            if !self.enabled(record.metadata()) {
                return;
            }
                
            let target = record.target();

            // Whitelist: only these named targets appear in the in-app console
            let is_allowed = [
                    "App", "UI", "HID", "TabletManager", "Pipeline", "Config",
                    "Startup", "Update", "Stats", "Tray", "Timer", "WebSocket",
                    "Telemetry", "Driver", "Detect",
            ].iter().any(|&t| target == t || target.starts_with(&format!("{t}::")))
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

            if let Ok(mut entries) = self.entries.write() {
                if entries.len() >= MAX_LOGS {
                    entries.pop_front();
                }
                entries.push_back(entry);
            }
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    let logger = GlobalLogger {
        entries: LOG_BUFFER.clone(),
    };
    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Debug))
        .expect("Failed to initialize logger");
}
