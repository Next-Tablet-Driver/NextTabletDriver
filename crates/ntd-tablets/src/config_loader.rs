use crate::config::{DigitizerIdentifier, TabletConfiguration};
use include_dir::{Dir, DirEntry, include_dir};
use std::collections::{HashMap, HashSet};

/// The tablet definitions of `OpenTabletDriver`, embedded in the binary at compile time.
pub static TABLET_CONFIGS_DIR: Dir = include_dir!(
    // The macro takes one string literal, so the path cannot be split over several lines.
    "$CARGO_MANIFEST_DIR/../../tablets/OpenTabletDriver/OpenTabletDriver.Configurations/Configurations" // https://github.com/OpenTabletDriver/OpenTabletDriver
);

/// Tablet definitions by (`VendorID`, `ProductID`).
pub type ConfigIndex = HashMap<(u16, u16), Vec<(TabletConfiguration, DigitizerIdentifier)>>;

/// The embedded definitions, indexed.
pub static EMBEDDED_INDEX: std::sync::LazyLock<ConfigIndex> =
    std::sync::LazyLock::new(|| index_configurations(load_embedded_configurations()));

/// Indexes `configs` by the (`VendorID`, `ProductID`) of each of their digitizers.
#[must_use]
pub fn index_configurations(configs: Vec<TabletConfiguration>) -> ConfigIndex {
    let mut index = HashMap::new();

    for config in configs {
        for digitizer in &config.digitizer_identifiers {
            index
                .entry((digitizer.vendor_id, digitizer.product_id))
                .or_insert_with(Vec::new)
                .push((config.clone(), digitizer.clone()));
        }
    }

    log::info!(target: "Driver", "Indexed {} configurations across {} unique VID:PID \
        pairs", index.values().flatten().count(), index.len());
    index
}

/// Every embedded tablet definition; the first one with a given name wins.
#[must_use]
pub fn load_embedded_configurations() -> Vec<TabletConfiguration> {
    let mut configs = Vec::new();
    load_embedded_recursive(&TABLET_CONFIGS_DIR, &mut configs, &mut HashSet::new());
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
                                    log::error!(target: "Driver", "Failed to parse embedded config \
                                        {}: {e}", file.path().display());
                                }
                            }
                        }
                        None => {
                            log::warn!(target: "Driver", "Embedded config file {} is not valid \
                                UTF-8", file.path().display());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;

    fn for_each_embedded_file(dir: &Dir, f: &mut impl FnMut(&include_dir::File)) {
        for entry in dir.entries() {
            match entry {
                DirEntry::Dir(sub) => for_each_embedded_file(sub, f),
                DirEntry::File(file) => f(file),
            }
        }
    }

    #[test]
    fn the_embedded_catalogue_has_unique_names_and_sane_specs() {
        let configs = load_embedded_configurations();
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
        assert!(EMBEDDED_INDEX.len() > 100);
        for ((vid, pid), entries) in EMBEDDED_INDEX.iter() {
            assert!(!entries.is_empty());
            for (_, digitizer) in entries {
                assert_eq!((digitizer.vendor_id, digitizer.product_id), (*vid, *pid));
            }
        }
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
        crate::test_support::evaluate_log_arguments();
        let mut configs = Vec::new();
        let mut names = HashSet::new();
        load_embedded_recursive(&ROOT, &mut configs, &mut names);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "Embedded Alpha");
        assert!(names.contains("Embedded Alpha"));
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
