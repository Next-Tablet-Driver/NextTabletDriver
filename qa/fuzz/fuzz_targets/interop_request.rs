#![no_main]

use libfuzzer_sys::fuzz_target;
use ntd_interop::protocol::{REQUEST_SIZE, Request};

// Any bytes a peer sends must decode to a request or to nothing, never panic; and a request that
// decodes must encode to bytes that decode to the same request.
fuzz_target!(|data: &[u8]| {
    let mut buffer = [0_u8; REQUEST_SIZE];
    let len = data.len().min(REQUEST_SIZE);
    if let (Some(to), Some(from)) = (buffer.get_mut(..len), data.get(..len)) {
        to.copy_from_slice(from);
    }
    if let Some(request) = Request::decode(buffer) {
        assert_eq!(Request::decode(request.encode()), Some(request));
    }
});
