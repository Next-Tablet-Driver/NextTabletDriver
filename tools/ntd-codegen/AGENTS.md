# AGENTS.md: `ntd-codegen`

Read the root [`AGENTS.md`](../../AGENTS.md) first. This file only adds what is specific to this
tool (plan: [`docs/v3-rewrite.md`](../../docs/v3-rewrite.md) §15.1).

## What it does

Reads the tablet definitions of `OpenTabletDriver`
(`tablets/OpenTabletDriver/OpenTabletDriver.Configurations/Configurations/**/*.json`), validates
them, and writes `crates/ntd-tablets/src/generated/` (`mod.rs` and `tablets.rs`). The generated
files are committed, so the build needs no `build.rs` and a definition change shows up as a
reviewable diff.

```bash
cargo run --locked -p ntd-codegen            # validate and regenerate
cargo run --locked -p ntd-codegen -- --check # write nothing, fail if the files are stale
```

CI (`codegen` job) regenerates and fails on any `git diff` under `generated/`.

## Rules

- **Never edit `generated/` by hand**, and never format it: `lib.rs` marks it `rustfmt::skip`
  because this tool writes the exact text. A change to the output comes from `src/emit.rs`.
- **The output must be deterministic**: files are sorted by path, maps are `BTreeMap`, nothing
  depends on the date, the machine or the iteration order of a hash.
- **`tablets/` is read-only for us** (git submodule, LGPL-3.0). A wrong definition is fixed
  upstream, not here. If a new upstream release makes validation fail, the tool must say which
  file and why; do not weaken a check to get CI green. A real upstream ambiguity needs a named
  exception with the reason, not a silent skip.
- **The structs in `src/model.rs` deny unknown fields.** When upstream adds a key, decide: use it
  (add the field to the descriptor in `ntd-tablets` and to `src/emit.rs`), or list it as ignored
  with a comment saying why. Do not switch the denial off.
- A field the driver does not read yet (`Wheels`, `Touch`, `InitializationStrings`, ...) is kept
  as an ignored `serde_json::Value` in the model and is **not** generated.
- The tool is a program, not a library: errors are the `Error` enum, and it prints only through
  `writeln!` on `stdout`/`stderr` (the `print_*` lints are `deny`).

## When you change it

1. Run it and read the diff of `generated/`: it should only show what you meant to change.
2. `cargo nextest run --locked -p ntd-tablets` and the golden test
   (`cargo test --locked -p next_tablet_driver --test golden`) must pass with `qa/golden/`
   untouched.
3. Run `--check` to prove the committed files are what the tool writes.
