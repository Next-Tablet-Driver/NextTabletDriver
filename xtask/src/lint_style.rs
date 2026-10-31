//! `cargo xtask lint-style`: what rustfmt and clippy cannot check.
//!
//! On stable, rustfmt wraps neither comments nor long strings and clippy has no line length lint,
//! so this task checks every `.rs` file of the repository for:
//!
//! 1. lines longer than 100 characters, comments and strings included (a line that holds a URL is
//!    exempt, since a URL cannot be split);
//! 2. decorative comments: banners (4 or more `=`, `-`, `*` or `#` in a row) and emojis;
//! 3. a to-do marker without an issue number, `TODO(#123)` being the only accepted form;
//! 4. `allow(unsafe_code)` in a crate that is not allowed to use `unsafe`;
//! 5. `#[rustfmt::skip]` without a comment right above it that says why;
//! 6. a direct read of the clock or a sleep in a crate that gets its time injected (rule 37 of
//!    the plan).

use std::path::Path;
use std::process::ExitCode;

/// Longest allowed line, in characters.
const MAX_LINE: usize = 100;

/// Crates that never read the time themselves: they take a `Clock` (rule 37).
const CLOCK_INJECTED: &[&str] = &["crates/ntd-pipeline/"];

/// Calls that read the time or wait, which a crate of [`CLOCK_INJECTED`] must not make.
const DIRECT_TIME_CALLS: &[&str] = &["Instant::now(", "SystemTime::now(", "sleep("];

/// Folders that hold no code of ours: build output, dependencies, the submodule and generated
/// files.
const SKIPPED_DIRS: &[&str] = &["target", "node_modules", ".git", "tablets", "dist"];

/// Paths (relative to the repository root) where `unsafe` may be allowed. Until the root crate is
/// split into the crates of the v3 plan, this is the crates that already call into the operating
/// system, a plugin or the shared memory; the list then shrinks to the FFI crates.
const UNSAFE_ALLOWED: &[&str] = &[
    "src/",
    "apps/sdk/",
    "apps/desktop/",
    "crates/ntd-hid/",
    "crates/ntd-interop/",
    "crates/ntd-output/",
    "crates/ntd-plugin-api/",
    "crates/ntd-platform/",
];

/// One broken rule.
#[derive(Debug, PartialEq, Eq)]
struct Violation {
    /// 1-based line number.
    line: usize,
    /// What is wrong.
    message: String,
}

impl Violation {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

/// Checks every `.rs` file under `root` and prints the violations. Succeeds when there are none.
pub(crate) fn run(root: &Path) -> ExitCode {
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort();

    let mut total = 0_usize;
    for relative in &files {
        let Ok(source) = std::fs::read_to_string(root.join(relative)) else {
            eprintln!("{relative}: cannot be read as UTF-8 text");
            total += 1;
            continue;
        };
        for violation in check_source(relative, &source) {
            println!("{relative}:{}: {}", violation.line, violation.message);
            total += 1;
        }
    }

    if total == 0 {
        println!("lint-style: {} files checked, no problem", files.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("lint-style: {total} problem(s) in {} files", files.len());
        ExitCode::FAILURE
    }
}

/// Collects the `.rs` files below `dir`, as paths relative to `root` with `/` separators.
fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !SKIPPED_DIRS.contains(&name.as_ref()) {
                collect(root, &path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && let Ok(relative) = path.strip_prefix(root)
        {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Applies the six rules to one file. `path` is relative to the repository root.
fn check_source(path: &str, source: &str) -> Vec<Violation> {
    if is_generated(source) {
        return Vec::new();
    }
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        if line.chars().count() > MAX_LINE
            && !line.contains("http://")
            && !line.contains("https://")
        {
            found.push(Violation::new(
                number,
                format!(
                    "line is {} characters long, the limit is {MAX_LINE}",
                    line.chars().count()
                ),
            ));
        }
        if let Some(comment) = comment_text(line) {
            check_comment(number, comment, &mut found);
        }
        if is_banner_line(line) {
            found.push(Violation::new(number, "decorative comment banner"));
        }
    }

    check_unsafe(path, &lines, &mut found);
    check_rustfmt_skip(&lines, &mut found);
    check_clock(path, &lines, &mut found);
    found.sort_by_key(|v| v.line);
    found.dedup();
    found
}

/// Files that say they are generated are not ours to format.
fn is_generated(source: &str) -> bool {
    source.lines().take(5).any(|line| {
        let lower = line.to_lowercase();
        lower.contains("@generated") || lower.contains("generated by")
    })
}

/// The text of the `//` comment on this line, without the slashes (and without a `!` or a third
/// `/`), or `None` when the line has no comment. A `//` inside a string is not a comment.
fn comment_text(line: &str) -> Option<&str> {
    let mut in_string = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if byte == b'\'' && bytes.get(i + 2) == Some(&b'\'') {
            // A character literal such as '"' or '/': skip it whole.
            i += 2;
        } else if byte == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let rest = line.get(i + 2..)?;
            return Some(rest.trim_start_matches(['/', '!']));
        }
        i += 1;
    }
    None
}

/// Banners made of the comment markers themselves: a row of `/` (`////`) or of stars after `/*`.
fn is_banner_line(line: &str) -> bool {
    let start = line.trim_start();
    start.starts_with("////")
        || start
            .strip_prefix("/*")
            .is_some_and(|rest| rest.starts_with("***"))
}

/// Rules 2 and 3 on the text of one comment.
fn check_comment(number: usize, comment: &str, found: &mut Vec<Violation>) {
    let trimmed = comment.trim_start();
    let run = trimmed.chars().take_while(|c| "=-*#".contains(*c)).count();
    if run >= 4 && trimmed.chars().next().is_some_and(|c| "=-*#".contains(c)) {
        found.push(Violation::new(number, "decorative comment banner"));
    }
    if comment.chars().any(is_emoji) {
        found.push(Violation::new(number, "emoji in a comment"));
    }
    for keyword in ["TODO", "FIXME"] {
        if mentions_without_issue(comment, keyword) {
            found.push(Violation::new(
                number,
                format!("{keyword} without an issue number, write {keyword}(#123)"),
            ));
        }
    }
}

/// Whether `comment` has the word `keyword` that is not followed by `(#digits)`.
fn mentions_without_issue(comment: &str, keyword: &str) -> bool {
    let mut from = 0;
    while let Some(offset) = comment.get(from..).and_then(|rest| rest.find(keyword)) {
        let start = from + offset;
        let end = start + keyword.len();
        let before_ok = comment
            .get(..start)
            .and_then(|s| s.chars().next_back())
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        let after = comment.get(end..).unwrap_or("");
        let after_ok = after
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        if before_ok && after_ok && !has_issue_number(after) {
            return true;
        }
        from = end;
    }
    false
}

/// Whether `text` starts with `(#` and digits, then `)`.
fn has_issue_number(text: &str) -> bool {
    text.strip_prefix("(#").is_some_and(|rest| {
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        digits > 0 && rest.get(digits..).is_some_and(|tail| tail.starts_with(')'))
    })
}

/// Pictographs and symbols that are used as decoration.
const fn is_emoji(c: char) -> bool {
    matches!(c as u32, 0x1F300..=0x1FAFF | 0x2600..=0x27BF | 0x2B50 | 0xFE0F)
}

/// Rule 4: `allow(unsafe_code)` outside the crates that may use `unsafe`.
fn check_unsafe(path: &str, lines: &[&str], found: &mut Vec<Violation>) {
    if UNSAFE_ALLOWED.iter().any(|prefix| path.starts_with(prefix)) {
        return;
    }
    for (index, line) in lines.iter().enumerate() {
        let start = line.trim_start();
        let opens = ["#![allow(", "#[allow(", "#![expect(", "#[expect("]
            .iter()
            .any(|opening| start.starts_with(opening));
        if !opens {
            continue;
        }
        // The attribute may be wrapped over several lines by rustfmt.
        let mut text = String::new();
        for part in lines.iter().skip(index).take(8) {
            text.extend(part.chars().filter(|c| !c.is_whitespace()));
            if part.trim_end().ends_with(']') {
                break;
            }
        }
        let lints = text.split("reason=").next().unwrap_or("");
        if lints.contains("unsafe_code") {
            found.push(Violation::new(
                index + 1,
                "allow(unsafe_code) outside the crates that may use unsafe",
            ));
        }
    }
}

/// Rule 5: `#[rustfmt::skip]` needs a comment above it that says why.
fn check_rustfmt_skip(lines: &[&str], found: &mut Vec<Violation>) {
    for (index, line) in lines.iter().enumerate() {
        if !line.trim_start().starts_with("#[rustfmt::skip")
            && !line.trim_start().starts_with("#![rustfmt::skip")
        {
            continue;
        }
        let justified = lines
            .iter()
            .take(index)
            .rev()
            .find(|previous| !previous.trim().is_empty())
            .is_some_and(|previous| previous.trim_start().starts_with("//"));
        if !justified {
            found.push(Violation::new(
                index + 1,
                "#[rustfmt::skip] without a comment that says why",
            ));
        }
    }
}

/// Rule 6: no direct read of the clock and no sleep in a crate that gets its time injected.
fn check_clock(path: &str, lines: &[&str], found: &mut Vec<Violation>) {
    if !CLOCK_INJECTED.iter().any(|prefix| path.starts_with(prefix)) {
        return;
    }
    for (index, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        if let Some(call) = DIRECT_TIME_CALLS.iter().find(|call| line.contains(**call)) {
            found.push(Violation::new(
                index + 1,
                format!("`{call}` in a crate that takes a Clock (rule 37)"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(path: &str, source: &str) -> Vec<String> {
        check_source(path, source)
            .into_iter()
            .map(|v| format!("{}: {}", v.line, v.message))
            .collect()
    }

    #[test]
    fn a_line_of_exactly_100_characters_is_accepted() {
        let line = format!("let s = \"{}\";", "a".repeat(100 - 11));
        assert_eq!(line.chars().count(), 100);
        assert!(messages("src/a.rs", &line).is_empty());
    }

    #[test]
    fn a_line_of_101_characters_is_reported_with_its_number() {
        let long = format!("let s = \"{}\";", "a".repeat(100 - 10));
        let source = format!("fn a() {{}}\n{long}\n");
        let found = messages("src/a.rs", &source);
        assert_eq!(found.len(), 1);
        assert!(
            found
                .first()
                .is_some_and(|m| m.starts_with("2: line is 101 characters"))
        );
    }

    #[test]
    fn a_long_comment_is_reported_like_code() {
        let source = format!("// {}\n", "word ".repeat(30));
        assert_eq!(messages("src/a.rs", &source).len(), 1);
    }

    #[test]
    fn a_long_line_with_a_url_is_accepted() {
        let source = format!("/// See https://example.com/{}\n", "a".repeat(120));
        assert!(messages("src/a.rs", &source).is_empty());
    }

    #[test]
    fn characters_are_counted_not_bytes() {
        let source = format!("// {}\n", "é".repeat(90));
        assert!(messages("src/a.rs", &source).is_empty());
    }

    #[test]
    fn banners_are_reported() {
        for banner in [
            "// ====== PARSING ======",
            "//-----",
            "//// ****",
            "// ####",
        ] {
            assert_eq!(messages("src/a.rs", banner).len(), 1, "{banner}");
        }
        assert_eq!(messages("src/a.rs", "/***** header").len(), 1);
    }

    #[test]
    fn short_runs_and_markdown_are_not_banners() {
        for text in [
            "// ---",
            "// a == b",
            "/// - item",
            "/// # Errors",
            "// -> result",
        ] {
            assert!(messages("src/a.rs", text).is_empty(), "{text}");
        }
    }

    #[test]
    fn an_emoji_in_a_comment_is_reported() {
        assert_eq!(
            messages("src/a.rs", "let a = 1; // mask \u{1F3AF}").len(),
            1
        );
        assert!(messages("src/a.rs", "// plain ascii and é").is_empty());
    }

    #[test]
    fn a_todo_needs_an_issue_number() {
        assert_eq!(messages("src/a.rs", "// TODO: later").len(), 1);
        assert_eq!(messages("src/a.rs", "// FIXME").len(), 1);
        assert_eq!(messages("src/a.rs", "// TODO(#12x): bad number").len(), 1);
        assert!(messages("src/a.rs", "// TODO(#123): later").is_empty());
        assert!(messages("src/a.rs", "// FIXME(#7)").is_empty());
    }

    #[test]
    fn words_that_only_contain_todo_are_not_reported() {
        assert!(messages("src/a.rs", "// the TODOS table, MASTODON, FIXMEs").is_empty());
    }

    #[test]
    fn slashes_inside_a_string_are_not_a_comment() {
        let source = "let url = \"http://x/TODO\"; let t = \"// ====\";\n";
        assert!(messages("src/a.rs", source).is_empty());
    }

    #[test]
    fn unsafe_may_be_allowed_in_the_listed_crates_only() {
        let attr = "#![allow(unsafe_code, reason = \"ffi\")]\n";
        assert!(messages("src/engine/x.rs", attr).is_empty());
        assert!(messages("apps/sdk/src/ffi.rs", attr).is_empty());
        assert_eq!(messages("plugins/p/src/lib.rs", attr).len(), 1);
        assert_eq!(
            messages("tools/ntd-recorder/src/main.rs", "#[allow(unsafe_code)]\n").len(),
            1
        );
    }

    #[test]
    fn a_wrapped_unsafe_allow_is_found() {
        let source = "#![allow(\n    clippy::panic,\n    unsafe_code,\n    reason = \"x\"\n)]\n";
        assert_eq!(messages("tools/t/src/main.rs", source).len(), 1);
    }

    #[test]
    fn an_allow_that_only_mentions_unsafe_in_its_reason_is_accepted() {
        let source = "#![allow(clippy::panic, reason = \"unsafe_code is not used\")]\n";
        assert!(messages("tools/t/src/main.rs", source).is_empty());
    }

    #[test]
    fn the_pipeline_may_not_read_the_clock_or_sleep() {
        let path = "crates/ntd-pipeline/src/projector.rs";
        assert_eq!(
            messages(
                path,
                "let t = Instant::now();
"
            )
            .len(),
            1
        );
        assert_eq!(
            messages(
                path,
                "std::thread::sleep(d);
"
            )
            .len(),
            1
        );
        assert!(
            messages(
                path,
                "// Instant::now() is read by the clock
"
            )
            .is_empty()
        );
        assert!(
            messages(
                "src/engine/x.rs",
                "let t = Instant::now();
"
            )
            .is_empty()
        );
    }

    #[test]
    fn rustfmt_skip_needs_a_comment_above() {
        assert_eq!(
            messages("src/a.rs", "#[rustfmt::skip]\nconst T: [u8; 2] = [1,2];\n").len(),
            1
        );
        let ok = "// A lookup table laid out by hand: one row per key.\n#[rustfmt::skip]\nconst T: \
            [u8; 2] = [1,2];\n";
        assert!(messages("src/a.rs", ok).is_empty());
        let blank = "// why\n\n#[rustfmt::skip]\nfn a() {}\n";
        assert!(messages("src/a.rs", blank).is_empty());
    }

    #[test]
    fn a_generated_file_is_skipped() {
        let source = format!("// @generated by a tool\n{}\n", "x".repeat(200));
        assert!(messages("src/a.rs", &source).is_empty());
    }

    #[test]
    fn violations_come_out_in_line_order() {
        let source = "// TODO\nfn a() {}\n// ====\n";
        let lines: Vec<usize> = check_source("src/a.rs", source)
            .iter()
            .map(|v| v.line)
            .collect();
        assert_eq!(lines, vec![1, 3]);
    }

    #[test]
    fn the_repository_walk_skips_build_output() {
        let mut files = Vec::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        collect(&root, &root, &mut files);
        assert!(files.iter().any(|f| f == "xtask/src/lint_style.rs"));
        assert!(
            files
                .iter()
                .all(|f| !f.starts_with("target/") && !f.starts_with("tablets/"))
        );
    }
}
