use crate::config::{DigitizerIdentifier, TabletConfiguration};
use crate::descriptor::{self, TabletDescriptor};
use std::collections::HashMap;

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

/// Every embedded tablet definition, in the order of the catalogue.
#[must_use]
pub fn load_embedded_configurations() -> Vec<TabletConfiguration> {
    descriptor::all()
        .iter()
        .map(TabletDescriptor::to_configuration)
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "the tests compare floats that are exact by construction or copied unchanged"
)]
mod tests {
    use super::*;
    use std::collections::HashSet;

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

    #[test]
    fn the_index_agrees_with_the_static_lookup() {
        crate::test_support::evaluate_log_arguments();
        for ((vid, pid), entries) in EMBEDDED_INDEX.iter() {
            let found: Vec<_> = descriptor::find(*vid, *pid).collect();
            assert_eq!(found.len(), entries.len(), "{vid:04x}:{pid:04x}");
            for ((config, digitizer), (tablet, descriptor)) in entries.iter().zip(found) {
                assert_eq!(config.name, tablet.name());
                assert_eq!(
                    digitizer.report_parser,
                    descriptor.to_identifier().report_parser
                );
            }
        }
    }
}
