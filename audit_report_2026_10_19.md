# Rust Best Practices Audit — 2026-10-19

Scope: root crate `next_tablet_driver`, `sdk/` (`ntd_sdk`, `ntd_plugin_api`), `src-tauri/`,
`plugins/*`, `qa/`. Branch `docs/rewrite-documentation`.
Reference: Apollo GraphQL Rust Best Practices (chapters 1-9).

## How this audit was done

- Read `Cargo.toml` (lints, profiles), `clippy.toml`, `rustfmt.toml`, `deny.toml`, `ci.yml`.
- Ran `cargo clippy --workspace --all-targets --all-features` at the repo root: **0 warnings**.
  The root is not a Cargo workspace, so this only covered `next_tablet_driver`. `sdk/`,
  `src-tauri/` and `plugins/*` were **not** linted here (CI does it crate by crate).
- Grepped the whole tree for `unsafe`, `#[allow]`, `Result<_, String>`, `dyn`, locks, `clone()`.
- Read in full: `engine/state.rs`, `engine/plugins/{error,instance,loader,trust}.rs`,
  `engine/tablet_manager/polling.rs` (hot path), `drivers/parsers/mod.rs`, `filters/mod.rs`,
  `sdk/src/ffi.rs` (lock + guard helpers), `startup.rs` (head), `websocket.rs` (handshake).
- **Not** read line by line: the ~40 tablet parsers, `settings/*`, `telemetry/*`, `autoupdate/*`,
  the Linux code paths (Windows host), the frontend. Findings there are pattern-based only.
- No code was changed. Nothing was benchmarked.

## Overall verdict

The project is already well above average. Strict lints (`unwrap_used`, `expect_used`, `panic`,
`indexing_slicing` denied; pedantic and nursery on), `// SAFETY:` on 85 of 86 `unsafe {`
blocks, `catch_unwind` at every FFI boundary, a real error enum for plugins, poisoned-lock
recovery helpers, and benches that run the production packet path. The items below are the
remaining gaps, ordered by value.

## High priority

### H1. Plugin trust check has a time-of-check/time-of-use gap
`engine/plugins/manager.rs` (~L120-145) hashes the file (`sha256_file`), checks the hash against
the trust store, then calls `PluginLoader::load(&path)`, which opens the library **by path**.
A process that can write the plugins folder can swap the file between the hash and the load.
The trust store exists to stop exactly that.
Fix: read the file once into memory (or a private temp copy), hash that copy, and load that same
copy. Alternatively keep an open handle and re-hash right before `Library::new`, and document
the residual risk (plugins run native code in-process anyway).

### H2. `Result<_, String>` is the main error type of the public API (34 signatures)
Seen in `settings/{profile,themes,otd_import}.rs`, `engine/plugins/{manager,trust}.rs`,
`engine/injector/{linux,windows}.rs`, `logger.rs`, `startup.rs`, `supervisor.rs`, and 12 Tauri
commands. `PluginError` shows the team knows the right pattern, but it is used only inside the
plugin loader. Strings lose the source error, cannot be matched, and force callers to compare
text (e.g. `trust_plugin` builds an English message that the UI may need to translate).
Fix (Ch. 4): one `thiserror` enum per module (`ProfileError`, `ThemeError`, `TrustError`,
`InjectorError`...) with `#[from]` for `io::Error` / `serde_json::Error`. Convert to `String`
**only** in `src-tauri/src/commands.rs` (Tauri needs `Serialize`; `impl Serialize` via
`to_string()` on the enum keeps one conversion point). `thiserror` also lets you delete the
hand-written `Display`/`From`/`source` in `plugins/error.rs` (~60 lines).

### H3. Five bundled plugins and `src-tauri` do not use the shared lint policy
- `plugins/*/Cargo.toml` (5 crates) have **no** `[lints]` table, so none of the denies apply to
  code that runs in the 1000 Hz+ path and crosses an FFI boundary. `sdk/plugin_api` and
  `sdk/` do have them.
- `src-tauri/Cargo.toml` denies `unwrap/expect/panic` but lacks `indexing_slicing`,
  `pedantic`, `nursery`, `undocumented_unsafe_blocks`, `lossy_float_literal`.
- `src-tauri/Cargo.toml` still has template metadata: `description = "A Tauri App"`,
  `authors = ["you"]`, `license = ""`, `repository = ""`, and `rust-version = "1.90"` while the
  other crates say `1.99.0`.
Fix: add a Cargo workspace (`[workspace]` + `[workspace.lints]`, members use
`[lints] workspace = true`) so the policy lives in one place. Note the plugins are built as
separate crates with their own `Cargo.lock`; if they must stay outside the workspace, copy the
table and add a CI check that compares them.

## Medium priority

### M1. Parser selection by substring match is order-dependent
`drivers/parsers/mod.rs::create_parser` is a ~60-arm `match` using `name.contains(...)`, where
correctness depends on arm order (`GeniusReportParserV2` before `Genius`, `UCLogicV1` before
`UCLogic`, `XP_PenReportParser` before `XP_Pen`...). A new name that contains an older one
silently picks the wrong parser. The input is a string from a JSON config.
Fix (Ch. 6/7): parse the config string once into a `ParserKind` enum (`#[derive(Deserialize)]`
with explicit `#[serde(rename = ...)]`), then `match` on the enum (exhaustive, checked by the
compiler). Keep a table-driven test that every config file resolves to a known kind. The
fallback arm should be an explicit, logged `ParserKind::Fallback`, not "anything else".

### M2. Per-field `Mutex` in stateful parsers
`wacom/intuos_v1.rs` holds four `Mutex<u16/i8/i8/u8>` and locks each one per packet (8 lock
calls on a 1000 Hz path), only because `ReportParser: Send + Sync` takes `&self`. `pl.rs`,
`intuos_v2/v3` follow the same shape (`unwrap_or_reset` appears per lock).
Fix: one `Mutex<PrevState>` (single lock, consistent snapshot), or atomics
(`AtomicU16`/`AtomicU8` with `Relaxed`) since these are independent previous values, or change
the trait to `parse(&mut self, ...)` since one polling thread owns the driver. The last option
is the cleanest (Ch. 9: do not pay for sharing you do not use). Measure with the existing
`qa/benches/hot_path.rs` before and after.

### M3. `#[allow]` instead of `#[expect]`; 85 of them, ~75 are `float_cmp` in tests
All are `#[allow(clippy::float_cmp)]` on `mod tests`. The guideline is `#[expect(...)]` with a
reason, so a stale suppression becomes a warning. Simpler: add once in `src/lib.rs`
`#![cfg_attr(test, allow(clippy::float_cmp))]` (or `clippy.toml` can't do this, so use the
attribute) and delete the 75 per-file lines. Also:
- `#[allow(dead_code)]` at `drivers/generic.rs:19`, `engine/interop/lock.rs:29`,
  `engine/interop/lock/linux.rs:15` contradicts `dead_code = "deny"`. For RAII-only fields,
  rename to `_guard`/`_handle` instead.
- `too_many_arguments` is allowed 7 times. `polling.rs` has three functions with 10-13
  parameters (`run_polling_loop`, `process_packet`, `process_packet_for_bench`).

### M4. Polling-loop signatures should be grouped into types (Ch. 1, 6)
`process_packet` takes 13 arguments, including three `&mut` loop-state values
(`last_stats_update`, `last_packet_time`, and the config). A `PacketLoopState` struct already
exists for the bench; use it in production too, and bundle `pipeline + injector + filters +
local_config` into one `Engine`/`PacketContext` struct. This removes 3 `#[allow]`s and the
duplicate "bench copy" argument list, and makes `process_packet_for_bench` a one-line wrapper.

### M5. Hot path takes 3-4 separate write locks per packet
`process_packet` writes `processed_frame` and `tablet_data` under two `RwLock`s each packet,
`data.clone()` for the shared copy and `try_send(data)` for the UI (even when the UI is not
visible the first write still happens). `TabletData` is ~80 bytes (`Copy`-able: all fields are
`Copy`; it only derives `Clone`).
Fix: derive `Copy` on `TabletData` (Ch. 1: small plain data, avoids `.clone()` noise at ~10
call sites), and consider publishing one snapshot struct behind a single lock or a
`arc-swap`/seqlock so readers never block the engine. Only do the second part if the benches
show lock contention; the existing `try_write` on stats shows the right instinct.

### M6. `SharedState` exposes public fields with no invariants
`SharedState` and its sub-states (`ConfigState`, `PipelineState`, `LifecycleState`,
`engine/state.rs:100-237`) have every field `pub`, including `reload_requested` and
`version` that must change together. Any module (and the SDK) can break the protocol.
Fix: methods such as `request_reload()` / `bump_version()`; keep fields `pub(crate)`.

## Low priority

### L1. `missing_docs` is commented out (`Cargo.toml:91`)
Only `ntd_plugin_api` and `ntd_sdk` are real public APIs. Turn on `missing_docs = "warn"` there
first. For the root crate, which is `publish = false`, leaving it off is acceptable; at least
document `drivers::parsers::ReportParser` (public trait, no docs) and `TabletStatus`
variants.

### L2. Test naming
583 `#[test]` functions, mostly `test_<thing>`. Ch. 5 prefers `<thing>_should_<result>_when_<case>`
(newer files such as `plugins/error.rs` already do: `messages_name_what_went_wrong`).
Do this gradually for new tests only; a mass rename has no runtime value.

### L3. `unwrap_or("")` / `unwrap_or_default` in URL parsing (`websocket.rs:~90`)
`is_origin_allowed` falls back to an empty host, which is rejected, so it is safe. A comment
saying so, or `let Some(host) = ... else { return false }`, makes the intent obvious.

### L4. Threads are spawned unnamed and detached
`supervisor.rs` and `autoupdate/github.rs` use `std::thread::spawn` and drop the handle, while
`websocket.rs` correctly uses `thread::Builder::new().name(...)`. Named threads make crash
dumps and profilers readable. Keep the handle (or document why it is detached) for the engine
thread so shutdown can `join`.

### L5. `unused_self` allow in the normalizer
`pipeline/normalizer.rs:26` allows `unused_self`: make `execute` an associated function if
`self` is truly unused, and drop the allow.

### L6. CI could enforce what is already policy
`ci.yml` runs `clippy -D warnings` per crate, which is good. Add:
- A grep/CI step that fails if a new `#[allow(` is added outside tests (forces `#[expect]` with
  a reason).

## What is already good (keep it)

- `unwrap`/`expect`/`panic`/`indexing_slicing` denied at crate level, with test-only relaxations
  in `clippy.toml`. Zero production `unwrap` was found by clippy.
- Every `extern "C"` entry in `plugin_api` and `sdk/ffi.rs` goes through `catch_unwind`
  (`ffi_guard`), and `panic = "unwind"` is set on purpose with a comment explaining why.
- `unsafe`: 85/86 blocks carry a `SAFETY:` comment; `undocumented_unsafe_blocks` is on.
- Layout checks as `const _: () = assert!(size_of...)` in `sdk/ffi.rs`.
- Poisoned lock handling is explicit and named (`unwrap_or_log` vs `unwrap_or_reset`).
- Atomic file writes (tmp + rename) in the trust store and profiles.
- Hot path avoids heap allocation and samples the clock; there are benches and fuzz targets for
  the math.
- `cargo-deny` (bans, licenses, sources, advisories), coverage gate, and `Cargo.lock` committed.

## Suggested order of work

1. H1 (security, small change).
2. H3 (workspace lints; will surface new clippy findings in plugins and Tauri to fix).
3. H2 (error enums), one module per PR, starting with `plugins/trust.rs` and `settings/profile.rs`.
4. M3 (delete 75 `allow`s) — mechanical, low risk, shrinks the diff noise.
5. M1, then M2 + M4 + M5 with before/after numbers from `qa/benches/hot_path.rs`.
6. L-items opportunistically.
