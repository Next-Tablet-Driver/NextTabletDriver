//! Linux: `hidapi` over `hidraw`.

use crate::hidapi_backend::HidApiBackend;
use ntd_core::hid::{HidBackend, HidDevice, HidDeviceInfo, HidError, OpenOptions};

/// The HID backend of this system.
pub struct Backend {
    inner: HidApiBackend,
}

impl Backend {
    /// Initialises the HID subsystem.
    ///
    /// # Errors
    /// The error of `hidapi`.
    pub fn new() -> Result<Self, HidError> {
        HidApiBackend::new().map(|inner| Self { inner })
    }
}

impl HidBackend for Backend {
    fn enumerate(&mut self) -> Result<Vec<HidDeviceInfo>, HidError> {
        self.inner.enumerate()
    }

    fn open(
        &mut self,
        info: &HidDeviceInfo,
        options: OpenOptions,
    ) -> Result<Box<dyn HidDevice>, HidError> {
        self.inner.open(info, options)
    }
}
