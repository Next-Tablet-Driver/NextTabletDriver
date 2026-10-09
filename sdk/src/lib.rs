//! # `NextTabletDriver` SDK
//!
//! Native, redistributable `cdylib`/`staticlib` that embeds and runs its own
//! instance of the `NextTabletDriver` engine directly inside the host
//! process (a game, a Blender plugin, ...). Consumers do not need the
//! desktop `NextTabletDriver` application installed or running.

mod engine_loop;
mod ffi;
mod logging;

pub use ffi::{
    NTD_ERR_COMMAND_FAILED, NTD_ERR_HID_INIT_FAILED, NTD_ERR_INVALID_ARGUMENT,
    NTD_ERR_NOT_INITIALIZED, NTD_ERR_NULL_POINTER, NTD_ERR_PANIC, NTD_OK, NTD_SDK_ABI_VERSION,
    NtdState,
};
