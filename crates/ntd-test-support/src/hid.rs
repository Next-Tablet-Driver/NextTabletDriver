use ntd_core::hid::{HidBackend, HidDevice, HidDeviceInfo, HidError, OpenOptions};
use ntd_core::sync::LockRecoveryExt;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The description of a fake tablet interface, with a path made from its ids.
#[must_use]
pub fn fake_tablet_info(vendor_id: u16, product_id: u16) -> HidDeviceInfo {
    HidDeviceInfo {
        vendor_id,
        product_id,
        interface_number: 0,
        usage_page: 0xFF0D,
        usage: 1,
        path: format!("fake:{vendor_id:04x}:{product_id:04x}"),
        manufacturer: Some("Fake".to_string()),
        product: Some("Fake tablet".to_string()),
    }
}

/// What a plugged device holds, shared between its handle and the devices opened from it.
#[derive(Default)]
struct DeviceState {
    plugged: bool,
    reports: VecDeque<Vec<u8>>,
    written: Vec<Vec<u8>>,
    features: Vec<Vec<u8>>,
    strings: HashMap<u8, String>,
}

struct Plugged {
    info: HidDeviceInfo,
    state: Arc<Mutex<DeviceState>>,
}

#[derive(Default)]
struct World {
    devices: Vec<Plugged>,
    opens: Vec<(String, OpenOptions)>,
    deny_permission: bool,
}

/// A HID subsystem made of the devices that a test plugs.
///
/// A clone is another handle on the same world: give one clone to the code under test (as
/// `Box<dyn HidBackend>`) and keep another to plug, unplug and inspect.
#[derive(Clone, Default)]
pub struct FakeHidBackend {
    world: Arc<Mutex<World>>,
}

/// The test's hand on one plugged device: feed it reports, read what was sent to it, unplug it.
#[derive(Clone)]
pub struct FakeHidHandle {
    state: Arc<Mutex<DeviceState>>,
}

impl FakeHidBackend {
    /// A world with no device.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Plugs a device, which `enumerate` lists and `open` can open from now on.
    #[allow(
        clippy::must_use_candidate,
        reason = "the handle is optional: a test that only needs the device listed ignores it"
    )]
    pub fn plug(&self, info: HidDeviceInfo) -> FakeHidHandle {
        let state = Arc::new(Mutex::new(DeviceState {
            plugged: true,
            ..DeviceState::default()
        }));
        self.world
            .lock()
            .unwrap_or_log("fake hid world")
            .devices
            .push(Plugged {
                info,
                state: Arc::clone(&state),
            });
        FakeHidHandle { state }
    }

    /// Makes every following `open` fail with [`HidError::PermissionDenied`] (or stops doing it).
    pub fn deny_permission(&self, deny: bool) {
        self.world
            .lock()
            .unwrap_or_log("fake hid world")
            .deny_permission = deny;
    }

    /// The path and the options of every successful or refused `open`, oldest first.
    #[must_use]
    pub fn opens(&self) -> Vec<(String, OpenOptions)> {
        self.world
            .lock()
            .unwrap_or_log("fake hid world")
            .opens
            .clone()
    }
}

impl FakeHidHandle {
    /// Queues one input report for the next `read_timeout`.
    pub fn push_report(&self, report: impl Into<Vec<u8>>) {
        self.state
            .lock()
            .unwrap_or_log("fake hid device")
            .reports
            .push_back(report.into());
    }

    /// Queues several input reports, in order.
    pub fn push_reports<I, R>(&self, reports: I)
    where
        I: IntoIterator<Item = R>,
        R: Into<Vec<u8>>,
    {
        let mut state = self.state.lock().unwrap_or_log("fake hid device");
        state.reports.extend(reports.into_iter().map(Into::into));
    }

    /// How many queued reports nobody has read yet.
    #[must_use]
    pub fn pending_reports(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_log("fake hid device")
            .reports
            .len()
    }

    /// Unplugs the device: the reads of devices already open fail with
    /// [`HidError::Disconnected`], and `open` and `enumerate` no longer see it.
    pub fn unplug(&self) {
        self.state.lock().unwrap_or_log("fake hid device").plugged = false;
    }

    /// Plugs the device again, as a user does when replugging the cable.
    pub fn replug(&self) {
        self.state.lock().unwrap_or_log("fake hid device").plugged = true;
    }

    /// Sets the string descriptor at `index`.
    pub fn set_string(&self, index: u8, value: impl Into<String>) {
        self.state
            .lock()
            .unwrap_or_log("fake hid device")
            .strings
            .insert(index, value.into());
    }

    /// The output reports written to the device, oldest first.
    #[must_use]
    pub fn written(&self) -> Vec<Vec<u8>> {
        self.state
            .lock()
            .unwrap_or_log("fake hid device")
            .written
            .clone()
    }

    /// The feature reports sent to the device, oldest first.
    #[must_use]
    pub fn feature_reports(&self) -> Vec<Vec<u8>> {
        self.state
            .lock()
            .unwrap_or_log("fake hid device")
            .features
            .clone()
    }
}

impl HidBackend for FakeHidBackend {
    fn enumerate(&mut self) -> Result<Vec<HidDeviceInfo>, HidError> {
        let world = self.world.lock().unwrap_or_log("fake hid world");
        Ok(world
            .devices
            .iter()
            .filter(|d| d.state.lock().unwrap_or_log("fake hid device").plugged)
            .map(|d| d.info.clone())
            .collect())
    }

    fn open(
        &mut self,
        info: &HidDeviceInfo,
        options: OpenOptions,
    ) -> Result<Box<dyn HidDevice>, HidError> {
        let mut world = self.world.lock().unwrap_or_log("fake hid world");
        world.opens.push((info.path.clone(), options));
        if world.deny_permission {
            return Err(HidError::PermissionDenied);
        }
        let state = world
            .devices
            .iter()
            .find(|d| d.info.path == info.path)
            .filter(|d| d.state.lock().unwrap_or_log("fake hid device").plugged)
            .map(|d| Arc::clone(&d.state))
            .ok_or(HidError::NotFound)?;
        drop(world);
        Ok(Box::new(FakeHidDevice { state }))
    }
}

/// A device opened from a [`FakeHidBackend`].
struct FakeHidDevice {
    state: Arc<Mutex<DeviceState>>,
}

impl FakeHidDevice {
    fn connected(&self) -> Result<std::sync::MutexGuard<'_, DeviceState>, HidError> {
        let state = self.state.lock().unwrap_or_log("fake hid device");
        if state.plugged {
            Ok(state)
        } else {
            Err(HidError::Disconnected)
        }
    }
}

impl HidDevice for FakeHidDevice {
    /// Never waits: with no report queued it returns 0 at once, as a timeout would.
    fn read_timeout(&self, buffer: &mut [u8], _timeout: Duration) -> Result<usize, HidError> {
        let report = self.connected()?.reports.pop_front();
        let Some(report) = report else {
            return Ok(0);
        };
        let len = report.len().min(buffer.len());
        if let (Some(to), Some(from)) = (buffer.get_mut(..len), report.get(..len)) {
            to.copy_from_slice(from);
        }
        Ok(len)
    }

    fn write(&self, data: &[u8]) -> Result<usize, HidError> {
        self.connected()?.written.push(data.to_vec());
        Ok(data.len())
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<(), HidError> {
        self.connected()?.features.push(data.to_vec());
        Ok(())
    }

    fn indexed_string(&self, index: u8) -> Result<Option<String>, HidError> {
        Ok(self.connected()?.strings.get(&index).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: Duration = Duration::from_millis(500);

    fn open_one(backend: &mut FakeHidBackend, options: OpenOptions) -> Box<dyn HidDevice> {
        let info = backend.enumerate().unwrap().remove(0);
        backend.open(&info, options).unwrap()
    }

    #[test]
    fn a_plugged_device_is_listed_and_its_reports_come_out_in_order() {
        let mut backend = FakeHidBackend::new();
        let tablet = backend.plug(fake_tablet_info(0x28bd, 0x0914));
        tablet.push_reports([vec![1, 2, 3], vec![4, 5]]);

        let device = open_one(&mut backend, OpenOptions::default());
        let mut buffer = [0u8; 8];
        assert_eq!(device.read_timeout(&mut buffer, TIMEOUT), Ok(3));
        assert_eq!(buffer.get(..3), Some(&[1, 2, 3][..]));
        assert_eq!(device.read_timeout(&mut buffer, TIMEOUT), Ok(2));
        // Nothing left: a timeout, not an error.
        assert_eq!(device.read_timeout(&mut buffer, TIMEOUT), Ok(0));
        assert_eq!(tablet.pending_reports(), 0);
    }

    #[test]
    fn a_report_longer_than_the_buffer_is_cut() {
        let mut backend = FakeHidBackend::new();
        backend.plug(fake_tablet_info(1, 2)).push_report([9u8; 10]);
        let device = open_one(&mut backend, OpenOptions::default());
        let mut buffer = [0u8; 4];
        assert_eq!(device.read_timeout(&mut buffer, TIMEOUT), Ok(4));
    }

    #[test]
    fn unplugging_fails_the_open_devices_and_hides_the_device_until_it_is_replugged() {
        let mut backend = FakeHidBackend::new();
        let tablet = backend.plug(fake_tablet_info(1, 2));
        let device = open_one(&mut backend, OpenOptions::default());
        let info = fake_tablet_info(1, 2);

        tablet.unplug();
        let mut buffer = [0u8; 4];
        assert_eq!(
            device.read_timeout(&mut buffer, TIMEOUT),
            Err(HidError::Disconnected)
        );
        assert_eq!(device.write(&[0]), Err(HidError::Disconnected));
        assert!(backend.enumerate().unwrap().is_empty());
        assert!(matches!(
            backend.open(&info, OpenOptions::default()),
            Err(HidError::NotFound)
        ));

        tablet.replug();
        assert_eq!(backend.enumerate().unwrap().len(), 1);
        assert!(backend.open(&info, OpenOptions::default()).is_ok());
    }

    #[test]
    fn opening_can_be_refused_and_every_attempt_is_logged_with_its_options() {
        let mut backend = FakeHidBackend::new();
        backend.plug(fake_tablet_info(1, 2));
        let info = fake_tablet_info(1, 2);

        backend.deny_permission(true);
        let exclusive = OpenOptions { exclusive: true };
        assert!(matches!(
            backend.open(&info, exclusive),
            Err(HidError::PermissionDenied)
        ));
        backend.deny_permission(false);
        assert!(backend.open(&info, OpenOptions::default()).is_ok());

        assert_eq!(
            backend.opens(),
            [
                (info.path.clone(), exclusive),
                (info.path, OpenOptions::default())
            ]
        );
    }

    #[test]
    fn what_the_driver_sends_to_the_device_is_kept_for_the_test() {
        let mut backend = FakeHidBackend::new();
        let tablet = backend.plug(fake_tablet_info(1, 2));
        tablet.set_string(201, "OEM02_T18e_123456");
        let device = open_one(&mut backend, OpenOptions::default());

        assert_eq!(device.write(&[0x02, 0xB0, 0x04]), Ok(3));
        assert_eq!(device.send_feature_report(&[0x02, 0x02]), Ok(()));
        assert_eq!(
            device.indexed_string(201),
            Ok(Some("OEM02_T18e_123456".to_string()))
        );
        assert_eq!(device.indexed_string(7), Ok(None));
        assert_eq!(tablet.written(), [vec![0x02, 0xB0, 0x04]]);
        assert_eq!(tablet.feature_reports(), [vec![0x02, 0x02]]);
    }

    #[test]
    fn a_path_that_is_not_plugged_is_not_found() {
        let mut backend = FakeHidBackend::new();
        assert!(matches!(
            backend.open(&fake_tablet_info(9, 9), OpenOptions::default()),
            Err(HidError::NotFound)
        ));
    }
}
