//! Shared by the parser fuzzers: one family of parsers fed with arbitrary bytes.

use ntd_tablets::config_loader::load_embedded_configurations;
use ntd_tablets::parsers::{ReportParser, create_parser};
use std::collections::BTreeSet;
use std::sync::OnceLock;

/// The parsers of `family` (a module of `ntd_tablets::parsers`): one for every parser class that a
/// tablet definition names and that `create_parser` maps into the family, plus `extra` classes for
/// a family that no definition uses today.
fn parsers(family: &str, extra: &[&str]) -> Vec<(String, Box<dyn ReportParser>)> {
    let marker = format!("::parsers::{family}::");
    let names: BTreeSet<String> = load_embedded_configurations()
        .into_iter()
        .flat_map(|config| config.digitizer_identifiers)
        .map(|digitizer| digitizer.report_parser)
        .chain(extra.iter().map(|name| (*name).to_string()))
        .collect();
    names
        .into_iter()
        .filter_map(|name| {
            let parser = create_parser(&name);
            parser.name().contains(&marker).then_some((name, parser))
        })
        .collect()
}

/// Feeds `data` to every parser of `family`. A parser must not panic on any bytes, and, as it
/// reads nothing but the report, a fresh parser must give the same answer to the same report.
///
/// The positions are not compared with the bounds of the definition: a pen out of range legally
/// reports values beyond them.
pub fn check(family: &'static str, extra: &'static [&'static str], data: &[u8]) {
    static CACHE: OnceLock<Vec<(String, Box<dyn ReportParser>)>> = OnceLock::new();
    let parsers = CACHE.get_or_init(|| parsers(family, extra));
    assert!(!parsers.is_empty(), "no parser in the family {family}");

    for (name, _) in parsers {
        // A fresh parser each time: some families keep state between reports.
        let first = create_parser(name).parse(data);
        let second = create_parser(name).parse(data);
        match (first, second) {
            (None, None) => {}
            (Some(a), Some(b)) => assert!(
                (a.status, a.x, a.y, a.pressure, a.tilt_x, a.tilt_y, a.buttons)
                    == (b.status, b.x, b.y, b.pressure, b.tilt_x, b.tilt_y, b.buttons),
                "{name} is not deterministic on {data:02x?}"
            ),
            _ => panic!("{name} is not deterministic on {data:02x?}"),
        }
    }
}
