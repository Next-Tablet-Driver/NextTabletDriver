//! Minimal stderr logger for the SDK.
//!
//! `engine_loop.rs`'s `log::info!`/`warn!`/`error!` calls are otherwise
//! complete no-ops when the SDK is embedded standalone: the only place in
//! the whole workspace that installs a `log` backend is the desktop app's
//! `logger::init()` (`src/logger.rs`), called from `src/main.rs` behind the
//! `gui` feature that `sdk/` never enables. Without this, a consumer has no
//! way to tell "no supported tablet is connected" apart from "something
//! failed silently" (HID init failure, stuck arbitration, ...).
//!
//! Installed once by `ntd_init`. Level is controlled by the `NTD_SDK_LOG`
//! environment variable (`off`/`error`/`warn`/`info`/`debug`/`trace`,
//! case-insensitive), defaulting to `info` if unset or unrecognised.

use log::{LevelFilter, Log, Metadata, Record};
use std::sync::Once;

struct StderrLogger;

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "[ntd_sdk] {:<5} [{}] {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn level_from_env() -> LevelFilter {
    std::env::var("NTD_SDK_LOG").map_or(LevelFilter::Info, |value| {
        match value.to_ascii_lowercase().as_str() {
            "off" => LevelFilter::Off,
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    })
}

/// Installs the SDK's stderr logger, unless a logger is already installed
/// (e.g. the host application set its own before/after loading the SDK).
/// Safe to call more than once -- only the first call has any effect.
pub fn init() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let level = level_from_env();
        if log::set_boxed_logger(Box::new(StderrLogger)).is_ok() {
            log::set_max_level(level);
        }
    });
}
