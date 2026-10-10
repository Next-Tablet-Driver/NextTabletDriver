# Adding a tablet

This guide shows how to get a new tablet working. You do not need to write Rust for every case. Start at the top and stop when your tablet works.

## How tablet support works

Two things make a tablet work:

1. **A definition.** A small JSON file that describes the tablet: its name, size, USB ID, and which parser to use. NextTabletDriver does not keep its own list. It uses the definitions of [OpenTabletDriver](https://github.com/OpenTabletDriver/OpenTabletDriver), which are included as a git submodule in `tablets/OpenTabletDriver`.
2. **A parser.** Rust code that decodes the bytes the tablet sends (called *reports*). Each tablet family has its own layout, so each family has its own parser. The definition names the parser to use.

Both must exist. If the definition names a parser that this project does not have, the driver prints a warning and falls back to a generic parser, which only works for tablets that happen to use a very common layout.

## Step 1: find your tablet's USB ID

Every USB device has a vendor ID and a product ID, written as `VID:PID`, for example `28bd:0914`.

- Open the **Console** tab with your tablet plugged in. If the tablet is not known, the driver prints its ID:

  ```text
  Found unrecognized potential tablet device: [28bd:0914] 'XP-Pen' - 'Deco 01 V2'.
  ```

- **Windows:** open Device Manager, find the tablet, open its properties, choose **Details**, then **Hardware Ids**. You see `VID_28BD&PID_0914`.
- **Linux:** run `lsusb` and read the `ID` column.

The definition files write IDs as **decimal** numbers. Convert hexadecimal to decimal before you search:

```bash
printf '%d\n' 0x28BD    # 10429
```

## Step 2: check if a definition exists

Search the definitions for your product ID. They are in `tablets/OpenTabletDriver/OpenTabletDriver.Configurations/Configurations/`, in one folder per brand.

```bash
grep -rl '"ProductID": 2324' tablets/OpenTabletDriver/OpenTabletDriver.Configurations/Configurations
```

- **You found a file, and the tablet works.** Nothing to do.
- **You found a file, but the tablet does not work.** Look at its `ReportParser` value and go to [Step 4](#step-4-write-a-parser).
- **You found nothing.** Go to [Step 3](#step-3-write-a-definition).

## Step 3: write a definition

The easiest way is to copy a definition of another tablet from the same brand, and change the values. A definition looks like this:

```json
{
  "Name": "Example Brand Model X",
  "Specifications": {
    "Digitizer": {
      "Width": 160.0,
      "Height": 100.0,
      "MaxX": 32000,
      "MaxY": 20000
    },
    "Pen": {
      "MaxPressure": 8191,
      "ButtonCount": 2
    }
  },
  "DigitizerIdentifiers": [
    {
      "VendorID": 4660,
      "ProductID": 22136,
      "InputReportLength": 12,
      "ReportParser": "OpenTabletDriver.Configurations.Parsers.XP_Pen.XP_PenOffsetPressureReportParser",
      "OutputInitReport": [ "ArAE" ]
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `Name` | The name shown in the app. Must be unique. |
| `Digitizer.Width`, `Height` | The physical size of the drawing surface, in millimeters. |
| `Digitizer.MaxX`, `MaxY` | The largest X and Y values the tablet reports. |
| `Pen.MaxPressure` | The largest pressure value, such as 2047, 4095 or 8191. |
| `Pen.ButtonCount` | Number of buttons on the pen. |
| `VendorID`, `ProductID` | The USB ID, in decimal. |
| `InputReportLength` | The size, in bytes, of the pen reports. The driver uses it to tell tablets with the same ID apart. |
| `ReportParser` | Which parser decodes the reports. See [Step 4](#step-4-write-a-parser). |
| `OutputInitReport`, `FeatureInitReport` | Optional. Bytes (in base64) that wake the tablet up or switch it to pen mode. |
| `DeviceStrings` | Optional. Text patterns used to tell apart tablets that share one ID and report length. |

The size and report values come from the tablet itself. If you can, test them with a capture of real reports.

### Try a definition on your own computer

You can test a definition without changing the repository. Put the `.json` file in a folder named `tablets`, then start the app. The driver looks for `tablets` in its **working directory**, which is the folder you start it from. If you start the app from a terminal, `cd` to the folder that contains `tablets` first.

A definition in `tablets` wins over a definition with the same `Name` in the built-in list. On Linux, also regenerate the udev rules so the system gives you access and ignores the real tablet. See [Linux setup](linux.md#using-a-tablet-that-is-not-in-the-rules-file).

### Share the definition

Send your definition to **OpenTabletDriver**. It keeps all definitions and other projects use them. After they accept it, it reaches NextTabletDriver with the next submodule update. This also helps OpenTabletDriver users.

If you cannot, [open a tablet support request](https://github.com/Next-Tablet-Driver/NextTabletDriver/issues/new?template=tablet_support.yml) and include:

- the exact brand and model,
- the `VID:PID`,
- what works and what does not,
- a few raw reports, if you can capture them (for example with the Tablet Debugger of OpenTabletDriver).

## Step 4: write a parser

You need a new parser only when no existing one decodes your tablet. You do need to read Rust and to know the report layout. The existing parsers in `src/drivers/parsers/` are good examples. Start with `fallback.rs`, which is the simplest.

### What a parser does

A parser implements the `ReportParser` trait. It gets the raw bytes of one report and returns a `TabletData`, or `None` if the report is not a pen report. The first byte is always the report ID. The driver adds a zero report ID on Linux, so parsers see the same layout on every system.

`TabletData` holds the status (`OutOfRange`, `Hover`, `Contact`, ...), X and Y in raw sensor units, pressure, tilt, buttons, and whether the tablet is connected.

### Example

This parser reads a made-up layout: byte 1 is a flag byte, then X, Y and pressure as 16-bit little-endian numbers.

```rust
use crate::drivers::parsers::ReportParser;
use crate::drivers::{TabletData, TabletStatus};

/// Layout (little endian):
/// byte 0 report ID, byte 1 flags (0 = pen out of range, bit 0 = tip touching),
/// bytes 2-3 X, bytes 4-5 Y, bytes 6-7 pressure.
pub struct ExampleParser;

impl ReportParser for ExampleParser {
    fn parse(&self, data: &[u8]) -> Option<TabletData> {
        let [_, flags, x_lo, x_hi, y_lo, y_hi, p_lo, p_hi, ..] = data else {
            return None; // too short to be a pen report
        };

        let status = if *flags == 0 {
            TabletStatus::OutOfRange
        } else if flags & 0x01 != 0 {
            TabletStatus::Contact
        } else {
            TabletStatus::Hover
        };

        let mut report = TabletData {
            status,
            x: u32::from(u16::from_le_bytes([*x_lo, *x_hi])),
            y: u32::from(u16::from_le_bytes([*y_lo, *y_hi])),
            pressure: u16::from_le_bytes([*p_lo, *p_hi]),
            is_connected: status != TabletStatus::OutOfRange,
            ..Default::default()
        };
        report.set_raw(data); // keep the raw bytes for the debugger
        Some(report)
    }
}
```

Notice the pattern match on the slice. It checks the length and gives names to the bytes in one step. This follows the project rule that production code must not use `data[i]` indexing, `unwrap` or `expect` (see [Architecture](architecture.md#safety-rules-in-the-code)).

### Test it

Add a test in the same file. Use real bytes from a capture, if you have them. The project style is a test that returns a `Result`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_touching_pen_is_a_contact() -> Result<(), Box<dyn std::error::Error>> {
        let packet = [0x02, 0x01, 0x10, 0x00, 0x20, 0x00, 0x40, 0x00];
        let report = ExampleParser.parse(&packet).ok_or("packet was rejected")?;
        assert_eq!(report.status, TabletStatus::Contact);
        assert_eq!((report.x, report.y, report.pressure), (16, 32, 64));
        assert!(report.is_connected);
        Ok(())
    }

    #[test]
    fn a_short_packet_is_rejected() {
        assert!(ExampleParser.parse(&[0x02, 0x01]).is_none());
    }
}
```

Also test the cases that go wrong: a packet that is too short, an empty packet, and the pen leaving the surface.

### Register it

1. Add `pub mod your_parser;` to `src/drivers/parsers/mod.rs` (or inside your brand's module).
2. Add a line to `create_parser` in the same file. It matches a **part of the name** that the definition gives in `ReportParser`:

   ```rust
   name if name.contains("YourReportParser") => Box::new(your_parser::ExampleParser),
   ```

   The first matching line wins. Put a specific name before a more general name that contains it. For example, `UCLogicV2` is listed before `UCLogic`.
3. Add your name to the tests of `create_parser` at the bottom of that file.
4. Run the checks from [Development](development.md#run-the-checks).

Then send a pull request. Say which tablet you tested on.

## Does the tablet need to be in the udev rules?

On Linux, yes. The file `scripts/99-nexttabletdriver.rules` is generated from the definitions. After the submodule changes, a maintainer regenerates it with `scripts/generate_udev_rules.sh`. If you only test locally, regenerate it yourself.
