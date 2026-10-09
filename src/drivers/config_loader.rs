use crate::drivers::config::{DigitizerIdentifier, TabletConfiguration};
use include_dir::{Dir, DirEntry, include_dir};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::Instant;

pub static TABLET_CONFIGS_DIR: Dir = include_dir!(
    "$CARGO_MANIFEST_DIR/tablets/OpenTabletDriver/OpenTabletDriver.Configurations/Configurations"
);

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
                                        names.insert(config.name.clone());
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
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

    fn for_each_embedded_file(dir: &Dir, f: &mut impl FnMut(&include_dir::File)) {
        for entry in dir.entries() {
            match entry {
                DirEntry::Dir(sub) => for_each_embedded_file(sub, f),
                DirEntry::File(file) => f(file),
            }
        }
    }

    #[test]
    fn the_loaded_catalogue_has_unique_names_and_sane_specs() {
        let configs = load_configurations();
        assert!(configs.len() > 100);
        let names: HashSet<_> = configs.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names.len(), configs.len(), "duplicate tablet names");
        for config in &configs {
            assert!(!config.digitizer_identifiers.is_empty(), "{}", config.name);
            let d = &config.specifications.digitizer;
            assert!(d.width > 0.0 && d.height > 0.0, "{}", config.name);
            assert!(d.max_x > 0.0 && d.max_y > 0.0, "{}", config.name);
        }
    }

    #[test]
    fn the_index_is_keyed_by_the_ids_of_each_digitizer() {
        assert!(INDEXED_CONFIGS.len() > 100);
        for ((vid, pid), entries) in INDEXED_CONFIGS.iter() {
            assert!(!entries.is_empty());
            for (_, digitizer) in entries {
                assert_eq!((digitizer.vendor_id, digitizer.product_id), (*vid, *pid));
            }
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

    const GOOD: &[u8] = br#"{
        "Name": "Embedded Alpha",
        "Specifications": {
            "Digitizer": { "Width": 160.0, "Height": 100.0, "MaxX": 16000, "MaxY": 10000 },
            "Pen": { "MaxPressure": 8191 }
        },
        "DigitizerIdentifiers": [
            { "VendorID": 1, "ProductID": 2, "ReportParser": "Test.Tablet.ReportParser" }
        ]
    }"#;

    static ENTRIES: [DirEntry<'static>; 5] = [
        DirEntry::File(include_dir::File::new("good.json", GOOD)),
        DirEntry::File(include_dir::File::new("duplicate.json", GOOD)),
        DirEntry::File(include_dir::File::new("broken.json", b"{ nope")),
        DirEntry::File(include_dir::File::new("binary.json", &[0xFF, 0xFE, 0xFD])),
        DirEntry::File(include_dir::File::new("notes.txt", b"not a configuration")),
    ];

    static NESTED: [DirEntry<'static>; 1] = [DirEntry::Dir(Dir::new("sub", &ENTRIES))];
    static ROOT: Dir<'static> = Dir::new("", &NESTED);

    #[test]
    fn embedded_loading_walks_folders_and_skips_duplicates_junk_and_broken_files() {
        log::set_max_level(log::LevelFilter::Trace);
        let mut configs = Vec::new();
        let mut names = HashSet::new();
        load_embedded_recursive(&ROOT, &mut configs, &mut names);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "Embedded Alpha");
        assert!(names.contains("Embedded Alpha"));
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
    fn the_whole_catalogue_loads_with_logging_enabled() {
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

    #[test]
    fn every_embedded_configuration_parses() {
        let mut files = Vec::new();
        for_each_embedded_file(&TABLET_CONFIGS_DIR, &mut |file| {
            files.push((
                file.path().to_path_buf(),
                file.contents_utf8().unwrap_or_default().to_string(),
            ));
        });
        let configurations: Vec<_> = files
            .iter()
            .filter(|(path, _)| path.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        let unparsable = configurations
            .iter()
            .filter(|(_, text)| serde_json::from_str::<TabletConfiguration>(text).is_err())
            .count();
        assert_eq!(
            unparsable, 0,
            "{unparsable} embedded configurations do not parse"
        );
        let count = configurations.len();
        assert!(count > 100, "only {count} embedded configurations");
    }
}
