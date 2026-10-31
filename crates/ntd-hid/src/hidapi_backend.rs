//! The `hidapi` implementation of the HID port, shared by the systems that have one.

use ntd_core::hid::{HidBackend, HidDevice, HidDeviceInfo, HidError, OpenOptions};
use std::ffi::CString;
use std::time::Duration;

/// Size of the buffer for a HID report descriptor (the specification caps it at 4096 bytes).
const DESCRIPTOR_BUFFER: usize = 4096;

/// Turns an error of `hidapi` into the error of the port. `hidapi` has no code for a refusal, so
/// the message is read, as the detection always did.
fn convert(error: &hidapi::HidError) -> HidError {
    if let hidapi::HidError::IoError { error } = error
        && error.kind() == std::io::ErrorKind::PermissionDenied
    {
        return HidError::PermissionDenied;
    }
    let message = error.to_string();
    let lower = message.to_lowercase();
    if lower.contains("permission") || lower.contains("denied") || lower.contains("access") {
        HidError::PermissionDenied
    } else {
        HidError::Other(message)
    }
}

fn describe(info: &hidapi::DeviceInfo) -> HidDeviceInfo {
    HidDeviceInfo {
        vendor_id: info.vendor_id(),
        product_id: info.product_id(),
        interface_number: info.interface_number(),
        usage_page: info.usage_page(),
        usage: info.usage(),
        path: info.path().to_string_lossy().into_owned(),
        manufacturer: info.manufacturer_string().map(str::to_owned),
        product: info.product_string().map(str::to_owned),
        serial_number: info.serial_number().map(str::to_owned),
        release_number: info.release_number(),
    }
}

/// The HID subsystem, through `hidapi`.
pub(crate) struct HidApiBackend {
    api: hidapi::HidApi,
}

impl HidApiBackend {
    /// Initialises `hidapi`.
    ///
    /// # Errors
    /// The error of `hidapi`, for example when the HID subsystem of the system is unusable.
    pub(crate) fn new() -> Result<Self, HidError> {
        hidapi::HidApi::new()
            .map(|api| Self { api })
            .map_err(|e| convert(&e))
    }
}

impl HidBackend for HidApiBackend {
    /// Scans the bus again each time: a tablet plugged after the start shows up.
    fn enumerate(&mut self) -> Result<Vec<HidDeviceInfo>, HidError> {
        self.api.refresh_devices().map_err(|e| convert(&e))?;
        Ok(self.api.device_list().map(describe).collect())
    }

    /// `hidapi` opens the interface shared; `options.exclusive` only matters on macOS, which this
    /// backend does not serve.
    fn open(
        &mut self,
        info: &HidDeviceInfo,
        _options: OpenOptions,
    ) -> Result<Box<dyn HidDevice>, HidError> {
        let path = CString::new(info.path.as_str()).map_err(|_| HidError::NotFound)?;
        let device = self.api.open_path(&path).map_err(|e| convert(&e))?;
        Ok(Box::new(HidApiDevice(device)))
    }
}

/// An interface opened through `hidapi`.
struct HidApiDevice(hidapi::HidDevice);

impl HidDevice for HidApiDevice {
    fn read_timeout(&self, buffer: &mut [u8], timeout: Duration) -> Result<usize, HidError> {
        let millis = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
        self.0.read_timeout(buffer, millis).map_err(|e| convert(&e))
    }

    fn write(&self, data: &[u8]) -> Result<usize, HidError> {
        self.0.write(data).map_err(|e| convert(&e))
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<(), HidError> {
        self.0.send_feature_report(data).map_err(|e| convert(&e))
    }

    fn indexed_string(&self, index: u8) -> Result<Option<String>, HidError> {
        self.0
            .get_indexed_string(i32::from(index))
            .map_err(|e| convert(&e))
    }

    fn report_descriptor(&self) -> Result<Vec<u8>, HidError> {
        let mut buffer = vec![0_u8; DESCRIPTOR_BUFFER];
        let len = self
            .0
            .get_report_descriptor(&mut buffer)
            .map_err(|e| convert(&e))?;
        buffer.truncate(len);
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_of_the_system_is_a_permission_error() {
        let refused = hidapi::HidError::HidApiError {
            message: "hid_open_path failed: Permission denied".to_string(),
        };
        assert_eq!(convert(&refused), HidError::PermissionDenied);
        let io = hidapi::HidError::IoError {
            error: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        };
        assert_eq!(convert(&io), HidError::PermissionDenied);
    }

    #[test]
    fn any_other_failure_keeps_its_message() {
        let other = hidapi::HidError::HidApiError {
            message: "device not found".to_string(),
        };
        assert_eq!(
            convert(&other),
            HidError::Other("hidapi error: device not found".to_string())
        );
    }

    #[test]
    fn a_path_with_a_nul_byte_cannot_be_opened() {
        let Ok(mut backend) = HidApiBackend::new() else {
            // No HID subsystem on this machine (a container, for example): nothing to open.
            return;
        };
        let info = HidDeviceInfo {
            path: "bad\0path".to_string(),
            ..ntd_test_support::fake_tablet_info(1, 2)
        };
        assert!(matches!(
            backend.open(&info, OpenOptions::default()),
            Err(HidError::NotFound)
        ));
    }
}
