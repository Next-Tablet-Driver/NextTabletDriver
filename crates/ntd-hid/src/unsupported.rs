//! Any other system: not implemented. Every call says [`HidError::Unsupported`], and the crate
//! still compiles for the target.

use ntd_core::hid::{HidBackend, HidDevice, HidDeviceInfo, HidError, OpenOptions};

/// The HID backend of this system: none.
pub struct Backend;

impl Backend {
    /// Always fails: there is no HID backend for this system.
    ///
    /// # Errors
    /// Always [`HidError::Unsupported`].
    pub const fn new() -> Result<Self, HidError> {
        Err(HidError::Unsupported)
    }
}

impl HidBackend for Backend {
    fn enumerate(&mut self) -> Result<Vec<HidDeviceInfo>, HidError> {
        Err(HidError::Unsupported)
    }

    fn open(
        &mut self,
        _info: &HidDeviceInfo,
        _options: OpenOptions,
    ) -> Result<Box<dyn HidDevice>, HidError> {
        Err(HidError::Unsupported)
    }
}
