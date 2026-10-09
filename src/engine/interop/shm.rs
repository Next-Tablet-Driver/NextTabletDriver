//! # Shared State Segment
//!
//! Fixed-size, seqlock-protected shared memory segment through which the
//! current HID owner ([`super::lock`]) publishes live tablet state to every
//! other process reading the same tablet (another SDK-embedded game, or the
//! desktop app, whichever isn't the owner).
//!
//! The seqlock lets a single writer publish new snapshots lock-free while any
//! number of readers in other processes copy out a consistent snapshot
//! without ever blocking the writer: a reader retries whenever it detects the
//! writer was mid-publish, instead of taking a mutex.

use std::sync::atomic::{AtomicU32, Ordering};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as os;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as os;

/// Bumped whenever [`SdkPublicState`]'s layout changes. A reader whose
/// compiled-in version doesn't match the segment's refuses to interpret the
/// payload rather than risk misreading a field.
pub const SDK_ABI_VERSION: u32 = 1;

/// Fixed capacity of [`SdkPublicState::device_name`], in bytes (UTF-8,
/// truncated, not necessarily NUL-terminated).
pub const DEVICE_NAME_CAPACITY: usize = 64;

/// Snapshot of live tablet state and the config it was produced under.
///
/// Published by the HID owner for every reader to consume. Mirrors the
/// fields the SDK's public `NtdState` FFI struct exposes; this struct *is*
/// that layout's single source of truth, shared across the process boundary.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdkPublicState {
    pub is_connected: bool,
    /// Raw `TabletStatus` discriminant.
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
    /// UTF-8 device name, truncated to fit; only the first `device_name_len`
    /// bytes are meaningful.
    pub device_name: [u8; DEVICE_NAME_CAPACITY],
    pub device_name_len: u32,
    pub vid: u16,
    pub pid: u16,
    /// Raw `DriverMode` discriminant.
    pub mode: u8,
    pub active_area_x: f32,
    pub active_area_y: f32,
    pub active_area_w: f32,
    pub active_area_h: f32,
    pub active_area_rotation: f32,
    /// Incremented by the owner every time the config changes, so readers
    /// (and command-channel callers) can tell config writes apart from noise.
    pub config_version: u32,
}

impl Default for SdkPublicState {
    fn default() -> Self {
        Self {
            is_connected: false,
            status: 0,
            u: 0.0,
            v: 0.0,
            screen_x: 0.0,
            screen_y: 0.0,
            pressure: 0,
            tilt_x: 0,
            tilt_y: 0,
            buttons: 0,
            is_down: false,
            eraser: false,
            device_name: [0u8; DEVICE_NAME_CAPACITY],
            device_name_len: 0,
            vid: 0,
            pid: 0,
            mode: 0,
            active_area_x: 0.0,
            active_area_y: 0.0,
            active_area_w: 0.0,
            active_area_h: 0.0,
            active_area_rotation: 0.0,
            config_version: 0,
        }
    }
}

/// The actual layout placed in shared memory: an ABI tag, a seqlock sequence
/// counter (even = stable, odd = a writer is mid-publish), and the payload.
///
/// `payload` is wrapped in `UnsafeCell` because it is mutated through a
/// shared reference (every accessor only ever holds `&ShmSegment`, since the
/// segment is reached through a raw pointer into memory shared across
/// processes). The seqlock protocol below, not Rust's aliasing rules, is
/// what keeps that sound.
#[repr(C)]
struct ShmSegment {
    abi_version: AtomicU32,
    seq: AtomicU32,
    payload: std::cell::UnsafeCell<SdkPublicState>,
}

// SAFETY: `payload` is only ever mutated by the single current HID owner
// (mutual exclusion guaranteed by `super::lock`) and only ever read by
// others through the seqlock retry protocol in `read_from` below, so
// concurrent access across threads/processes never races on the same bytes
// without a synchronizing `seq` load/store around it.
unsafe impl Sync for ShmSegment {}

const fn segment_size() -> usize {
    std::mem::size_of::<ShmSegment>()
}

/// Writer handle held by the current HID owner. Publishes new snapshots into
/// the shared segment for every reader to see.
pub struct ShmWriter {
    mapping: os::Mapping,
}

impl ShmWriter {
    /// Creates (or takes over) the well-known shared segment and marks it
    /// with the current ABI version. Only the HID owner should hold one of
    /// these at a time; callers are expected to have already acquired
    /// [`super::lock::HidOwnerGuard`].
    #[must_use]
    pub fn create() -> Option<Self> {
        let mapping = os::create_mapping(segment_size())?;
        // SAFETY: `mapping` points to at least `segment_size()` freshly
        // mapped bytes, valid for `ShmSegment`'s layout (see module docs on
        // `ShmSegment` for why a zeroed/OS-provided bit pattern is valid).
        let segment = unsafe { &*mapping.as_ptr().cast::<ShmSegment>() };
        segment
            .abi_version
            .store(SDK_ABI_VERSION, Ordering::Release);
        Some(Self { mapping })
    }

    /// Publishes a new snapshot, visible to readers as soon as this returns.
    pub fn publish(&self, payload: &SdkPublicState) {
        // SAFETY: see `ShmSegment`'s doc comment; this writer is the sole
        // publisher for the lifetime of the mapping.
        let segment = unsafe { &*self.mapping.as_ptr().cast::<ShmSegment>() };
        segment.seq.fetch_add(1, Ordering::AcqRel);
        // SAFETY: exclusive-writer invariant above; readers never dereference
        // this pointer, they only ever copy through it after checking `seq`.
        unsafe {
            *segment.payload.get() = *payload;
        }
        segment.seq.fetch_add(1, Ordering::Release);
    }
}

/// Reader handle held by every process that isn't the current HID owner.
pub struct ShmReader {
    mapping: os::Mapping,
}

/// Bounds the seqlock retry loop so a reader can never spin forever; the
/// writer's critical section is a handful of field copies, so real
/// contention resolves in a couple of iterations at most.
const MAX_READ_RETRIES: u32 = 32;

impl ShmReader {
    /// Opens the well-known shared segment. Returns `None` if it doesn't
    /// exist yet (no owner has ever published) or its ABI version doesn't
    /// match this build.
    #[must_use]
    pub fn open() -> Option<Self> {
        let mapping = os::open_mapping(segment_size())?;
        let reader = Self { mapping };
        // SAFETY: see `ShmSegment`'s doc comment.
        let segment = unsafe { &*reader.mapping.as_ptr().cast::<ShmSegment>() };
        if segment.abi_version.load(Ordering::Acquire) != SDK_ABI_VERSION {
            return None;
        }
        Some(reader)
    }

    /// Reads the latest published snapshot, retrying internally if a writer
    /// is caught mid-publish. Returns `None` only under pathological
    /// contention that outlasts [`MAX_READ_RETRIES`] retries.
    #[must_use]
    pub fn read(&self) -> Option<SdkPublicState> {
        // SAFETY: see `ShmSegment`'s doc comment.
        let segment = unsafe { &*self.mapping.as_ptr().cast::<ShmSegment>() };
        for _ in 0..MAX_READ_RETRIES {
            let before = segment.seq.load(Ordering::Acquire);
            if before % 2 != 0 {
                continue;
            }
            // SAFETY: `payload` is `Copy`; this is a bytewise snapshot read
            // whose consistency is validated by the matching `seq` check
            // below, per the seqlock protocol.
            let snapshot = unsafe { *segment.payload.get() };
            let after = segment.seq.load(Ordering::Acquire);
            if before == after {
                return Some(snapshot);
            }
        }
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    // Both the round-trip check and the concurrency stress below share this
    // one test function rather than being split across two `#[test]`s: they
    // all target the same fixed, well-known segment name (matching the
    // plan's single machine-wide name per platform), and cargo runs tests in
    // the same binary in parallel by default: two tests independently
    // creating/opening that name would race each other's writer state.
    #[test]
    fn write_read_round_trip_and_no_torn_reads_under_concurrency() {
        let writer = Arc::new(ShmWriter::create().expect("writer should create the segment"));
        let reader = ShmReader::open().expect("reader should open the same segment");

        // Every counter-derived field set to the same value up front, so the
        // stress loop below (which checks that invariant on every read) stays
        // valid even for reads that land before the writer thread's first
        // publish.
        let initial = SdkPublicState {
            is_connected: true,
            u: 0.5,
            pressure: 4096,
            tilt_x: 4096,
            tilt_y: 4096,
            screen_x: 4096.0,
            screen_y: 4096.0,
            config_version: 4096,
            ..Default::default()
        };
        writer.publish(&initial);
        let read_back = reader.read().expect("read should succeed");
        assert_eq!(read_back, initial);

        let stop = Arc::new(AtomicBool::new(false));

        let writer_handle = {
            let writer = Arc::clone(&writer);
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut i: i32 = 0;
                while !stop.load(Ordering::Relaxed) {
                    // Every field derived from the same counter: any mix of
                    // fields from two different iterations is detectable.
                    let state = SdkPublicState {
                        is_connected: true,
                        pressure: i,
                        tilt_x: i,
                        tilt_y: i,
                        screen_x: i as f32,
                        screen_y: i as f32,
                        config_version: i as u32,
                        ..Default::default()
                    };
                    writer.publish(&state);
                    i = i.wrapping_add(1);
                }
            })
        };

        for _ in 0..20_000 {
            if let Some(snapshot) = reader.read() {
                let expected = snapshot.pressure;
                assert_eq!(snapshot.tilt_x, expected, "torn read detected");
                assert_eq!(snapshot.tilt_y, expected, "torn read detected");
                assert_eq!(snapshot.screen_x as i32, expected, "torn read detected");
                assert_eq!(snapshot.screen_y as i32, expected, "torn read detected");
                assert_eq!(
                    snapshot.config_version, expected as u32,
                    "torn read detected"
                );
            }
        }

        stop.store(true, Ordering::Relaxed);
        writer_handle
            .join()
            .expect("writer thread should not panic");
    }
}
