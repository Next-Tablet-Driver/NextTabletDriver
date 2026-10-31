//! `ntd-recorder`: captures what a drawing tablet sends, for bug reports and tablet support.
//!
//! ```text
//! ntd-recorder list
//! ntd-recorder record [VID:PID] [--out file.ntdcap]
//! ntd-recorder check file.ntdcap
//! ntd-recorder trim in.ntdcap out.ntdcap [--per-step N]
//! ```
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an interactive command-line tool: its output is the user interface"
)]

mod capture;

use capture::{Capture, DeviceInfo, InterfaceInfo, Report, StepInfo, serial_hint, to_hex};
use ntd_hid::{Backend, HidBackend, HidDevice, HidDeviceInfo, OpenOptions};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// What the user is asked to do, in order, and for how long (seconds).
const STEPS: &[(&str, &str, u64)] = &[
    (
        "out-of-range",
        "Keep the pen away from the tablet. Do not touch anything.",
        3,
    ),
    (
        "hover",
        "Hold the pen just above the tablet, without touching it. Keep it still.",
        5,
    ),
    (
        "hover-move",
        "Move the pen slowly above the tablet, without touching it.",
        5,
    ),
    (
        "touch-light",
        "Draw slowly on the tablet with a very light pressure.",
        5,
    ),
    (
        "touch-hard",
        "Draw slowly on the tablet with a hard pressure.",
        5,
    ),
    ("tilt", "Draw while tilting the pen in every direction.", 5),
    (
        "pen-button-1",
        "Hover over the tablet and press the first pen button several times.",
        5,
    ),
    (
        "pen-button-2",
        "Hover over the tablet and press the second pen button several times.",
        5,
    ),
    (
        "tablet-buttons",
        "Press every button of the tablet, one after the other.",
        10,
    ),
    (
        "corners",
        "Touch each of the four corners, then the center, one at a time.",
        10,
    ),
];

const USAGE: &str = "usage:
  ntd-recorder list
  ntd-recorder record [VID:PID] [--out FILE]
  ntd-recorder check FILE
  ntd-recorder trim IN OUT [--per-step N]";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("list") => list(),
        Some("record") => record(args.get(1..).unwrap_or_default()),
        Some("check") => check(args.get(1).map(String::as_str)),
        Some("trim") => trim(args.get(1..).unwrap_or_default()),
        _ => Err(USAGE.into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Distinct (VID, PID) pairs with a display name, in a stable order.
fn devices(backend: &mut dyn HidBackend) -> Result<Vec<(u16, u16, String)>> {
    let mut found: Vec<(u16, u16, String)> = Vec::new();
    for d in backend.enumerate()? {
        let key = (d.vendor_id, d.product_id);
        if !found.iter().any(|(v, p, _)| (*v, *p) == key) {
            let name = format!(
                "{} {}",
                d.manufacturer.as_deref().unwrap_or("?"),
                d.product.as_deref().unwrap_or("?")
            );
            found.push((key.0, key.1, name));
        }
    }
    found.sort_by_key(|(v, p, _)| (*v, *p));
    Ok(found)
}

fn print_devices(all: &[(u16, u16, String)]) {
    for (i, (v, p, name)) in all.iter().enumerate() {
        println!("{:>3}  {v:04x}:{p:04x}  {name}", i + 1);
    }
}

fn list() -> Result<()> {
    let mut backend = Backend::new()?;
    print_devices(&devices(&mut backend)?);
    Ok(())
}

fn parse_vid_pid(s: &str) -> Option<(u16, u16)> {
    let (v, p) = s.split_once(':')?;
    let hex = |x: &str| u16::from_str_radix(x.trim_start_matches("0x"), 16).ok();
    Some((hex(v)?, hex(p)?))
}

fn ask(prompt: &str) -> Result<String> {
    print!("{prompt}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_owned())
}

fn pick_device(backend: &mut dyn HidBackend, wanted: Option<(u16, u16)>) -> Result<(u16, u16)> {
    if let Some(ids) = wanted {
        return Ok(ids);
    }
    let all = devices(backend)?;
    print_devices(&all);
    let choice: usize = ask("Number of your tablet: ")?.parse()?;
    let (v, p, _) = all.get(choice.wrapping_sub(1)).ok_or("no such device")?;
    Ok((*v, *p))
}

fn record(args: &[String]) -> Result<()> {
    let mut ids = None;
    let mut out: Option<PathBuf> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--out" {
            out = Some(PathBuf::from(it.next().ok_or("--out needs a file name")?));
        } else {
            ids = Some(parse_vid_pid(a).ok_or("expected VID:PID in hexadecimal, e.g. 28bd:0914")?);
        }
    }

    let mut backend = Backend::new()?;
    let (vid, pid) = pick_device(&mut backend, ids)?;
    let infos: Vec<HidDeviceInfo> = backend
        .enumerate()?
        .into_iter()
        .filter(|d| d.vendor_id == vid && d.product_id == pid)
        .collect();
    let first = infos.first().ok_or("device not found")?;

    let mut cap = Capture::new(DeviceInfo {
        vendor_id: vid,
        product_id: pid,
        manufacturer: first.manufacturer.clone(),
        product: first.product.clone(),
        serial_hint: first.serial_number.as_deref().and_then(serial_hint),
        release_number: first.release_number,
    });

    println!(
        "Close NextTabletDriver and any other tablet driver first, or the tablet cannot be read."
    );
    let mut open: Vec<(usize, Box<dyn HidDevice>)> = Vec::new();
    for info in &infos {
        let index = cap.interfaces.len();
        match backend.open(info, OpenOptions::default()) {
            Ok(dev) => {
                let descriptor_hex = dev.report_descriptor().ok().map(|d| to_hex(&d));
                // Some interfaces open fine but refuse reads (the OS keeps them for itself).
                if let Err(e) = dev.read_timeout(&mut [0_u8; 1024], Duration::ZERO) {
                    eprintln!(
                        "skipped interface {} (usage {:04x}:{:04x}): cannot read it ({e})",
                        info.interface_number, info.usage_page, info.usage
                    );
                    continue;
                }
                cap.interfaces.push(InterfaceInfo {
                    index,
                    interface_number: info.interface_number,
                    usage_page: info.usage_page,
                    usage: info.usage,
                    descriptor_hex,
                });
                open.push((index, dev));
            }
            Err(e) => eprintln!(
                "skipped interface {} (usage {:04x}:{:04x}): {e}",
                info.interface_number, info.usage_page, info.usage
            ),
        }
    }
    if open.is_empty() {
        return Err("no interface could be opened".into());
    }
    println!("Opened {} interface(s).\n", open.len());

    let start = Instant::now();
    for (step, (id, instruction, secs)) in STEPS.iter().enumerate() {
        println!("Step {}/{}: {instruction}", step + 1, STEPS.len());
        ask(&format!("Press Enter, then do it for {secs} seconds... "))?;
        let step_start = elapsed_ms(start);
        let deadline = Instant::now() + Duration::from_secs(*secs);
        let mut buf = [0_u8; 1024];
        while Instant::now() < deadline {
            let mut idle = true;
            for (index, dev) in &open {
                let n = dev.read_timeout(&mut buf, Duration::ZERO)?;
                if n > 0 {
                    idle = false;
                    cap.reports.push(Report {
                        t_ms: elapsed_ms(start),
                        interface: *index,
                        step,
                        data_hex: to_hex(buf.get(..n).unwrap_or_default()),
                    });
                }
            }
            if idle {
                std::thread::sleep(Duration::from_micros(200));
            }
        }
        cap.steps.push(StepInfo {
            id: (*id).to_owned(),
            instruction: (*instruction).to_owned(),
            start_ms: step_start,
            end_ms: elapsed_ms(start),
        });
        println!("  done ({} reports so far)\n", cap.reports.len());
    }

    let out = out.unwrap_or_else(|| PathBuf::from(format!("{vid:04x}-{pid:04x}.ntdcap")));
    cap.write(&out)?;
    println!("Saved {} reports to {}", cap.reports.len(), out.display());
    println!("Attach this file to your issue. It contains no personal data.");
    Ok(())
}

fn elapsed_ms(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn check(path: Option<&str>) -> Result<()> {
    let cap = Capture::read(&PathBuf::from(path.ok_or("check needs a file")?))?;
    println!(
        "{:04x}:{:04x} {} - {} interface(s), {} step(s), {} report(s)",
        cap.device.vendor_id,
        cap.device.product_id,
        cap.device.product.as_deref().unwrap_or("?"),
        cap.interfaces.len(),
        cap.steps.len(),
        cap.reports.len()
    );
    Ok(())
}

/// Default number of reports kept per step by `trim`.
const DEFAULT_PER_STEP: usize = 40;

fn trim(args: &[String]) -> Result<()> {
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        return Err(USAGE.into());
    };
    let per_step = match args.get(2).map(String::as_str) {
        Some("--per-step") => args.get(3).ok_or("--per-step needs a number")?.parse()?,
        Some(other) => return Err(format!("unexpected argument {other}").into()),
        None => DEFAULT_PER_STEP,
    };
    let cap = Capture::read(&PathBuf::from(input))?;
    let small = cap.trimmed(per_step);
    small.write(&PathBuf::from(output))?;
    println!(
        "Kept {} of {} reports.",
        small.reports.len(),
        cap.reports.len()
    );
    Ok(())
}
