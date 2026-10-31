#![no_main]

use libfuzzer_sys::fuzz_target;
use ntd_interop::protocol::{RESPONSE_SIZE, Response};

// Any byte the owner sends back must decode to a response or to nothing, never panic; and a
// response that decodes must encode to bytes that decode to the same response.
fuzz_target!(|data: &[u8]| {
    let mut buffer = [0_u8; RESPONSE_SIZE];
    let len = data.len().min(RESPONSE_SIZE);
    if let (Some(to), Some(from)) = (buffer.get_mut(..len), data.get(..len)) {
        to.copy_from_slice(from);
    }
    if let Some(response) = Response::decode(buffer) {
        assert_eq!(Response::decode(response.encode()), Some(response));
    }
});
