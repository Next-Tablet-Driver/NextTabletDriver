//! The tablet definitions the program loads: the ones in the local `tablets` folder, then the ones
//! embedded in `ntd-tablets`.

use ntd_tablets::TabletConfiguration;
use ntd_tablets::config_loader::{ConfigIndex, index_configurations, load_embedded_configurations};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Pre-indexed configuration map: (`VendorID`, `ProductID`) -> (Config, `DigitizerInfo`)
pub static INDEXED_CONFIGS: std::sync::LazyLock<ConfigIndex> =
    std::sync::LazyLock::new(|| index_configurations(load_configurations()));

/// Loads every tablet definition: the ones in the local `tablets` folder first, then the embedded
/// ones.
#[must_use]
pub fn load_configurations() -> Vec<TabletConfiguration> {
    load_configurations_from(Path::new("tablets"))
}

/// Tablet configurations found in `local_dir` (if it exists), then the embedded ones; the first
/// configuration with a given name wins.
fn load_configurations_from(local_dir: &Path) -> Vec<TabletConfiguration> {
    let global_start = Instant::now();
    let mut configs = Vec::new();
    let mut loaded_names = HashSet::new();

    if local_dir.exists() {
        let disk_start = Instant::now();
        load_from_disk_recursive(local_dir, &mut configs, &mut loaded_names);
        log::debug!(
            target: "Driver",
            "Loaded {} configs from disk in {:.2?}",
            configs.len(),
            disk_start.elapsed()
        );
    }

    let embedded_start = Instant::now();
    let prev_len = configs.len();
    for config in load_embedded_configurations() {
        if loaded_names.insert(config.name.clone()) {
            configs.push(config);
        }
    }
    log::debug!(
        target: "Driver",
        "Loaded {} configs from embedded in {:.2?}",
        configs.len() - prev_len,
        embedded_start.elapsed()
    );

    log::info!(
        target: "Driver",
        "Total {} tablet configurations loaded in {:.2?}",
        configs.len(),
        global_start.elapsed()
    );
    configs
}

fn load_from_disk_recursive(
    path: &Path,
    configs: &mut Vec<TabletConfiguration>,
    names: &mut HashSet<String>,
) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                load_from_disk_recursive(&p, configs, names);
            } else if p.extension().and_then(|s| s.to_str()) == Some("json") {
                match fs::read_to_string(&p) {
                    Ok(content) => match serde_json::from_str::<TabletConfiguration>(&content) {
                        Ok(config) => {
                            if !names.contains(&config.name) {
                                names.insert(config.name.clone());
                                configs.push(config);
                            }
                        }
                        Err(e) => {
                            log::error!(target: "Driver", "Failed to parse disk config {}: \
                                {e}", p.display());
                        }
                    },
                    Err(e) => {
                        log::error!(target: "Driver", "Failed to read disk config {}: \
                            {e}", p.display());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const MINIMAL: &str = r#"{
        "Name": "NAME",
        "Specifications": {
            "Digitizer": { "Width": 160.0, "Height": 100.0, "MaxX": 16000, "MaxY": 10000 },
            "Pen": { "MaxPressure": 8191 }
        },
        "DigitizerIdentifiers": [
            { "VendorID": 1, "ProductID": 2, "ReportParser": "Test.Tablet.ReportParser" }
        ]
    }"#;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ntd_configs_{name}_{nanos}"));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn disk_loading_walks_subfolders_and_skips_duplicates_and_junk() {
        let dir = TempDir::new("walk");
        fs::write(dir.0.join("a.json"), MINIMAL.replace("NAME", "Alpha")).unwrap();
        fs::create_dir_all(dir.0.join("nested")).unwrap();
        fs::write(
            dir.0.join("nested").join("b.json"),
            MINIMAL.replace("NAME", "Beta"),
        )
        .unwrap();
        // Same name as an already loaded configuration: ignored.
        fs::write(
            dir.0.join("nested").join("dup.json"),
            MINIMAL.replace("NAME", "Alpha"),
        )
        .unwrap();
        fs::write(dir.0.join("broken.json"), "{ nope").unwrap();
        fs::write(dir.0.join("notes.txt"), "not a configuration").unwrap();

        let mut configs = Vec::new();
        let mut names = HashSet::new();
        load_from_disk_recursive(&dir.0, &mut configs, &mut names);

        let mut loaded: Vec<_> = configs.iter().map(|c| c.name.clone()).collect();
        loaded.sort();
        assert_eq!(loaded, ["Alpha", "Beta"]);
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn disk_loading_a_missing_folder_is_a_no_op() {
        let mut configs = Vec::new();
        let mut names = HashSet::new();
        load_from_disk_recursive(Path::new("ntd-no-such-folder"), &mut configs, &mut names);
        assert!(configs.is_empty());
    }

    #[test]
    fn disk_loading_reports_files_that_cannot_be_read_or_parsed() {
        log::set_max_level(log::LevelFilter::Trace);
        let dir = TempDir::new("unreadable");
        fs::write(dir.0.join("binary.json"), [0xFF, 0xFE, 0xFD]).unwrap();
        fs::write(dir.0.join("broken.json"), "{ nope").unwrap();
        let mut configs = Vec::new();
        let mut names = HashSet::new();
        load_from_disk_recursive(&dir.0, &mut configs, &mut names);
        assert!(configs.is_empty());
    }

    #[test]
    fn smoke_the_whole_catalogue_loads_with_logging_enabled() {
        log::set_max_level(log::LevelFilter::Trace);
        assert!(load_configurations().len() > 100);
        assert!(!INDEXED_CONFIGS.is_empty());
    }

    #[test]
    fn without_a_local_folder_only_the_embedded_catalogue_is_loaded() {
        let configs = load_configurations_from(Path::new("ntd-no-such-folder"));
        assert!(configs.len() > 100);
    }

    #[test]
    fn local_configurations_are_loaded_before_the_embedded_ones() {
        let dir = TempDir::new("local_first");
        fs::write(
            dir.0.join("mine.json"),
            MINIMAL.replace("NAME", "Disk Alpha"),
        )
        .unwrap();
        let configs = load_configurations_from(&dir.0);
        assert_eq!(configs[0].name, "Disk Alpha");
        assert!(configs.len() > 100);
    }
}
