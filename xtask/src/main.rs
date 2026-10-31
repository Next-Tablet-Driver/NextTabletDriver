//! Repository tasks, run with `cargo xtask <task>`.
//!
//! Tasks:
//! - `lint-style`: the checks of the quality policy that rustfmt and clippy cannot make, see
//!   [`lint_style`];
//! - `ci-plan`: the crates a change touches, to run only their CI jobs, see [`ci_plan`].

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool: its output is the user interface"
)]

mod ci_plan;
mod lint_style;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <task>\n\ntasks:\n  \
lint-style   check the line width, comments, to-dos, unsafe and rustfmt::skip rules\n  \
ci-plan      list the crates a change touches [--base REV] [--event NAME] [--files FILE]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("lint-style") => lint_style::run(&workspace_root()),
        Some("ci-plan") => ci_plan::run(&workspace_root(), args.get(1..).unwrap_or_default()),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// The root of the workspace: the parent of this crate's folder.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}
