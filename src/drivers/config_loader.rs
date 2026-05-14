use crate::drivers::config::{DigitizerIdentifier, TabletConfiguration};
use include_dir::{Dir, DirEntry, include_dir};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::Instant;

pub static TABLET_CONFIGS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/tablets");

type ConfigIndex = HashMap<(u16, u16), Vec<(TabletConfiguration, DigitizerIdentifier)>>;

/// Pre-indexed configuration map: (`VendorID`, `ProductID`) -> (Config, `DigitizerInfo`)
pub static INDEXED_CONFIGS: std::sync::LazyLock<ConfigIndex> =
    std::sync::LazyLock::new(load_and_index_configurations);

fn load_and_index_configurations() -> ConfigIndex {
    let configs = load_configurations();
    let mut index = HashMap::new();

    for config in configs {
        for digitizer in &config.digitizer_identifiers {
            index
                .entry((digitizer.vendor_id, digitizer.product_id))
                .or_insert_with(Vec::new)
                .push((config.clone(), digitizer.clone()));
        }
    }

    log::info!(target: "Driver", "Indexed {} configurations across {} unique VID:PID pairs", index.values().flatten().count(), index.len());
    index
}

#[must_use]
pub fn load_configurations() -> Vec<TabletConfiguration> {
    let global_start = Instant::now();
    let mut configs = Vec::new();
    let mut loaded_names = HashSet::new();

    let local_dir = Path::new("tablets");
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
    load_embedded_recursive(&TABLET_CONFIGS_DIR, &mut configs, &mut loaded_names);
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

fn load_embedded_recursive(
    dir: &Dir,
    configs: &mut Vec<TabletConfiguration>,
    names: &mut HashSet<String>,
) {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(sub_dir) => {
                load_embedded_recursive(sub_dir, configs, names);
            }
            DirEntry::File(file) => {
                if file.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    match file.contents_utf8() {
                        Some(content_str) => {
                            match serde_json::from_str::<TabletConfiguration>(content_str) {
                                Ok(config) => {
                                    if !names.contains(&config.name) {
                                        configs.push(config);
                                    }
                                }
                                Err(e) => {
                                    log::error!(target: "Driver", "Failed to parse embedded config {}: {e}", file.path().display());
                                }
                            }
                        }
                        None => {
                            log::warn!(target: "Driver", "Embedded config file {} is not valid UTF-8", file.path().display());
                        }
                    }
                }
            }
        }
    }
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
                            log::error!(target: "Driver", "Failed to parse disk config {}: {e}", p.display());
                        }
                    },
                    Err(e) => {
                        log::error!(target: "Driver", "Failed to read disk config {}: {e}", p.display());
                    }
                }
            }
        }
    }
}
