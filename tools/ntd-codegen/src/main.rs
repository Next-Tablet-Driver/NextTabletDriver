//! `ntd-codegen`: validates the `OpenTabletDriver` tablet definitions and writes the static tables
//! of `ntd-tablets` (`crates/ntd-tablets/src/generated/`).
//!
//! `cargo run --locked -p ntd-codegen` regenerates the files; with `--check` it writes nothing
//! and fails when the committed files differ from what it would write.

mod emit;
mod model;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// What stops the tool.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// A file or folder could not be read or written.
    #[error("{path}: {message}")]
    Io {
        /// The file or folder.
        path: String,
        /// The OS message.
        message: String,
    },
    /// A definition is not valid JSON of the expected shape.
    #[error("{path}: {source}")]
    Json {
        /// The definition.
        path: PathBuf,
        /// The parse error.
        source: serde_json::Error,
    },
    /// The definitions break a rule.
    #[error("{} problem(s) in the definitions:\n{}", .0.len(), .0.join("\n"))]
    Invalid(Vec<String>),
    /// `--check` found a generated file that is not up to date.
    #[error("{0} is out of date: run `cargo run --locked -p ntd-codegen`")]
    OutOfDate(String),
    /// An argument the tool does not know.
    #[error("unknown argument {0}; the only option is --check")]
    Usage(String),
}

impl Error {
    /// An I/O error on `path`.
    pub(crate) fn io(path: &Path, error: &std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        }
    }
}

fn run() -> Result<String, Error> {
    let mut check = false;
    for arg in std::env::args().skip(1) {
        if arg == "--check" {
            check = true;
        } else {
            return Err(Error::Usage(arg));
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let upstream = root.join("tablets/OpenTabletDriver");
    let configurations = upstream.join("OpenTabletDriver.Configurations");
    let tablets = model::load(&configurations.join("Configurations"))?;
    // The parsers of the configurations, and the base parser of the plugin API.
    let parsers = model::parser_classes(&[
        &configurations.join("Parsers"),
        &upstream.join("OpenTabletDriver.Plugin/Tablet"),
    ])?;
    let problems = model::validate(&tablets, &parsers);
    if !problems.is_empty() {
        return Err(Error::Invalid(problems));
    }

    let out_dir = root.join("crates/ntd-tablets/src/generated");
    std::fs::create_dir_all(&out_dir).map_err(|e| Error::io(&out_dir, &e))?;
    for (name, content) in emit::files(&tablets) {
        let path = out_dir.join(name);
        if check {
            if std::fs::read_to_string(&path).ok().as_deref() != Some(content.as_str()) {
                return Err(Error::OutOfDate(path.display().to_string()));
            }
        } else {
            std::fs::write(&path, content).map_err(|e| Error::io(&path, &e))?;
        }
    }
    Ok(format!("{} tablet definitions", tablets.len()))
}

fn main() -> ExitCode {
    match run() {
        Ok(summary) => {
            let _ = writeln!(std::io::stdout(), "ntd-codegen: {summary}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "ntd-codegen: {error}");
            ExitCode::FAILURE
        }
    }
}
