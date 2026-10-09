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
