//! # Windows Ink pen injection
//!
//! Makes the tablet show up as a real pen for drawing programs (Krita, Photoshop, Clip Studio,
//! Paint...). It uses the synthetic pointer API of Windows 10 (1809 and later): the driver
//! creates a virtual pen device and reports the position, the pressure and the tilt on it, so
//! programs get the same data as from a Wacom-class device. No kernel driver is needed.
//!
//! This is opt-in (the "Drawing" page). Without it the injector keeps sending plain mouse
//! events, which is what rhythm games want.
//!
//! The pen has a life cycle that Windows checks: it must enter the range, then touch, then
//! lift, then leave. A report that skips a step is dropped without any error, so the
//! transitions live in [`pointer_flags`], where they are tested.

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::Controls::{
    CreateSyntheticPointerDevice, DestroySyntheticPointerDevice, HSYNTHETICPOINTERDEVICE,
    POINTER_FEEDBACK_DEFAULT, POINTER_TYPE_INFO, POINTER_TYPE_INFO_0,
};
use windows_sys::Win32::UI::Input::Pointer::{
    InjectSyntheticPointerInput, POINTER_FLAG_DOWN, POINTER_FLAG_FIRSTBUTTON,
    POINTER_FLAG_INCONTACT, POINTER_FLAG_INRANGE, POINTER_FLAG_NEW, POINTER_FLAG_UP,
    POINTER_FLAG_UPDATE, POINTER_INFO, POINTER_PEN_INFO,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PEN_FLAG_ERASER, PEN_FLAG_INVERTED, PEN_FLAG_NONE, PEN_MASK_PRESSURE, PEN_MASK_TILT_X,
    PEN_MASK_TILT_Y, PT_PEN,
};

/// The pressure scale of the pipeline (`ProcessedFrame::pressure`).
const PIPELINE_MAX_PRESSURE: i32 = 8191;
/// The pressure scale of a Windows pen pointer.
const WINDOWS_MAX_PRESSURE: u32 = 1024;
/// Windows tilt is in degrees, from -90 to 90.
const MAX_TILT: i32 = 90;

/// What the next pen report carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Sample {
    x: i32,
    y: i32,
    /// Windows scale (0 to 1024).
    pressure: u32,
    tilt_x: i32,
    tilt_y: i32,
}

/// A virtual Windows Ink pen.
#[allow(clippy::struct_excessive_bools)]
pub struct InkPen {
    device: HSYNTHETICPOINTERDEVICE,
    /// The pen is in the range of the tablet (hovering or touching).
    in_range: bool,
    /// The tip touches the tablet.
    in_contact: bool,
    sample: Sample,
    /// The eraser end of the pen is the one near the tablet.
    eraser: bool,
    /// A position was recorded and not reported yet.
    fresh: bool,
    /// An injection failed and was logged: do not flood the log at every report.
    failure_logged: bool,
}

impl InkPen {
    /// Creates the virtual pen, or `None` when Windows refuses (before Windows 10 1809, or the
    /// pointer injection is not available in the session).
    pub fn create() -> Option<Self> {
        // SAFETY: plain call, the arguments are values; the result is checked for null below.
        let device = unsafe { CreateSyntheticPointerDevice(PT_PEN, 1, POINTER_FEEDBACK_DEFAULT) };
        if device.is_null() {
            log::warn!(target: "Injector", "Windows Ink is not available: the virtual pen could not be created, the mouse is used instead.");
            return None;
        }
        log::info!(target: "Injector", "Windows Ink pen created.");
        Some(Self {
            device,
            in_range: false,
            in_contact: false,
            sample: Sample::default(),
            eraser: false,
            fresh: false,
            failure_logged: false,
        })
    }

    /// Tells which end of the pen is used. Applies to the next report, so drawing programs switch
    /// to their eraser tool as soon as the eraser end comes near the tablet.
    pub const fn set_eraser(&mut self, eraser: bool) {
        self.eraser = eraser;
    }

    /// Remembers the position, the pressure and the tilt of the next report.
    ///
    /// `(x, y)` is a position on the Windows desktop and `origin` is the top-left corner of the
    /// desktop (negative when a screen sits left of or above the main one). Nothing is sent yet:
    /// [`Self::report`] sends it once the tip state is known, so one tablet packet makes one
    /// injection.
    pub fn record(
        &mut self,
        (x, y): (f32, f32),
        origin: (i32, i32),
        pressure: i32,
        tilt_x: i32,
        tilt_y: i32,
    ) {
        let (x, y) = pen_position(x, y, origin);
        self.sample = Sample {
            x,
            y,
            pressure: scale_pressure(pressure),
            tilt_x: tilt_x.clamp(-MAX_TILT, MAX_TILT),
            tilt_y: tilt_y.clamp(-MAX_TILT, MAX_TILT),
        };
        self.fresh = true;
    }

    /// Sends the recorded position with the given tip state.
    ///
    /// Does nothing when no position was recorded since the last report.
    pub fn report(&mut self, in_contact: bool) {
        if !self.fresh {
            return;
        }
        self.fresh = false;

        if let Some(flags) = pointer_flags(self.in_range, self.in_contact, true, in_contact) {
            // A hovering pen has no pressure.
            let sample = if in_contact {
                self.sample
            } else {
                Sample {
                    pressure: 0,
                    ..self.sample
                }
            };
            self.inject(flags, sample);
        }
        self.in_range = true;
        self.in_contact = in_contact;
    }

    /// Takes the pen out of the range of the tablet, lifting it first if it touches.
    pub fn leave(&mut self) {
        self.fresh = false;
        if self.in_contact
            && let Some(flags) = pointer_flags(true, true, true, false)
        {
            self.inject(flags, self.sample);
            self.in_contact = false;
        }
        if self.in_range {
            if let Some(flags) = pointer_flags(true, false, false, false) {
                self.inject(flags, self.sample);
            }
            self.in_range = false;
        }
    }

    fn inject(&mut self, flags: u32, sample: Sample) {
        let pen = POINTER_PEN_INFO {
            pointerInfo: POINTER_INFO {
                pointerType: PT_PEN,
                pointerFlags: flags,
                ptPixelLocation: POINT {
                    x: sample.x,
                    y: sample.y,
                },
                ..Default::default()
            },
            penFlags: pen_flags(self.eraser),
            penMask: PEN_MASK_PRESSURE | PEN_MASK_TILT_X | PEN_MASK_TILT_Y,
            pressure: sample.pressure,
            rotation: 0,
            tiltX: sample.tilt_x,
            tiltY: sample.tilt_y,
        };
        let info = POINTER_TYPE_INFO {
            r#type: PT_PEN,
            Anonymous: POINTER_TYPE_INFO_0 { penInfo: pen },
        };
        // SAFETY: `self.device` is a live device made by `create` and destroyed only in `drop`;
        // `info` is a valid, initialized structure for the duration of the call.
        let ok = unsafe { InjectSyntheticPointerInput(self.device, &raw const info, 1) };
        if ok == 0 && !self.failure_logged {
            self.failure_logged = true;
            log::warn!(target: "Injector", "Windows Ink: the pen report was refused by Windows.");
        }
    }
}

impl Drop for InkPen {
    fn drop(&mut self) {
        self.leave();
        // SAFETY: the device was made by `create` and is destroyed once, here.
        unsafe { DestroySyntheticPointerDevice(self.device) };
    }
}

/// The pen is only used from the polling thread that owns the injector. The handle is a plain
/// kernel object handle, not tied to the thread that created it.
// SAFETY: see above; the handle is never shared, only moved with its owner.
unsafe impl Send for InkPen {}

/// The pen flags for the end of the pen in use. Windows wants both flags for the eraser end:
/// the pen is inverted, and the tool is an eraser.
const fn pen_flags(eraser: bool) -> u32 {
    if eraser {
        PEN_FLAG_INVERTED | PEN_FLAG_ERASER
    } else {
        PEN_FLAG_NONE
    }
}

/// Converts a desktop position to the position Windows expects from a pen pointer.
///
/// Windows reads the position of a synthetic pen as a distance from the top-left corner of the
/// whole desktop, not from the main screen. Sending desktop coordinates as they are lands the pen
/// one screen too far whenever a screen sits left of or above the main one.
const fn pen_position(x: f32, y: f32, origin: (i32, i32)) -> (i32, i32) {
    (
        (x.round() as i32).saturating_sub(origin.0),
        (y.round() as i32).saturating_sub(origin.1),
    )
}

/// Converts the pipeline pressure (0 to 8191) to the Windows pen scale (0 to 1024).
fn scale_pressure(pressure: i32) -> u32 {
    let clamped = pressure.clamp(0, PIPELINE_MAX_PRESSURE) as u32;
    // Rounded to the nearest step, so a light touch is not lost to the integer division.
    (clamped * WINDOWS_MAX_PRESSURE + PIPELINE_MAX_PRESSURE as u32 / 2)
        / PIPELINE_MAX_PRESSURE as u32
}

/// The pointer flags for the move from `(was_in_range, was_in_contact)` to
/// `(in_range, in_contact)`, or `None` when nothing happens.
///
/// The pen must go through the steps Windows expects: enter the range (`NEW`), touch (`DOWN`),
/// lift (`UP`), then leave the range (a report without `INRANGE`). A pen cannot touch outside
/// the range, so `in_contact` is ignored when `in_range` is false.
#[allow(clippy::fn_params_excessive_bools)]
const fn pointer_flags(
    was_in_range: bool,
    was_in_contact: bool,
    in_range: bool,
    in_contact: bool,
) -> Option<u32> {
    let in_contact = in_range && in_contact;
    match (was_in_range, in_range) {
        (false, false) => None,
        // Leaving the range: the tip was lifted before, so this is a plain update.
        (true, false) => Some(POINTER_FLAG_UPDATE),
        (was_range, true) => {
            let entering = if was_range { 0 } else { POINTER_FLAG_NEW };
            let tip = match (was_in_contact && was_range, in_contact) {
                (false, false) => POINTER_FLAG_UPDATE,
                (false, true) => {
                    POINTER_FLAG_INCONTACT | POINTER_FLAG_DOWN | POINTER_FLAG_FIRSTBUTTON
                }
                (true, true) => {
                    POINTER_FLAG_INCONTACT | POINTER_FLAG_UPDATE | POINTER_FLAG_FIRSTBUTTON
                }
                (true, false) => POINTER_FLAG_UP,
            };
            Some(POINTER_FLAG_INRANGE | entering | tip)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pen_that_stays_away_sends_nothing() {
        assert_eq!(pointer_flags(false, false, false, false), None);
        assert_eq!(pointer_flags(false, false, false, true), None);
    }

    #[test]
    fn entering_the_range_hovering_is_a_new_pointer() {
        assert_eq!(
            pointer_flags(false, false, true, false),
            Some(POINTER_FLAG_INRANGE | POINTER_FLAG_NEW | POINTER_FLAG_UPDATE)
        );
    }

    #[test]
    fn hovering_updates_the_pointer() {
        assert_eq!(
            pointer_flags(true, false, true, false),
            Some(POINTER_FLAG_INRANGE | POINTER_FLAG_UPDATE)
        );
    }

    #[test]
    fn touching_the_tablet_presses_the_first_button() {
        assert_eq!(
            pointer_flags(true, false, true, true),
            Some(
                POINTER_FLAG_INRANGE
                    | POINTER_FLAG_INCONTACT
                    | POINTER_FLAG_DOWN
                    | POINTER_FLAG_FIRSTBUTTON
            )
        );
    }

    #[test]
    fn a_stroke_in_progress_is_an_update_with_the_button_held() {
        assert_eq!(
            pointer_flags(true, true, true, true),
            Some(
                POINTER_FLAG_INRANGE
                    | POINTER_FLAG_INCONTACT
                    | POINTER_FLAG_UPDATE
                    | POINTER_FLAG_FIRSTBUTTON
            )
        );
    }

    #[test]
    fn lifting_the_tip_keeps_the_pen_in_range() {
        assert_eq!(
            pointer_flags(true, true, true, false),
            Some(POINTER_FLAG_INRANGE | POINTER_FLAG_UP)
        );
    }

    #[test]
    fn leaving_the_range_drops_the_range_flag() {
        let flags = pointer_flags(true, false, false, false).unwrap();
        assert_eq!(flags & POINTER_FLAG_INRANGE, 0);
        assert_ne!(flags & POINTER_FLAG_UPDATE, 0);
    }

    #[test]
    fn a_pen_that_appears_already_touching_is_new_and_down_in_one_report() {
        let flags = pointer_flags(false, false, true, true).unwrap();
        assert_ne!(flags & POINTER_FLAG_NEW, 0);
        assert_ne!(flags & POINTER_FLAG_DOWN, 0);
        assert_ne!(flags & POINTER_FLAG_INRANGE, 0);
    }

    #[test]
    fn a_desktop_with_one_screen_on_the_left_shifts_the_position_right() {
        // Main screen at 0, another one at -1920: the desktop starts at -1920.
        assert_eq!(pen_position(960.0, 540.0, (-1920, 0)), (2880, 540));
        assert_eq!(pen_position(-960.0, 540.0, (-1920, 0)), (960, 540));
    }

    #[test]
    fn a_desktop_that_starts_at_the_main_screen_keeps_the_position() {
        assert_eq!(pen_position(960.4, 540.6, (0, 0)), (960, 541));
    }

    #[test]
    fn a_screen_above_the_main_one_shifts_the_position_down() {
        assert_eq!(pen_position(100.0, 100.0, (0, -1080)), (100, 1180));
    }

    #[test]
    fn the_tip_end_sets_no_pen_flag() {
        assert_eq!(pen_flags(false), PEN_FLAG_NONE);
    }

    #[test]
    fn the_eraser_end_is_inverted_and_an_eraser() {
        let flags = pen_flags(true);
        assert_ne!(flags & PEN_FLAG_INVERTED, 0);
        assert_ne!(flags & PEN_FLAG_ERASER, 0);
    }

    #[test]
    fn the_pressure_is_scaled_to_the_windows_range() {
        assert_eq!(scale_pressure(0), 0);
        assert_eq!(scale_pressure(PIPELINE_MAX_PRESSURE), WINDOWS_MAX_PRESSURE);
        assert_eq!(scale_pressure(4096), 512);
    }

    #[test]
    fn the_pressure_stays_in_range() {
        assert_eq!(scale_pressure(-5), 0);
        assert_eq!(scale_pressure(100_000), WINDOWS_MAX_PRESSURE);
    }

    #[test]
    fn a_light_touch_is_not_lost() {
        assert!(scale_pressure(5) >= 1);
    }
}
