//! # `ntd-test-support`
//!
//! The fakes that let a test run the engine, the pipeline or a driver without a tablet, without
//! injecting real input and without waiting for real time (rule 72 of the plan). A crate adds it as
//! a **dev-dependency** only:
//!
//! ```toml
//! [dev-dependencies]
//! ntd-test-support = { workspace = true }
//! ```
//!
//! - [`FakeHidBackend`] implements `ntd_core::hid::HidBackend` over devices that the test plugs,
//!   feeds with reports and unplugs;
//! - [`RecordingSink`] implements `ntd_core::output::OutputSink` and keeps what would have been
//!   injected;
//! - [`ManualClock`] implements `ntd_core::clock::Clock` and moves only when told to.
//!
//! None of them sleeps, spawns a thread or touches the operating system.

mod clock;
mod hid;
mod output;

pub use clock::ManualClock;
pub use hid::{FakeHidBackend, FakeHidHandle, fake_tablet_info};
pub use output::{OutputEvent, RecordingSink};
