# AGENTS.md

Instructions for AI coding assistants (and a short orientation for people). `CLAUDE.md` is a
symlink to this file, so every tool reads the same text.

## What this project is

NextTabletDriver is a low-latency driver for drawing tablets (osu!, drawing), written in Rust. It
reads HID reports from the tablet, parses them with ports of OpenTabletDriver's parsers, maps them
to the screen, runs a filter pipeline (native plugins), and injects the result as mouse or pen
input. The interface is a Svelte app in a Tauri shell.

The code is being rewritten in phases. **Read these before you start**:

- [`docs/v3-rewrite.md`](docs/v3-rewrite.md) is the plan. **§9 is normative** (the "DO NOT" rules)
  and **§17 is the roadmap**: take the first task of the current phase whose dependencies are done,
  work on that one task only, and never tick a box without running the checks below.
- [`docs/architecture.md`](docs/architecture.md), [`docs/development.md`](docs/development.md) and
  [`.github/CONTRIBUTING.md`](.github/CONTRIBUTING.md) describe the code as it is today.

## Layout today

```
Cargo.toml        one Cargo workspace, one Cargo.lock; the root crate (src/) is the engine
apps/desktop/     Tauri wiring (commands, events, tray); crate `app`
apps/sdk/         `ntd_sdk`, the C ABI for integrators
crates/           `ntd-core` (pure types and maths), `ntd-tablets` (definitions and parsers),
                  `ntd-pipeline` (processing stages and the `Filter` trait),
                  `ntd-test-support` (fakes of HID, output and clock: dev-dependency only),
                  `ntd-plugin-api` (the plugin ABI); the tablet tables of
                  `ntd-tablets` are generated, never edited (`cargo run --locked -p ntd-codegen`)
plugins/          the official plugins (workspace members)
tools/            `ntd-recorder` (captures a tablet), `ntd-codegen` (tablet tables), more to come
xtask/            `cargo xtask lint-style`, `cargo xtask ci-plan`
frontend/         Svelte 5 + TypeScript; `src/lib/infrastructure/` is the only layer that talks to
                  Tauri (`tauri/` and `os.ts`) and the only place that uses `console` (`log.ts`)
tablets/          OpenTabletDriver definitions (git submodule, LGPL-3.0, read-only for us)
qa/               benchmarks, fuzzers, property tests, `qa/golden/` (parser golden files)
```

The root crate is split into the crates of the plan (§4) in phase 2 onward.

## Before you say a task is done

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo nextest run --locked --workspace --all-features     # not `cargo test`: tests share state
cargo test --locked --all-features --doc -p next_tablet_driver -p ntd-core -p ntd-tablets -p ntd-pipeline
cargo xtask lint-style
cargo machete
typos
npm --prefix frontend run format:check
npm --prefix frontend run check
npm --prefix frontend run lint
npm --prefix frontend run test -- --run
```

Add `npm --prefix frontend run e2e` when you touch the interface. `cargo xtask ci-plan` lists the
crates your change touches. The desktop app needs the Tauri system libraries on Linux (see
`.github/actions/linux-deps/action.yml`).

## Rules that matter most (the full list is §9)

- **Never open a pull request.** Prepare the branch, the commits and the description; a human opens
  the PR. Never push to `main` (§14.3).
- **English only**: code, comments, commits, PR text, docs (rule 116).
- **No AI model name, session link or co-author trailer** in commits, code or docs (rule 87).
- **Do not invent** APIs, crates, Tauri commands or config fields: search the code first (rule 79).
- **A move is `git mv` plus fixed paths**, nothing else. One pull request, one intent.
- **Parsers**: never "simplify" a mask or a timing without understanding it; use `git blame`. A
  parser change needs golden files, and a capture comes from a real tablet: never invent or edit
  one (rules 80, 103).
- **`bindings.ts` is generated** from the Rust code: never edit it by hand
  (`UPDATE_BINDINGS=1 cargo test -p app --test bindings` regenerates it).
- **Lints are `deny`** in `[workspace.lints]`: do not downgrade one to get CI green. An `allow`
  needs `reason = "..."`. No `unwrap`, `expect`, `panic!`, indexing or `println!` on a runtime path.
- **100 columns** in `.rs`, `.ts` and `.svelte`. Comments explain why, never what.
- **Dependencies** are declared once in `[workspace.dependencies]` and must pass `cargo deny`.
- If a maintainer tells you something different from this file, follow the maintainer, except for
  the first rule.
