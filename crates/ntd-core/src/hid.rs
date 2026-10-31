//! The port through which the engine talks to HID devices.
//!
//! The traits only describe what the engine needs; the crate that owns `hidapi` implements them
//! for the real hardware, and `ntd-test-support` implements them for tests that have no tablet.

use std::time::Duration;

/// What the system tells about a HID interface before it is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidDeviceInfo {
    /// USB `VendorID`.
    pub vendor_id: u16,
    /// USB `ProductID`.
    pub product_id: u16,
    /// The interface number, or -1 when the system does not tell.
    pub interface_number: i32,
    /// HID usage page of the top-level collection.
    pub usage_page: u16,
    /// HID usage of the top-level collection.
    pub usage: u16,
    /// The system path that identifies the interface; opaque, only the backend reads it.
    pub path: String,
    /// The manufacturer string, when the device has one.
    pub manufacturer: Option<String>,
    /// The product string, when the device has one.
    pub product: Option<String>,
}

/// How to open a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpenOptions {
    /// Keep the system driver from also using the device (needed on macOS so the tablet does not
    /// move the cursor twice). Backends that cannot do it ignore the option.
    pub exclusive: bool,
}

/// Why a HID call failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HidError {
    /// No device matches: it was never there, or it is gone.
    #[error("no such HID device")]
    NotFound,
    /// The system refuses access (udev rules, Input Monitoring permission, another owner).
    #[error("permission denied")]
    PermissionDenied,
    /// The device was unplugged while it was open.
    #[error("the device was disconnected")]
    Disconnected,
    /// This platform cannot do it.
    #[error("not supported on this platform")]
    Unsupported,
    /// Any other failure of the backend, with its own message.
    #[error("{0}")]
    Other(String),
}

/// An open HID interface. It is used from the polling thread only, but it moves there from the
/// thread that opened it.
pub trait HidDevice: Send {
    /// Reads one input report, waiting at most `timeout`. Returns the number of bytes written to
    /// `buffer`; 0 means that nothing arrived in time.
    ///
    /// # Errors
    /// [`HidError::Disconnected`] when the device is gone, or the error of the backend.
    fn read_timeout(&self, buffer: &mut [u8], timeout: Duration) -> Result<usize, HidError>;

    /// Writes one output report (the first byte is the report ID, 0 when there is none).
    ///
    /// # Errors
    /// The error of the backend.
    fn write(&self, data: &[u8]) -> Result<usize, HidError>;

    /// Sends one feature report (the first byte is the report ID).
    ///
    /// # Errors
    /// The error of the backend.
    fn send_feature_report(&self, data: &[u8]) -> Result<(), HidError>;

    /// Reads the string descriptor at `index`, `None` when the device has none there.
    ///
    /// # Errors
    /// The error of the backend.
    fn indexed_string(&self, index: u8) -> Result<Option<String>, HidError>;
}

/// The HID subsystem: finds interfaces and opens them.
pub trait HidBackend: Send {
    /// Lists every HID interface the system shows right now.
    ///
    /// # Errors
    /// The error of the backend.
    fn enumerate(&mut self) -> Result<Vec<HidDeviceInfo>, HidError>;

    /// Opens the interface described by `info`.
    ///
    /// # Errors
    /// [`HidError::NotFound`] when it is gone, [`HidError::PermissionDenied`] when the system
    /// refuses, or the error of the backend.
    fn open(
        &mut self,
        info: &HidDeviceInfo,
        options: OpenOptions,
    ) -> Result<Box<dyn HidDevice>, HidError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_traits_are_object_safe() {
        fn takes(_: Option<Box<dyn HidBackend>>, _: Option<Box<dyn HidDevice>>) {}
        takes(None, None);
    }

    #[test]
    fn a_device_is_opened_shared_by_default() {
        assert!(!OpenOptions::default().exclusive);
    }

    #[test]
    fn errors_read_as_sentences() {
        assert_eq!(HidError::PermissionDenied.to_string(), "permission denied");
        assert_eq!(HidError::Other("busy".into()).to_string(), "busy");
    }
}
