#![no_main]

mod parser_common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| parser_common::check("floogoo", &[], data));
