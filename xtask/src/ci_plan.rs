//! `cargo xtask ci-plan`: which crates a change touches, and so which CI jobs to run.
//!
//! The command reads the workspace from `cargo metadata`, maps the changed files to crates, and
//! adds every crate that depends on them. It runs everything instead when the event is not a pull
//! request (push to `main`, tag, nightly, manual run) or when a global file changed. The result is
//! printed, and written to `$GITHUB_OUTPUT` when that variable is set, for the jobs that read it:
//!
//! - `full`: `true` when everything runs;
//! - `any`: `true` when at least one crate is affected;
//! - `crates`: the affected crates, as a JSON array;
//! - `packages`: the same crates as `-p name` arguments for cargo;
//! - `matrix`: the operating systems to test on, as a JSON object for `strategy.matrix`.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, ExitCode};

/// Files whose change can affect every crate: the whole CI runs.
const GLOBAL_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "clippy.toml",
    "rustfmt.toml",
    "deny.toml",
];

/// Folders whose change can affect every crate.
const GLOBAL_DIRS: &[&str] = &[".github/", ".cargo/"];

/// What the root crate owns. Its manifest is at the root of the repository, so without this list
/// every file of the repository would belong to it.
const ROOT_CRATE_DIRS: &[&str] = &["src/", "qa/", "tablets/", "resources/", "build.rs"];

/// One workspace crate.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Crate {
    /// Package name.
    name: String,
    /// Folder of its manifest, relative to the repository root, with `/` separators and a
    /// trailing `/`; empty for the root crate.
    dir: String,
    /// Names of the workspace crates it depends on.
    dependencies: Vec<String>,
}

/// The crates of the workspace.
#[derive(Debug, Default)]
struct Graph {
    crates: Vec<Crate>,
}

/// What to run.
#[derive(Debug, PartialEq, Eq)]
struct Plan {
    /// Everything runs, whatever changed.
    full: bool,
    /// The affected crates, sorted by name.
    crates: Vec<String>,
}

impl Graph {
    /// The crate that owns `file`, the one with the longest matching folder.
    fn owner(&self, file: &str) -> Option<&Crate> {
        self.crates
            .iter()
            .filter(|c| {
                if c.dir.is_empty() {
                    ROOT_CRATE_DIRS.iter().any(|dir| file.starts_with(dir))
                } else {
                    file.starts_with(&c.dir)
                }
            })
            .max_by_key(|c| c.dir.len())
    }

    /// The crates in `start`, plus every crate that depends on one of them, directly or not.
    fn with_dependents(&self, start: &BTreeSet<String>) -> BTreeSet<String> {
        let mut affected = start.clone();
        loop {
            let before = affected.len();
            for krate in &self.crates {
                if krate.dependencies.iter().any(|dep| affected.contains(dep)) {
                    affected.insert(krate.name.clone());
                }
            }
            if affected.len() == before {
                return affected;
            }
        }
    }

    fn all_names(&self) -> BTreeSet<String> {
        self.crates.iter().map(|c| c.name.clone()).collect()
    }
}

/// Decides what to run for `files` changed by an event named `event`.
fn plan(graph: &Graph, files: &[String], event: &str) -> Plan {
    let global = files.iter().any(|f| {
        GLOBAL_FILES.contains(&f.as_str()) || GLOBAL_DIRS.iter().any(|dir| f.starts_with(dir))
    });
    if event != "pull_request" || global {
        return Plan {
            full: true,
            crates: graph.all_names().into_iter().collect(),
        };
    }
    let touched: BTreeSet<String> = files
        .iter()
        .filter_map(|f| graph.owner(f))
        .map(|c| c.name.clone())
        .collect();
    Plan {
        full: false,
        crates: graph.with_dependents(&touched).into_iter().collect(),
    }
}

/// The test matrix: Windows and Linux on x64 for a pull request, ARM64 as well for a full run.
fn matrix(full: bool) -> Value {
    let mut include = vec![
        json!({ "os": "ubuntu-latest", "name": "Linux x64" }),
        json!({ "os": "windows-latest", "name": "Windows x64" }),
    ];
    if full {
        include.push(json!({ "os": "ubuntu-24.04-arm", "name": "Linux ARM64" }));
        include.push(json!({ "os": "windows-11-arm", "name": "Windows ARM64" }));
    }
    json!({ "include": include })
}

/// Reads the workspace crates from the output of `cargo metadata --no-deps`.
fn graph_from_metadata(metadata: &str, root: &Path) -> Result<Graph, String> {
    let value: Value = serde_json::from_str(metadata).map_err(|e| format!("bad metadata: {e}"))?;
    let packages = value
        .get("packages")
        .and_then(Value::as_array)
        .ok_or("the metadata has no `packages`")?;
    let names: BTreeSet<&str> = packages
        .iter()
        .filter_map(|p| p.get("name").and_then(Value::as_str))
        .collect();
    let root = root.to_string_lossy().replace('\\', "/");

    let mut crates = Vec::new();
    for package in packages {
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .ok_or("a package has no name")?;
        let manifest = package
            .get("manifest_path")
            .and_then(Value::as_str)
            .ok_or("a package has no manifest_path")?
            .replace('\\', "/");
        let dir = relative_dir(&manifest, &root)
            .ok_or_else(|| format!("{manifest} is not under {root}"))?;
        let dependencies = package
            .get("dependencies")
            .and_then(Value::as_array)
            .map(|deps| {
                deps.iter()
                    .filter_map(|d| d.get("name").and_then(Value::as_str))
                    .filter(|d| names.contains(d) && *d != name)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        crates.push(Crate {
            name: name.to_owned(),
            dir,
            dependencies,
        });
    }
    Ok(Graph { crates })
}

/// The folder of `manifest` relative to `root`, with a trailing `/`; empty at the root.
fn relative_dir(manifest: &str, root: &str) -> Option<String> {
    let root = root.trim_start_matches("//?/").trim_end_matches('/');
    let manifest = manifest.trim_start_matches("//?/");
    let folder = manifest.strip_suffix("Cargo.toml")?;
    let relative = folder.strip_prefix(root)?.trim_start_matches('/');
    Some(relative.to_owned())
}

/// Runs the command: `cargo xtask ci-plan [--base <rev>] [--event <name>] [--files <file>]`.
pub(crate) fn run(root: &Path, args: &[String]) -> ExitCode {
    match try_run(root, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("ci-plan: {message}");
            ExitCode::FAILURE
        }
    }
}

fn try_run(root: &Path, args: &[String]) -> Result<(), String> {
    let mut base = "origin/main".to_owned();
    let mut event = std::env::var("GITHUB_EVENT_NAME").unwrap_or_else(|_| "pull_request".into());
    let mut files_from: Option<String> = None;
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        let value = args
            .get(index + 1)
            .cloned()
            .ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--base" => base = value?,
            "--event" => event = value?,
            "--files" => files_from = Some(value?),
            other => return Err(format!("unknown argument {other}")),
        }
        index += 2;
    }

    let metadata = command_output(
        Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .current_dir(root),
    )?;
    let graph = graph_from_metadata(&metadata, root)?;

    let listing = match files_from {
        Some(path) => {
            std::fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?
        }
        None => command_output(
            Command::new("git")
                .args(["diff", "--name-only", &format!("{base}...HEAD")])
                .current_dir(root),
        )?,
    };
    let files: Vec<String> = listing
        .lines()
        .map(|l| l.trim().replace('\\', "/"))
        .filter(|l| !l.is_empty())
        .collect();

    let result = plan(&graph, &files, &event);
    report(&result, files.len());
    write_outputs(&result)
}

/// Runs a command and returns its standard output.
fn command_output(command: &mut Command) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|e| format!("cannot run {command:?}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{command:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("{command:?} printed non UTF-8 text: {e}"))
}

fn report(result: &Plan, changed: usize) {
    let mode = if result.full {
        "full run"
    } else {
        "targeted run"
    };
    println!("ci-plan: {changed} changed file(s), {mode}");
    println!(
        "crates: {}",
        if result.crates.is_empty() {
            "(none)".into()
        } else {
            result.crates.join(", ")
        }
    );
}

/// Appends the outputs to the file named by `$GITHUB_OUTPUT`, if any.
fn write_outputs(result: &Plan) -> Result<(), String> {
    let Ok(path) = std::env::var("GITHUB_OUTPUT") else {
        return Ok(());
    };
    let packages: String = result
        .crates
        .iter()
        .map(|c| format!("-p {c}"))
        .collect::<Vec<_>>()
        .join(" ");
    let outputs: BTreeMap<&str, String> = BTreeMap::from([
        ("full", result.full.to_string()),
        ("any", (!result.crates.is_empty()).to_string()),
        ("crates", json!(result.crates).to_string()),
        ("packages", packages),
        ("matrix", matrix(result.full).to_string()),
    ]);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|e| format!("cannot open {path}: {e}"))?;
    for (key, value) in outputs {
        writeln!(file, "{key}={value}").map_err(|e| format!("cannot write {path}: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(name: &str, dir: &str, deps: &[&str]) -> Crate {
        Crate {
            name: name.into(),
            dir: dir.into(),
            dependencies: deps.iter().map(|d| (*d).to_owned()).collect(),
        }
    }

    /// api <- core (root) <- sdk, desktop; plugin <- api; tool alone.
    fn fake() -> Graph {
        Graph {
            crates: vec![
                krate("core", "", &["api"]),
                krate("api", "crates/api/", &[]),
                krate("sdk", "apps/sdk/", &["core", "api"]),
                krate("desktop", "apps/desktop/", &["core", "api"]),
                krate("plugin", "plugins/plugin/", &["api"]),
                krate("tool", "tools/tool/", &[]),
            ],
        }
    }

    fn files(list: &[&str]) -> Vec<String> {
        list.iter().map(|f| (*f).to_owned()).collect()
    }

    fn names(plan: &Plan) -> Vec<&str> {
        plan.crates.iter().map(String::as_str).collect()
    }

    #[test]
    fn a_documentation_change_touches_no_crate() {
        let result = plan(
            &fake(),
            &files(&["docs/a.md", "README.md", "frontend/src/a.ts"]),
            "pull_request",
        );
        assert!(!result.full);
        assert!(result.crates.is_empty());
    }

    #[test]
    fn a_leaf_crate_touches_only_itself() {
        let result = plan(&fake(), &files(&["tools/tool/src/main.rs"]), "pull_request");
        assert_eq!(names(&result), vec!["tool"]);
        let result = plan(
            &fake(),
            &files(&["plugins/plugin/src/lib.rs"]),
            "pull_request",
        );
        assert_eq!(names(&result), vec!["plugin"]);
    }

    #[test]
    fn a_change_reaches_every_crate_that_depends_on_it() {
        let result = plan(&fake(), &files(&["crates/api/src/lib.rs"]), "pull_request");
        assert_eq!(
            names(&result),
            vec!["api", "core", "desktop", "plugin", "sdk"]
        );
    }

    #[test]
    fn dependents_are_found_through_other_crates() {
        let graph = Graph {
            crates: vec![
                krate("a", "a/", &[]),
                krate("b", "b/", &["a"]),
                krate("c", "c/", &["b"]),
            ],
        };
        let result = plan(&graph, &files(&["a/src/lib.rs"]), "pull_request");
        assert_eq!(names(&result), vec!["a", "b", "c"]);
    }

    #[test]
    fn the_root_crate_owns_only_its_folders() {
        let graph = fake();
        assert_eq!(
            graph.owner("src/engine/x.rs").map(|c| c.name.as_str()),
            Some("core")
        );
        assert_eq!(
            graph
                .owner("qa/golden/a/snapshot.txt")
                .map(|c| c.name.as_str()),
            Some("core")
        );
        assert!(graph.owner("docs/a.md").is_none());
        assert!(graph.owner("frontend/package.json").is_none());
    }

    #[test]
    fn the_longest_folder_wins() {
        let graph = Graph {
            crates: vec![
                krate("outer", "apps/", &[]),
                krate("inner", "apps/inner/", &[]),
            ],
        };
        assert_eq!(
            graph.owner("apps/inner/src/a.rs").map(|c| c.name.as_str()),
            Some("inner")
        );
        assert_eq!(
            graph.owner("apps/other/a.rs").map(|c| c.name.as_str()),
            Some("outer")
        );
    }

    #[test]
    fn a_global_file_runs_everything() {
        for file in [
            "Cargo.lock",
            "Cargo.toml",
            "rust-toolchain.toml",
            "deny.toml",
            ".github/workflows/ci.yml",
            ".cargo/config.toml",
        ] {
            let result = plan(&fake(), &files(&[file]), "pull_request");
            assert!(result.full, "{file}");
            assert_eq!(result.crates.len(), 6, "{file}");
        }
    }

    #[test]
    fn anything_but_a_pull_request_runs_everything() {
        for event in ["push", "schedule", "workflow_dispatch", "release"] {
            let result = plan(&fake(), &files(&["docs/a.md"]), event);
            assert!(result.full, "{event}");
            assert_eq!(result.crates.len(), 6, "{event}");
        }
    }

    #[test]
    fn the_matrix_adds_arm64_for_a_full_run_only() {
        let count = |full: bool| {
            matrix(full)
                .get("include")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        };
        assert_eq!(count(false), 2);
        assert_eq!(count(true), 4);
    }

    #[test]
    fn the_graph_is_read_from_cargo_metadata() {
        let metadata = json!({
            "packages": [
                { "name": "app", "manifest_path": "C:\\repo\\apps\\app\\Cargo.toml",
                  "dependencies": [ { "name": "lib" }, { "name": "serde" } ] },
                { "name": "lib", "manifest_path": "C:\\repo\\crates\\lib\\Cargo.toml",
                  "dependencies": [] },
                { "name": "root", "manifest_path": "C:\\repo\\Cargo.toml",
                  "dependencies": [ { "name": "lib" } ] }
            ]
        })
        .to_string();
        let graph = graph_from_metadata(&metadata, Path::new("C:\\repo")).unwrap_or_default();
        let by_name: BTreeMap<&str, &Crate> =
            graph.crates.iter().map(|c| (c.name.as_str(), c)).collect();
        assert_eq!(
            by_name.get("app").map(|c| c.dir.as_str()),
            Some("apps/app/")
        );
        assert_eq!(
            by_name.get("app").map(|c| c.dependencies.clone()),
            Some(vec!["lib".to_owned()])
        );
        assert_eq!(
            by_name.get("lib").map(|c| c.dir.as_str()),
            Some("crates/lib/")
        );
        assert_eq!(by_name.get("root").map(|c| c.dir.as_str()), Some(""));
    }

    #[test]
    fn bad_metadata_is_an_error() {
        assert!(graph_from_metadata("not json", Path::new(".")).is_err());
        assert!(graph_from_metadata("{}", Path::new(".")).is_err());
    }
}
