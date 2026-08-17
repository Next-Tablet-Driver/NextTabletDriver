//! State and log-filtering logic for the console/log viewer tab.

use super::TabletMapperApp;
use crate::engine::state::LockRecoveryExt;
use std::sync::atomic::Ordering;

/// UI state for the console/log viewer tab: search, level filters, and the
/// derived filtered-log cache.
#[allow(clippy::struct_excessive_bools)]
pub struct ConsoleState {
    /// Sub-string filter for searching the console logs.
    pub search: String,
    /// Show INFO level logs in the console panel.
    pub show_info: bool,
    /// Show WARN level logs in the console panel.
    pub show_warn: bool,
    /// Show ERROR level logs in the console panel.
    pub show_error: bool,
    /// Show DEBUG level logs in the console panel.
    pub show_debug: bool,
    /// Automatically scroll to the bottom when a new log arrives.
    pub autoscroll: bool,
    /// Monotonically increasing sequence number used to track if new logs have been received
    /// and if the cache needs to be re-filtered and regenerated.
    pub cache_log_sequence: u64,
    /// The search term used to generate the current cache.
    pub cache_search: String,
    /// The filter switches used to generate the current cache: `(info, warn, error, debug)`.
    pub cache_filters: (bool, bool, bool, bool),
    /// List of pre-filtered log entries currently loaded in the console UI.
    pub cache_filtered: Vec<crate::logger::LogEntry>,
}

impl TabletMapperApp {
    pub fn get_filtered_logs(&mut self) -> (usize, &[crate::logger::LogEntry]) {
        let logs = crate::logger::LOG_BUFFER.read().unwrap_or_log("logs");
        let current_filters = (
            self.console.show_info,
            self.console.show_warn,
            self.console.show_error,
            self.console.show_debug,
        );
        let current_sequence = crate::logger::LOG_SEQUENCE.load(Ordering::Acquire);
        if self.console.cache_log_sequence == current_sequence
            && self.console.cache_search == self.console.search
            && self.console.cache_filters == current_filters
        {
            return (logs.len(), &self.console.cache_filtered);
        }
        let search_lower = self.console.search.to_lowercase();
        let mut filtered: Vec<_> = logs
            .iter()
            .filter(|log| {
                let level_match = match log.level.as_str() {
                    "Info" => self.console.show_info,
                    "Warn" => self.console.show_warn,
                    "Error" => self.console.show_error,
                    "Debug" => self.console.show_debug,
                    _ => true,
                };
                if !level_match {
                    return false;
                }
                if search_lower.is_empty() {
                    return true;
                }
                log.search_text.contains(&search_lower)
            })
            .cloned()
            .collect();
        filtered.reverse();
        let all_count = logs.len();
        drop(logs);
        self.console.cache_filtered = filtered;
        self.console.cache_log_sequence = current_sequence;
        self.console.cache_search = self.console.search.clone();
        self.console.cache_filters = current_filters;

        (all_count, &self.console.cache_filtered)
    }
}
