//! Golden-file tests for the report parsers.
//!
//! Each folder of `qa/golden/` holds a real capture of one tablet (`capture.ntdcap`, written by
//! `tools/ntd-recorder`) and the parsed result committed as `snapshot.txt`. The test replays the
//! capture through the parser the tablet's configuration selects and compares every line with the
//! snapshot, so any change of a parser's output, even by one byte, fails.
//!
//! After a deliberate change, regenerate the snapshots with `UPDATE_GOLDEN=1 cargo test --test
//! golden` and review the diff.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that panics is a failing test"
)]

use next_tablet_driver::drivers::TabletData;
use next_tablet_driver::drivers::config_loader::INDEXED_CONFIGS;
use next_tablet_driver::drivers::parsers::{ReportParser, create_parser};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Highest `.ntdcap` layout version this test understands.
const SUPPORTED_FORMAT_VERSION: u32 = 1;

#[derive(Deserialize)]
struct Capture {
    format: String,
    format_version: u32,
    device: Device,
    steps: Vec<Step>,
    reports: Vec<Report>,
}

#[derive(Deserialize)]
struct Device {
    vendor_id: u16,
    product_id: u16,
}

#[derive(Deserialize)]
struct Step {
    id: String,
}

#[derive(Deserialize, Clone)]
struct Report {
    interface: usize,
    step: usize,
    data_hex: String,
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("qa")
        .join("golden")
}

fn from_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn load(dir: &Path) -> Capture {
    let text = std::fs::read_to_string(dir.join("capture.ntdcap")).unwrap();
    let capture: Capture = serde_json::from_str(&text).unwrap();
    assert_eq!(capture.format, "ntdcap", "{}: not a capture", dir.display());
    assert!(
        capture.format_version <= SUPPORTED_FORMAT_VERSION,
        "{}: capture format {} is newer than this test",
        dir.display(),
        capture.format_version
    );
    capture
}

/// The parser class the tablet's configuration names for this device and report length.
fn parser_name(capture: &Capture) -> String {
    let candidates = INDEXED_CONFIGS
        .get(&(capture.device.vendor_id, capture.device.product_id))
        .unwrap_or_else(|| {
            panic!(
                "no configuration for {:04x}:{:04x}",
                capture.device.vendor_id, capture.device.product_id
            )
        });
    let len = capture.reports.first().map(|r| r.data_hex.len() / 2);
    let chosen = candidates
        .iter()
        .find(|(_, d)| d.input_report_length.is_some() && d.input_report_length == len)
        .or_else(|| candidates.first())
        .unwrap();
    chosen.1.report_parser.clone()
}

/// One line per parsed field. `receive_time` and `parser_time` are left out: they change at every
/// run and say nothing about the parser's logic.
fn render(data: &TabletData) -> String {
    format!(
        "status={} x={} y={} pressure={} tilt=({},{}) buttons={:#04x} eraser={} hover={} connected={}",
        data.status,
        data.x,
        data.y,
        data.pressure,
        data.tilt_x,
        data.tilt_y,
        data.buttons,
        data.eraser,
        data.hover_distance,
        data.is_connected
    )
}

/// Replays `reports` in order through one parser per interface and renders the result.
fn replay(parser_class: &str, steps: &[Step], reports: &[Report]) -> String {
    let mut parsers: HashMap<usize, Box<dyn ReportParser>> = HashMap::new();
    let mut out = format!("# parser: {parser_class}\n");
    for (i, r) in reports.iter().enumerate() {
        let parser = parsers
            .entry(r.interface)
            .or_insert_with(|| create_parser(parser_class));
        let bytes = from_hex(&r.data_hex);
        let result = parser
            .parse(&bytes)
            .map_or_else(|| "None".to_owned(), |d| render(&d));
        writeln!(
            out,
            "{i} if{} {} {} -> {result}",
            r.interface, steps[r.step].id, r.data_hex
        )
        .unwrap();
    }
    out
}

fn tablet_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join("capture.ntdcap").is_file())
        .collect();
    dirs.sort();
    dirs
}

#[test]
fn every_capture_parses_to_its_committed_snapshot() {
    let dirs = tablet_dirs();
    assert!(!dirs.is_empty(), "qa/golden has no capture");
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();

    for dir in dirs {
        let capture = load(&dir);
        let actual = replay(&parser_name(&capture), &capture.steps, &capture.reports);
        let snapshot = dir.join("snapshot.txt");
        if update {
            std::fs::write(&snapshot, &actual).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&snapshot).unwrap_or_else(|_| {
            panic!(
                "{} is missing; run with UPDATE_GOLDEN=1 to create it",
                snapshot.display()
            )
        });
        // Compare line by line so a failure names the first report that changed.
        let (exp, act): (Vec<&str>, Vec<&str>) =
            (expected.lines().collect(), actual.lines().collect());
        assert_eq!(
            exp.len(),
            act.len(),
            "{}: the number of reports changed",
            dir.display()
        );
        for (e, a) in exp.iter().zip(&act) {
            assert_eq!(e, a, "{}: the parser output changed", dir.display());
        }
    }
}

#[test]
fn changing_one_byte_of_a_report_changes_the_output() {
    let dir = tablet_dirs().into_iter().next().unwrap();
    let capture = load(&dir);
    let class = parser_name(&capture);
    let expected = std::fs::read_to_string(dir.join("snapshot.txt")).unwrap();

    // Pick a report the parser accepts and change its X coordinate (byte 2 of the report).
    let mut reports = capture.reports.clone();
    let target = reports
        .iter()
        .position(|r| {
            let bytes = from_hex(&r.data_hex);
            create_parser(&class).parse(&bytes).is_some()
        })
        .unwrap();
    let mut bytes = from_hex(&reports[target].data_hex);
    bytes[2] ^= 0x01;
    reports[target].data_hex = bytes.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    });

    // Compare only what the parser produced (after `->`): the raw bytes differ by construction.
    let parsed = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|l| l.split_once("-> ").map(|(_, r)| r.to_owned()))
            .collect()
    };
    assert_ne!(
        parsed(&replay(&class, &capture.steps, &reports)),
        parsed(&expected)
    );
}
