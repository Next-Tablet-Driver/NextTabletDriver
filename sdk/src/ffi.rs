//! # C ABI
//!
//! The SDK's public surface: six `extern "C"` functions plus the
//! [`NtdState`] snapshot struct, meant to be consumed through
//! `include/ntd_sdk.h` (C/C++) or the generated C# bindings (Unity and
//! other .NET hosts).
//!
//! **Every** exported function is wrapped in [`panic::catch_unwind`].
//! `sdk/Cargo.toml` sets `panic = "unwind"` specifically so a panic can
//! never cross this boundary into the host process. None of the code in
//! this module uses `unwrap`/`expect`/`panic!`/slice indexing (denied by
//! `sdk/Cargo.toml`'s lint table, same as the root crate's).

use crate::engine_loop;
use next_tablet_driver::core::config::models::{ActiveArea, DriverMode};
use next_tablet_driver::engine::interop::command::{Request, Response, send_command};
use next_tablet_driver::engine::interop::lock::try_acquire_hid_owner;
use next_tablet_driver::engine::interop::shm::DEVICE_NAME_CAPACITY;
use next_tablet_driver::engine::state::{LockRecoveryExt, SharedState};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Bumped whenever this FFI surface changes in a backward-incompatible way.
///
/// Independent from [`next_tablet_driver::engine::interop::shm::SDK_ABI_VERSION`],
/// which versions the separate inter-process shared-memory layout.
pub const NTD_SDK_ABI_VERSION: u32 = 1;

pub const NTD_OK: i32 = 0;
/// `ntd_init` only: the HID API itself failed to initialise (no supported
/// backend, missing permissions, ...).
pub const NTD_ERR_HID_INIT_FAILED: i32 = -1;
/// `ntd_init` was never called, or `ntd_shutdown` already ran.
pub const NTD_ERR_NOT_INITIALIZED: i32 = -2;
/// A required output pointer was null.
pub const NTD_ERR_NULL_POINTER: i32 = -3;
/// An argument was outside its valid range (e.g. an unknown mode byte).
pub const NTD_ERR_INVALID_ARGUMENT: i32 = -4;
/// The current HID owner couldn't be reached to apply a forwarded command.
pub const NTD_ERR_COMMAND_FAILED: i32 = -5;
/// The call panicked; caught at the FFI boundary and never propagated to
/// the host.
pub const NTD_ERR_PANIC: i32 = -6;

/// Fixed capacity of [`NtdState::device_name`], in bytes.
///
/// Kept as a local literal (rather than referencing
/// [`next_tablet_driver::engine::interop::shm::DEVICE_NAME_CAPACITY`]
/// directly) so it can be emitted as a `#define` in the generated C header.
/// [`NtdState::device_name`] itself uses the literal `64` rather than this
/// constant, since `csbindgen`, unlike `cbindgen`, can't resolve a named
/// constant into a fixed-size C# buffer length. The `const _` assertions
/// below keep all three (this constant, the real capacity, and the struct's
/// array literal) from silently drifting apart.
pub const NTD_DEVICE_NAME_CAPACITY: usize = 64;

const _: () = assert!(
    NTD_DEVICE_NAME_CAPACITY == DEVICE_NAME_CAPACITY,
    "NTD_DEVICE_NAME_CAPACITY must match engine::interop::shm::DEVICE_NAME_CAPACITY"
);

/// Live tablet + config snapshot returned by [`ntd_poll_state`].
///
/// Field-for-field mirror of [`next_tablet_driver::engine::interop::shm::SdkPublicState`].
/// The two layouts are asserted equal in size below, since both cross an
/// FFI/ABI boundary (this one to the host process, that one to other
/// processes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NtdState {
    pub is_connected: bool,
    pub status: u8,
    pub u: f32,
    pub v: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub pressure: i32,
    pub tilt_x: i32,
    pub tilt_y: i32,
    pub buttons: u8,
    pub is_down: bool,
    pub eraser: bool,
    pub device_name: [u8; 64],
    pub device_name_len: u32,
    pub vid: u16,
    pub pid: u16,
    pub mode: u8,
    pub active_area_x: f32,
    pub active_area_y: f32,
    pub active_area_w: f32,
    pub active_area_h: f32,
    pub active_area_rotation: f32,
    pub config_version: u32,
}

const _: () = assert!(
    size_of::<[u8; 64]>() == NTD_DEVICE_NAME_CAPACITY,
    "NtdState::device_name's array literal must match NTD_DEVICE_NAME_CAPACITY"
);

const _: () = assert!(
    size_of::<NtdState>() == size_of::<next_tablet_driver::engine::interop::shm::SdkPublicState>(),
    "NtdState must stay layout-compatible with SdkPublicState"
);

struct EngineHandle {
    shared: Arc<SharedState>,
    is_owner: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

fn engine_slot() -> &'static Mutex<Option<EngineHandle>> {
    static ENGINE: OnceLock<Mutex<Option<EngineHandle>>> = OnceLock::new();
    ENGINE.get_or_init(|| Mutex::new(None))
}

fn lock_slot() -> MutexGuard<'static, Option<EngineHandle>> {
    match engine_slot().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Runs `f` and converts an unwinding panic into [`NTD_ERR_PANIC`] instead of
/// letting it cross into the host process, which is undefined behaviour for
/// an FFI boundary.
fn ffi_guard<F: FnOnce() -> i32>(f: F) -> i32 {
    panic::catch_unwind(AssertUnwindSafe(f)).unwrap_or(NTD_ERR_PANIC)
}

/// `ntd_init` returns as soon as the background thread is *spawned*, not
/// once it has determined whether this process is the HID owner or a reader
/// (that requires the thread to actually get scheduled and acquire the named
/// lock -- typically low single-digit milliseconds, but not zero). A caller
/// that calls `ntd_set_mode`/`ntd_set_active_area` immediately after
/// `ntd_init`, exactly as this SDK's own examples and docs do, can otherwise
/// race a still-`false` `is_owner` flag: this process is about to become the
/// owner, but isn't yet, so the write falls into the reader path and tries
/// to forward to an owner that doesn't exist yet, failing with
/// [`NTD_ERR_COMMAND_FAILED`]. Retry both the owner-flag check and the
/// reader-side forward for up to this long before giving up for real.
const ROLE_RESOLUTION_TIMEOUT: Duration = Duration::from_secs(1);
const ROLE_RESOLUTION_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Applies `request` locally if this process is (or, within the startup
/// grace period, becomes) the HID owner; otherwise forwards it to whichever
/// process is, retrying through transient "no owner listening yet" gaps.
fn apply_or_forward(
    shared: &Arc<SharedState>,
    is_owner: &AtomicBool,
    request: Request,
    apply_local: impl Fn(&Arc<SharedState>),
) -> i32 {
    let deadline = Instant::now() + ROLE_RESOLUTION_TIMEOUT;
    loop {
        if is_owner.load(Ordering::Relaxed) {
            apply_local(shared);
            return NTD_OK;
        }

        match send_command(request) {
            Ok(Response::Ok) => return NTD_OK,
            Err(_) if Instant::now() < deadline => thread::sleep(ROLE_RESOLUTION_POLL_INTERVAL),
            Ok(Response::Rejected) | Err(_) => return NTD_ERR_COMMAND_FAILED,
        }
    }
}

const fn decode_mode(byte: u8) -> Option<DriverMode> {
    match byte {
        0 => Some(DriverMode::Absolute),
        1 => Some(DriverMode::Relative),
        _ => None,
    }
}

/// Returns this build's FFI ABI version, so a host can detect a mismatch
/// against the header/bindings it was compiled with.
#[unsafe(no_mangle)]
pub extern "C" fn ntd_sdk_abi_version() -> u32 {
    panic::catch_unwind(|| NTD_SDK_ABI_VERSION).unwrap_or(0)
}

/// Starts the embedded engine: becomes the HID owner if no other process
/// currently is one, otherwise starts in reader mode and mirrors the real
/// owner's state. Idempotent: calling this again while already initialised
/// is a no-op that returns [`NTD_OK`].
#[unsafe(no_mangle)]
pub extern "C" fn ntd_init() -> i32 {
    ffi_guard(|| {
        crate::logging::init();

        let mut slot = lock_slot();
        if slot.is_some() {
            return NTD_OK;
        }

        // Synchronous feasibility probe: if we're about to become the HID
        // owner, validate `HidApi::new()` succeeds on this system *before*
        // returning, so a caller sees a broken HID backend immediately
        // instead of only discovering it later by polling `NtdState`. The
        // background thread performs its own `HidApi::new()` once it
        // starts (needed regardless, since every automatic retry after a
        // crash re-initialises from scratch) rather than reusing this one.
        let hid_owner = try_acquire_hid_owner();
        if hid_owner.is_some()
            && let Err(e) = hidapi::HidApi::new()
        {
            log::error!(target: "Ffi", "CRITICAL: Failed to initialise HID API: {e}");
            return NTD_ERR_HID_INIT_FAILED;
        }
        drop(hid_owner);

        let shared = Arc::new(SharedState::new());
        let is_owner = Arc::new(AtomicBool::new(false));
        let thread = engine_loop::spawn(Arc::clone(&shared), Arc::clone(&is_owner));
        *slot = Some(EngineHandle {
            shared,
            is_owner,
            thread,
        });
        NTD_OK
    })
}

/// Stops the embedded engine and joins its background thread. Safe to call
/// even if [`ntd_init`] was never called, or was already shut down.
#[unsafe(no_mangle)]
pub extern "C" fn ntd_shutdown() {
    let _: i32 = ffi_guard(|| {
        let handle = lock_slot().take();
        if let Some(handle) = handle {
            handle
                .shared
                .shutdown_requested
                .store(true, Ordering::Relaxed);
            let _ = handle.thread.join();
        }
        NTD_OK
    });
}

/// Copies the current tablet + config snapshot into `*out_state`.
///
/// # Safety
///
/// `out_state` must be a non-null, valid, properly aligned pointer to a
/// writable `NtdState`, valid for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ntd_poll_state(out_state: *mut NtdState) -> i32 {
    ffi_guard(|| {
        if out_state.is_null() {
            return NTD_ERR_NULL_POINTER;
        }

        let slot = lock_slot();
        let Some(handle) = slot.as_ref() else {
            return NTD_ERR_NOT_INITIALIZED;
        };
        let shared = Arc::clone(&handle.shared);
        drop(slot);
        let state = build_ntd_state(&shared);

        // SAFETY: caller guarantees `out_state` is non-null and valid for a
        // writable `NtdState`, per this function's safety doc.
        unsafe {
            *out_state = state;
        }
        NTD_OK
    })
}

fn build_ntd_state(shared: &Arc<SharedState>) -> NtdState {
    let device = shared.device_state.read().unwrap_or_log("device_state");
    let mut device_name = [0u8; DEVICE_NAME_CAPACITY];
    let name_bytes = device.name.as_bytes();
    let name_len = name_bytes.len().min(device_name.len());
    if let (Some(dest), Some(src)) = (device_name.get_mut(..name_len), name_bytes.get(..name_len)) {
        dest.copy_from_slice(src);
    }
    let vid = device.vid;
    let pid = device.pid;
    drop(device);

    let data = shared.tablet_data.read().unwrap_or_log("tablet_data");
    let is_connected = data.is_connected;
    let status = data.status as u8;
    let buttons = data.buttons;
    let eraser = data.eraser;
    drop(data);

    let frame = shared
        .processed_frame
        .read()
        .unwrap_or_log("processed_frame");
    let (u, v, screen_x, screen_y, pressure, tilt_x, tilt_y, is_down) = (
        frame.u,
        frame.v,
        frame.screen_x,
        frame.screen_y,
        frame.pressure,
        frame.tilt_x,
        frame.tilt_y,
        frame.is_down,
    );
    drop(frame);

    let config = shared.config.read().unwrap_or_log("config");
    let mode = match config.mode {
        DriverMode::Absolute => 0,
        DriverMode::Relative => 1,
    };
    let active_area = config.active_area;
    drop(config);

    NtdState {
        is_connected,
        status,
        u,
        v,
        screen_x,
        screen_y,
        pressure,
        tilt_x,
        tilt_y,
        buttons,
        is_down,
        eraser,
        device_name,
        device_name_len: name_len as u32,
        vid,
        pid,
        mode,
        active_area_x: active_area.x,
        active_area_y: active_area.y,
        active_area_w: active_area.w,
        active_area_h: active_area.h,
        active_area_rotation: active_area.rotation,
        config_version: shared.config_version.load(Ordering::Relaxed),
    }
}

/// Sets the driver mode (`0` = absolute, `1` = relative). Writes directly if
/// this process is the current HID owner, otherwise forwards the change to
/// whichever process is.
#[unsafe(no_mangle)]
pub extern "C" fn ntd_set_mode(mode: u8) -> i32 {
    ffi_guard(|| {
        let Some(driver_mode) = decode_mode(mode) else {
            return NTD_ERR_INVALID_ARGUMENT;
        };

        let slot = lock_slot();
        let Some(handle) = slot.as_ref() else {
            return NTD_ERR_NOT_INITIALIZED;
        };
        let shared = Arc::clone(&handle.shared);
        let is_owner = Arc::clone(&handle.is_owner);
        drop(slot);

        apply_or_forward(
            &shared,
            &is_owner,
            Request::SetMode(driver_mode),
            |shared| {
                engine_loop::apply_set_mode(shared, driver_mode);
            },
        )
    })
}

/// Sets the active mapping area (millimeters, clamped to the current
/// device's physical surface). Writes directly if this process is the
/// current HID owner, otherwise forwards the change to whichever process is.
#[unsafe(no_mangle)]
pub extern "C" fn ntd_set_active_area(x: f32, y: f32, w: f32, h: f32, rotation: f32) -> i32 {
    ffi_guard(|| {
        let area = ActiveArea {
            x,
            y,
            w,
            h,
            rotation,
        };

        let slot = lock_slot();
        let Some(handle) = slot.as_ref() else {
            return NTD_ERR_NOT_INITIALIZED;
        };
        let shared = Arc::clone(&handle.shared);
        let is_owner = Arc::clone(&handle.is_owner);
        drop(slot);

        apply_or_forward(&shared, &is_owner, Request::SetActiveArea(area), |shared| {
            engine_loop::apply_set_active_area(shared, area);
        })
    })
}
