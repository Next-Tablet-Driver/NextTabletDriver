#![no_main]

mod parser_common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| parser_common::check(
    "tenmoon",
    // No definition of OpenTabletDriver names this parser yet.
    &["OpenTabletDriver.Configurations.Parsers.10moon.10moonReportParser"],
    data,
));
