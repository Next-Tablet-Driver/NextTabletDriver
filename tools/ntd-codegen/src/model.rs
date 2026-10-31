//! The tablet definitions as the JSON files describe them, and their validation.
//!
//! The structs list every key `OpenTabletDriver` uses today and refuse any other: a new upstream
//! field must be a conscious decision (use it, or add it here as ignored), not a silent loss.

use crate::Error;
use base64::Engine as _;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// One definition file.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub(crate) struct Tablet {
    pub(crate) name: String,
    pub(crate) specifications: Specifications,
    pub(crate) digitizer_identifiers: Vec<Digitizer>,
    pub(crate) attributes: Option<Attributes>,
    /// A second device (dedicated keys); the driver does not use it yet.
    #[serde(rename = "AuxiliaryDeviceIdentifiers")]
    pub(crate) _auxiliary_device_identifiers: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub(crate) struct Specifications {
    pub(crate) digitizer: DigitizerSpecs,
    pub(crate) pen: PenSpecs,
    pub(crate) auxiliary_buttons: Option<ButtonSpecs>,
    #[serde(rename = "Wheels")]
    pub(crate) _wheels: Option<serde_json::Value>,
    #[serde(rename = "MouseButtons")]
    pub(crate) _mouse_buttons: Option<serde_json::Value>,
    #[serde(rename = "Touch")]
    pub(crate) _touch: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub(crate) struct DigitizerSpecs {
    pub(crate) width: f64,
    pub(crate) height: f64,
    #[serde(rename = "MaxX")]
    pub(crate) max_x: f64,
    #[serde(rename = "MaxY")]
    pub(crate) max_y: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub(crate) struct PenSpecs {
    pub(crate) max_pressure: u16,
    pub(crate) button_count: Option<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub(crate) struct ButtonSpecs {
    pub(crate) button_count: Option<u8>,
}

/// The per-definition and per-digitizer overrides. Only the keys the driver reads are typed.
#[derive(Debug, Deserialize)]
pub(crate) struct Attributes {
    pub(crate) libinputoverride: Option<String>,
    #[serde(rename = "Interface")]
    pub(crate) interface: Option<String>,
    /// Other keys of `OpenTabletDriver` (`FeatureInitDelayMs`, `SkipTests`, `HidReports`,
    /// `VerifiedLPI`): the driver ignores them.
    #[serde(flatten)]
    pub(crate) _others: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub(crate) struct Digitizer {
    #[serde(rename = "VendorID")]
    pub(crate) vendor_id: u16,
    #[serde(rename = "ProductID")]
    pub(crate) product_id: u16,
    pub(crate) input_report_length: Option<usize>,
    pub(crate) output_report_length: Option<usize>,
    pub(crate) device_strings: Option<BTreeMap<u8, String>>,
    pub(crate) report_parser: String,
    pub(crate) output_init_report: Option<Vec<String>>,
    pub(crate) feature_init_report: Option<Vec<String>>,
    pub(crate) attributes: Option<Attributes>,
    /// String descriptor indexes to read on connection; the driver does not use them yet.
    #[serde(rename = "InitializationStrings")]
    pub(crate) _initialization_strings: Option<serde_json::Value>,
}

/// Every definition under `dir`, sorted by path, with the file each one comes from.
pub(crate) fn load(dir: &Path) -> Result<Vec<(PathBuf, Tablet)>, Error> {
    let mut files = Vec::new();
    collect(dir, "json", &mut files)?;
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, &e))?;
            let tablet = serde_json::from_str(&text).map_err(|source| Error::Json {
                path: path.clone(),
                source,
            })?;
            Ok((path, tablet))
        })
        .collect()
}

/// The names of the parser classes `OpenTabletDriver` defines under `dirs`.
pub(crate) fn parser_classes(dirs: &[&Path]) -> Result<BTreeSet<String>, Error> {
    let mut sources = Vec::new();
    for dir in dirs {
        collect(dir, "cs", &mut sources)?;
    }
    let mut classes = BTreeSet::new();
    for path in sources {
        let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, &e))?;
        for line in text.lines() {
            let mut words = line.split_whitespace().skip_while(|w| *w != "class");
            if let Some(name) = words.nth(1) {
                let name = name.split([':', '<', '(']).next().unwrap_or(name);
                classes.insert(name.to_string());
            }
        }
    }
    Ok(classes)
}

fn collect(dir: &Path, extension: &str, files: &mut Vec<PathBuf>) -> Result<(), Error> {
    for entry in std::fs::read_dir(dir).map_err(|e| Error::io(dir, &e))? {
        let path = entry.map_err(|e| Error::io(dir, &e))?.path();
        if path.is_dir() {
            collect(&path, extension, files)?;
        } else if path.extension().is_some_and(|e| e == extension) {
            files.push(path);
        }
    }
    Ok(())
}

/// What tells two digitizers apart when a device is matched.
type Identity<'a> = (
    u16,
    u16,
    Option<usize>,
    Option<usize>,
    Option<&'a BTreeMap<u8, String>>,
);

/// Every problem of the definitions, one line each; empty when they are valid.
pub(crate) fn validate(tablets: &[(PathBuf, Tablet)], parsers: &BTreeSet<String>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut names: HashMap<&str, &Path> = HashMap::new();
    // Two different tablets that no field can tell apart would make the match ambiguous.
    let mut identities: HashMap<Identity<'_>, &str> = HashMap::new();

    for (path, tablet) in tablets {
        let at = path.display();
        if let Some(first) = names.insert(&tablet.name, path) {
            problems.push(format!(
                "{at}: the name \"{}\" is already used by {}",
                tablet.name,
                first.display()
            ));
        }
        let d = &tablet.specifications.digitizer;
        for (label, value) in [
            ("Width", d.width),
            ("Height", d.height),
            ("MaxX", d.max_x),
            ("MaxY", d.max_y),
        ] {
            if !(value.is_finite() && value > 0.0) {
                problems.push(format!(
                    "{at}: Digitizer.{label} must be positive, got {value}"
                ));
            }
        }
        if tablet.specifications.pen.max_pressure == 0 {
            problems.push(format!("{at}: Pen.MaxPressure must be positive"));
        }
        if tablet.digitizer_identifiers.is_empty() {
            problems.push(format!("{at}: DigitizerIdentifiers is empty"));
        }
        for id in &tablet.digitizer_identifiers {
            let who = format!("{at} ({:04x}:{:04x})", id.vendor_id, id.product_id);
            if id.input_report_length == Some(0) {
                problems.push(format!("{who}: InputReportLength must be positive"));
            }
            let class = id
                .report_parser
                .rsplit('.')
                .next()
                .unwrap_or(&id.report_parser);
            if !parsers.contains(class) {
                problems.push(format!(
                    "{who}: the parser \"{}\" is not defined by OpenTabletDriver",
                    id.report_parser
                ));
            }
            for report in id
                .output_init_report
                .iter()
                .chain(&id.feature_init_report)
                .flatten()
            {
                if base64::engine::general_purpose::STANDARD
                    .decode(report)
                    .is_err()
                {
                    problems.push(format!("{who}: \"{report}\" is not valid base64"));
                }
            }
            let identity = (
                id.vendor_id,
                id.product_id,
                id.input_report_length,
                id.output_report_length,
                id.device_strings.as_ref(),
            );
            if let Some(other) = identities.insert(identity, &tablet.name)
                && other != tablet.name
            {
                problems.push(format!(
                    "{who}: same VID:PID, report lengths and device strings as \"{other}\""
                ));
            }
        }
    }
    problems
}
