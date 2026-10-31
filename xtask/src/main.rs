//! Repository tasks, run with `cargo xtask <task>`.
//!
//! Tasks:
//! - `lint-style`: the checks of the quality policy that rustfmt and clippy cannot make, see
//!   [`lint_style`].

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool: its output is the user interface"
)]

mod lint_style;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <task>\n\ntasks:\n  lint-style   check the line width, \
    comments, TODOs, unsafe and rustfmt::skip rules";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("lint-style") {
        lint_style::run(&workspace_root())
    } else {
        eprintln!("{USAGE}");
        ExitCode::FAILURE
    }
}

/// The root of the workspace: the parent of this crate's folder.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}
