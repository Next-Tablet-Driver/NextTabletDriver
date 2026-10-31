# NextTabletDriver v3: backend rewrite plan

> **Status**: framing document. It decides **why** the backend is being rebuilt, **how** it is
> done, and above all **what must not be done**.
>
> **Audience**: human contributors **and** AI assistants (LLMs). If you are an LLM working on
> this repository: the section ["What NOT to do"](#9-what-not-to-do) is **normative**. A rule
> that starts with "DO NOT" is a prohibition, not a suggestion. When in doubt, stop and ask
> instead of "doing your best".
>
> **Progress and next task**: see the [roadmap (§17)](#17-roadmap). It is the source of truth
> for what is done and what comes next.

---

## Contents

0. [Summary of decisions](#0-summary-of-decisions)
1. [Assessment of v2](#1-assessment-of-v2)
2. [New repository or same repository?](#2-new-repository-or-same-repository)
3. [Big-bang rewrite or incremental migration?](#3-big-bang-rewrite-or-incremental-migration)
4. [Target architecture: the workspace](#4-target-architecture-the-workspace)
5. [The backend ↔ frontend contract](#5-the-backend--frontend-contract)
6. [The frontend: kept, but its data layer changes](#6-the-frontend-kept-but-its-data-layer-changes)
7. [Migration plan by phase](#7-migration-plan-by-phase)
8. [Rules per crate](#8-rules-per-crate)
9. [What NOT to do](#9-what-not-to-do)
10. [PR checklist](#10-pr-checklist)
11. [Issues found outside the architecture](#11-issues-found-outside-the-architecture)
12. [Code quality](#12-code-quality)
13. [CI/CD](#13-cicd)
14. [Contributions, AI and AGENTS.md](#14-contributions-ai-and-agentsmd)
15. [Internal tools](#15-internal-tools)
16. [Target platforms](#16-target-platforms)
17. [Roadmap](#17-roadmap)

---

## 0. Summary of decisions

| Question | Decision | One-line reason |
|---|---|---|
| Rebuild the backend architecture? | **Yes** | Modules have circular dependencies that a single crate cannot prevent. |
| New "NextTabletDriver v3" repository? | **No. Same repository, version 3.0.0** | v2 auto-update is hardcoded to this repository. A new one would strand v2 users. |
| Big-bang rewrite? | **No. Incremental migration** | 18 parser families and a latency-sensitive pipeline: parity cannot be rewritten in one go. |
| Keep the frontend? | **Yes** | Svelte 5 + TS + tests + design system + E2E: the base is sound. |
| Change anything in the frontend? | **Yes, the data layer and the quality rules** | Hand-written types, catch-all `Config`, magic strings, `invoke` outside the layer. |
| How many crates? | **About fifteen, no more** | Enough to enforce dependency direction, not so many that it becomes bureaucracy. |
| Platforms? | **Windows, Linux, macOS; x64, 32-bit x86, ARM** | Not all in 3.0: v3 ships Windows and Linux first, but the architecture leaves room for every platform (§16). |
| Language? | **English everywhere** | Code, comments, commits, PRs, AGENTS.md and `docs/`, including this document. |
| 2.x releases during the rewrite? | **Total freeze** after one transition release | The only 2.x release is the one that ships the real updater key (§2, P0.2). No exceptions. |
| 2.x maintenance after 3.0? | **None** | Everyone moves to 3.0 through auto-update. |
| v2 plugins in v3? | **Not supported: new ABI v3** | Official plugins are rebuilt; third-party authors follow a migration guide. |
| Websocket and SDK? | **Versioned v3 protocol, websocket v2 kept for all of 3.x** | Overlays and integrations keep working; v2 websocket is removed in 4.0. |
| Telemetry? | **On by default (opt-out), as in v2** | Anonymous, no personal data, can be turned off in Settings. |

---

## 1. Assessment of v2

### 1.1 What works (do not break it)

- A **processing pipeline that is already fairly pure** (`src/engine/pipeline/transformer.rs`,
  `projector.rs`, `src/core/math/*`). It would be easy to test if it were isolated.
- A separate **plugin ABI** (`sdk/plugin_api`), plugin **trust verification**
  (`src/engine/plugins/trust.rs`, minisign) and real plugins (`plugins/*`).
- **Tablet descriptors as data** (`tablets/`) rather than code.
- **Quality**: clippy, `deny.toml`, coverage thresholds (`coverage-thresholds.json`), fuzzing
  (`qa/fuzz`), Playwright E2E and a native smoke test.
- A frontend organised by **features** (`frontend/src/lib/features/*`).

### 1.2 What does not work

The backend (≈ 23,500 lines) lives in **a single crate**, `next_tablet_driver`. Inside a crate,
`crate::anything` is always reachable, so **nothing prevents bad dependencies**. These exist today:

| Violation | Where | Why it matters |
|---|---|---|
| `drivers` → `engine` | `src/drivers/parsers/wacom/intuos_v1.rs:3`, `pl.rs:3` import `engine::state::{LockRecoveryExt, WriteRecoverExt}` | A parser (bytes → report) should be pure. Here it depends on the runtime and its locks. |
| `filters` ↔ `engine::plugins` (cycle) | `src/filters/mod.rs:8` ↔ `src/engine/plugins/manager.rs:12` | A cycle: neither can be tested or reused without the other. |
| `engine` → `application` | `src/engine/tablet_manager/owner.rs:38,61,252` call `application::telemetry::capture_event` | The low layer calls the high layer: telemetry is welded to the engine. |
| `engine` → `settings` (globals) | `src/engine/plugins/manager.rs:44-80` call `settings::get_plugins_dir()` / `get_settings_dir()` | Global paths: impossible to test with a temp directory or embed the engine elsewhere. |
| `engine` → `settings` (logs) | `src/engine/tablet_manager/polling.rs:390` | The polling loop knows about the settings module. |
| `desktop` feature | `Cargo.toml` `[features]` | It exists only to *remove* I/O for the SDK. A symptom of a missing split. |
| Three `Cargo.lock` files | root, `src-tauri/`, `sdk/` | Versions drift; `src-tauri/Cargo.toml` even says "copied: this crate has its own lockfile". |
| God files | `core/config/models.rs` (934 lines), `tablet_manager/polling.rs` (715 lines) | Too many responsibilities per file. |
| Catch-all `MappingConfig` | `core/config/models.rs` | One object holds the mapping area, bindings, websocket, theme, language, autostart… |
| Bindings and modes as `String` | `tip_binding: String`, `pen_button_bindings: Vec<String>` | Errors surface at runtime instead of compile time; the frontend has to guess the format. |

**Conclusion**: code is not what is missing. **Boundaries** are. Sub-crates are the cheapest way
to get boundaries that are **checked by the compiler**.

---

## 2. New repository or same repository?

### Decision: **same repository**; v3 is simply `3.0.0`.

### Why not a new repository

1. **Auto-update is wired to this repository.**
   - `src-tauri/tauri.conf.json` → `https://github.com/Next-Tablet-Driver/NextTabletDriver/releases/latest/download/latest.json`
   - `src/application/services/autoupdate/github.rs` → `api.github.com/repos/Next-Tablet-Driver/NextTabletDriver/releases`

   If v3 lives in another repository, **every v2 install stays on v2 forever** without knowing it.
   This is the strongest reason and it settles the question on its own.
2. **Git history**: `git blame` on a Wacom parser explains *why* a given byte is masked. That
   knowledge is irreplaceable. A new repository cuts it off.
3. **Issues, PRs, stars, search ranking, external links** (osu!, forums, wikis): they all point here.
4. **Plugin ecosystem**: plugin authors follow this repository and this SDK.
5. **"v3" is a version, not a product.** That is what SemVer is for: `3.0.0` means breaking changes.

### What we do instead

- Work on `main` in small PRs (§3). No long-lived `v3` branch.
- **Transition release, then total freeze.** Today's updater public key is a placeholder (§11).
  One last 2.x release ships the real key (roadmap P0.2) so that every v2 install can verify and
  accept 3.0. After it, **no 2.x release is published** until `3.0.0-beta.1`, not even a security
  fix.
- **No 2.x maintenance after 3.0**: there is no `release/2.x` branch. Everyone moves to 3.0
  through auto-update.
- If the project is ever renamed: **rename the repository** on GitHub (redirects are kept) **and**
  ship a release whose updater points to the new endpoint first. Never the other way round.

---

## 3. Big-bang rewrite or incremental migration?

### Decision: **incremental migration** ("strangler fig"), no big bang.

### Why

- The new architecture **maps almost one-to-one onto the existing modules** (§4). We **move**
  code and **cut dependencies**; we do not reinvent algorithms.
- The parsers encode years of reverse engineering. Rewriting them reintroduces bugs that were
  already fixed.
- The product is **latency sensitive** (osu!). A big bang mixes latency regressions with
  architecture changes: nobody can tell what broke.
- **Second-system effect**: a full rewrite always grows in scope ("while we're at it…") and never
  ships.
- The existing tests (unit, fuzz, E2E, native smoke) **are the safety net** throughout the move.
  A big bang loses them too.

### Golden rule

> After every PR, the binary builds, the tests pass and the application works for a user.
> There is never a state of "it will work once everything is done".

---

## 4. Target architecture: the workspace

### 4.1 Layout

```
Cargo.toml                 # [workspace] + [workspace.dependencies] + [workspace.lints]
Cargo.lock                 # ONE lockfile
crates/
  ntd-core/                # pure types and maths
  ntd-tablets/             # descriptors + parsers (bytes → report), pure
  ntd-hid/                 # HID I/O (hidapi)
  ntd-pipeline/            # transform, projection, filter chain, Filter trait
  ntd-plugin-api/          # stable C ABI (formerly sdk/plugin_api), published on crates.io
  ntd-plugin-host/         # loading, trust verification, plugin management
  ntd-output/              # OS input injection (SendInput / uinput)
  ntd-interop/             # shm, lock, inter-process command protocol
  ntd-settings/            # profiles, preferences, themes, OTD import
  ntd-platform/            # OS integration outside HID/injection: autostart, single
                           # instance, thread priority, high-resolution timer (§16.4)
  ntd-engine/              # runtime: threads, state, buttons, orchestration
  ntd-services/            # websocket, autoupdate, telemetry, supervisor
  ntd-ipc/                 # frontend contract DTOs + TS generation (tauri-specta)
  ntd-test-support/        # fakes (HID, output, clock), fixtures; dev-only
apps/
  desktop/                 # formerly src-tauri: Tauri wiring only
  sdk/                     # cdylib/staticlib for integrators
plugins/                   # official plugins, workspace members
frontend/                  # structure unchanged
tablets/                   # tablet data (consumed by ntd-codegen)
qa/fuzz/                   # fuzz targets on ntd-tablets, ntd-pipeline and ntd-interop
tools/
  ntd-codegen/             # tablets/*.json → generated Rust in ntd-tablets (§15.1)
  ntd-tester/              # HID replay → real engine → fake output (§15.2)
  ntd-recorder/            # tablet HID capture for issues (§15.3)
  ntd-stress/              # websocket and interop protocol stress (§15.5)
xtask/                     # dev tasks (TS types, lint-style, CI plan)
```

### 4.2 Responsibilities and dependencies

| Crate | Taken from | Depends on | I/O allowed |
|---|---|---|---|
| `ntd-core` | `core/input`, `core/math`, types from `core/config/models` that are really shared, base errors | — | **None** |
| `ntd-tablets` | `drivers/parsers`, `drivers/config_loader`, `drivers/models`, `drivers/config`, VID/PID matching from `drivers/detection` | core | **None** (descriptors are generated code) |
| `ntd-hid` | `drivers/hid_caps`, enumeration from `detection`, `tablet_manager/reader` | core, tablets | HID |
| `ntd-pipeline` | `engine/pipeline/*`, `filters` (chain + trait) | core | **None** |
| `ntd-plugin-api` | `sdk/plugin_api` | — | **None** |
| `ntd-plugin-host` | `engine/plugins/*` | pipeline, plugin-api | files, `libloading` |
| `ntd-output` | `engine/injector/*` | core | OS (injection) |
| `ntd-interop` | `engine/interop/*` | core | shm, OS locks |
| `ntd-settings` | `settings/*` | core | files (paths **injected**) |
| `ntd-platform` | autostart, single instance, thread priority and timer (today scattered across `startup.rs`, `application/`, `engine/`) | core | OS |
| `ntd-engine` | `engine/tablet_manager/*`, `engine/state` | core, tablets, hid, pipeline, plugin-host, output, interop, settings, platform | threads |
| `ntd-services` | `application/services/*`, `application/telemetry/*` | engine, settings | network |
| `ntd-ipc` | implicit DTOs of `src-tauri/src/commands.rs` + `frontend/.../tauri/commands.ts` | core, settings (types) | **None** |
| `apps/desktop` | `src-tauri` | engine, services, ipc, platform | Tauri |
| `apps/sdk` | `sdk` | core, tablets, hid, pipeline | through its dependencies |

### 4.3 Graph (arrows **always point down**)

```
                 apps/desktop                 apps/sdk
                /     |      \                /   |   \
        services     ipc      \          tablets hid pipeline
            |          \       \
          engine ───────┴───────┘
   ┌────┬───┼────────┬─────────┬─────────┬──────────┬──────────┐
  hid tablets pipeline plugin-host output  interop  settings  platform
   │    │      │         │   │     │        │         │          │
   └────┴──────┴─────────┘  plugin-api      └─────────┴──────────┴──► core
                 (everything depends on core, core depends on nothing)
```

### 4.4 The three inversions that break the cycles

1. **Filters ↔ plugins**: `ntd-pipeline` defines `trait Filter`. `ntd-plugin-host` provides a
   `PluginFilter: Filter`. The pipeline does not know plugins exist.
2. **Engine → telemetry**: `ntd-engine` publishes `EngineEvent`s (e.g. `DeviceLost`,
   `ParserError`) on a channel. `ntd-services` subscribes and decides whether to send telemetry.
3. **Engine → global paths**: a `Paths { settings, plugins, logs, … }` struct is built **once** in
   `apps/desktop` and passed as a parameter. No library crate calls `directories` itself.

### 4.5 Public engine interface

```rust
// ntd-engine: sketch, not a frozen API
pub struct Engine { /* private */ }

impl Engine {
    pub fn start(
        cfg: EngineConfig,
        deps: EngineDeps,
    ) -> Result<(EngineHandle, Receiver<EngineEvent>), EngineError>;
}

#[derive(Clone)]
pub struct EngineHandle { /* Sender<Command> */ }

impl EngineHandle {
    pub fn apply_mapping(&self, m: Mapping) -> Result<(), EngineError>;
    pub fn set_bindings(&self, b: Bindings) -> Result<(), EngineError>;
    pub fn set_filter_chain(&self, c: FilterChainConfig) -> Result<(), EngineError>;
    pub fn shutdown(self);
}

pub struct EngineDeps {
    pub hid: Box<dyn HidBackend>,      // real or fake
    pub output: Box<dyn OutputSink>,   // real or fake
    pub clock: Box<dyn Clock>,         // real or fake
    pub paths: Paths,
}
```

The engine is driven **only** through this handle. There is no global `SharedState` that every
Tauri command locks and mutates in its own way.

### 4.6 Workspace configuration

```toml
[workspace]
resolver = "3"
members = ["crates/*", "apps/*", "plugins/*", "tools/*", "xtask"]

[workspace.package]
edition = "2024"
rust-version = "1.99.0"   # MSRV, see §12.6
license = "MIT"
repository = "https://github.com/Next-Tablet-Driver/NextTabletDriver"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "2"
log = "0.4"
crossbeam-channel = "0.5"
hidapi = "2.6.3"
# … one version of each dependency for the whole workspace

# The full lint set is in §12.3.
```

Every crate: `[lints] workspace = true`, `serde.workspace = true`, etc. Every crate is
`publish = false` except `ntd-plugin-api` (§8).

---

## 5. The backend ↔ frontend contract

The frontend is kept, so **the contract is the most important thing to protect**.

### 5.1 Principle

- `ntd-ipc` holds **every** type exchanged with the frontend: command arguments and return
  values, event payloads, errors.
- The TypeScript types **and** typed command/event functions are **generated** from Rust with
  **`tauri-specta`** into `frontend/src/lib/infrastructure/tauri/bindings.ts`.
- CI regenerates the bindings and **fails on any diff** (`cargo xtask gen-ts && git diff --exit-code`).

### 5.2 Order

1. **First**, freeze the v2 contract as is (the 26 commands of `src-tauri/src/commands.rs`, the
   events `tablet-event`, `log-event`, `device-changed`, `profile-loaded`, `navigate-to-updates`)
   and generate the bindings. The frontend's behaviour does not change.
2. **Then** evolve the contract towards its v3 shape (§6.3), one command at a time, each change
   **compiled on both sides** thanks to the generated bindings.

### 5.3 Errors

Commands return `Result<T, IpcError>`, where `IpcError` is a serialised enum
(`{ kind: "DeviceNotFound" } | { kind: "InvalidMapping", field: "…" } | …`). The frontend
`switch`es on `kind`. An error is never `.to_string()`ed for the frontend to parse.

### 5.4 External integrations: websocket and SDK

- **Websocket**: v3 introduces a **versioned** protocol. Each message carries a protocol
  version; the client chooses it at connection time. The **v2 protocol stays available for the
  whole 3.x line** through a compatibility layer, with the same messages and fields as today, so
  existing overlays (osu! and others) keep working. It is removed in 4.0 and deprecated in the
  3.0 release notes.
- **SDK** (`apps/sdk`, C and C# bindings): v3 is a **new major SDK version**, with a migration
  guide `docs/sdk-migration-v3.md`. Within 3.x, the SDK API does not break.
- Both protocols (websocket v2 and v3) are covered by tests and by `ntd-stress` (§15.5).

---

## 6. The frontend: kept, but its data layer changes

### 6.1 Verdict: keeping the frontend is **not** a mistake

- A modern, suitable stack: **Svelte 5** (runes), **TypeScript**, **Vite**.
- Organised by features (`mapping`, `pen`, `filters`, `settings`, `console`, `release`, `credits`).
- A design system with tokens, validated themes (`theme/validate.ts`, `contrast.ts`) and tests.
- Unit tests (Vitest + Testing Library), Playwright E2E, native smoke test, Tauri mock
  (`src/dev/tauri-mock.ts`).
- ≈ 12,000 lines **that work**. Rewriting them would not fix any of the issues below: they all
  come from the **contract** and the **data layer**, not from Svelte.

> Rewriting the frontend *and* the backend at the same time means losing every fixed point for
> detecting regressions. The frontend is kept **precisely** so it can serve as the reference.

### 6.2 What is wrong in the frontend

| Issue | Where | Consequence |
|---|---|---|
| Hand-written types duplicated from Rust | `infrastructure/tauri/commands.ts` (`Config`, `Area`, `PluginPropertyKind`…) | They silently drift from Rust. A typo means `undefined` at runtime. |
| Catch-all `Config` | same file; `get_config` / `set_config(new_config: MappingConfig)` in Rust | Every small change sends **the whole** object back, with possible conflicts if two screens edit at once. |
| Magic strings | `mode: string`, `tip_binding: string`, event names like `"tablet-event"` | No compile-time check. |
| `invoke` called directly in components | `components/MenuBar.svelte:93,113,194,278` | Bypasses the `infrastructure` layer, breaking the mock and consistency. |
| "Dirty" detection with `JSON.stringify` | `features/mapping/mapping-state.svelte.ts` (the comment mentions serialisation order) | Fragile: depends on key order. |
| `setTimeout(…, 50)` for `isLoaded` | `mapping-state.svelte.ts` | A workaround for a race, not a fix. |
| `listen` called directly in a component | `features/mapping/TabletVisualizer.svelte` | Bypasses `infrastructure/events.ts`. |
| High-frequency stream via JSON `emit` | `src-tauri/src/lib.rs:242` (`tablet-event`) | CPU-heavy; the right tool is `tauri::ipc::Channel`. |
| Two icon libraries | `package.json`: `@lucide/svelte` **and** `lucide-svelte` | Dead weight, two APIs. |

### 6.3 What must change (and nothing more)

1. **Generated bindings** (§5): every hand-written DTO interface in `commands.ts` goes away.
2. **A single gateway to Tauri**: `infrastructure/tauri/` is the **only** place that imports
   `@tauri-apps/api`. An ESLint `no-restricted-imports` rule enforces it everywhere else.
3. **Split `Config` by domain**, in the contract and in the stores:
   - `Mapping` (areas, rotation, mode, snapping, aspect ratio)
   - `PenSettings` (thresholds, pressure, curve, tilt)
   - `Bindings` (typed: `enum Binding { Key{…}, Mouse{…}, Disabled, … }`)
   - `FilterChain` (enabled plugins and their properties)
   - `AppPreferences` (theme, language, autostart, tray, high-resolution timer, telemetry)
   - `WebSocketSettings`

   One command pair per domain (`get_mapping` / `update_mapping`…). One Svelte store per domain.
4. **The backend is the source of truth.** The frontend sends an intent (`update_mapping`), the
   backend validates, applies and returns the real state (or a `mapping-changed` event). The
   frontend shows **what the backend returns**. No "save" that blindly overwrites state.
5. **Dirty state and history**: structural comparison (field by field), no `JSON.stringify`.
   Undo/redo can stay in the frontend, on typed domain objects.
6. **Real-time stream**: `tablet-event` goes through a Tauri `Channel`, decimated to the display
   rate (≈ 60 Hz) **in the backend**. The frontend never receives 1000 Hz.
7. **Remove synchronisation `setTimeout`s** in favour of real states (`loading | ready | error`).
8. **Keep**: UI components, design system, themes, pages, tests, E2E, Tauri mock (kept up to date
   automatically by the generated bindings).
9. **Frontend quality rules** (§12.7) apply like the backend ones.

---

## 7. Migration plan by phase

Each phase = several small PRs. The **exit criterion** is mandatory before the next phase. The
detailed, checkable task list is the roadmap (§17).

### Phase 0: Safety nets (before touching the architecture)

- Fix the updater public key (§11), publish the **transition 2.x release**, then start the
  release freeze (§2).
- Write `ntd-recorder` (§15.3) first: it produces the captures that golden files need.
- Add **golden file tests** for the parsers: for each tablet, raw HID reports → expected parsed
  report (snapshot). This is the most important safety net.
- Add a **latency benchmark** of the pipeline (criterion already exists) and record the baseline.
- Freeze the current IPC contract (§5.2 step 1) with `tauri-specta`.
- **Exit**: green CI, golden files for every tablet with a real capture, latency baseline recorded.

### Phase 1: Workspace, quality, CI, contribution rules

- Create the `[workspace]`, merge the three `Cargo.lock` files, introduce
  `[workspace.dependencies]` and `[workspace.lints]`.
- Move `src-tauri` → `apps/desktop`, `sdk` → `apps/sdk`, `sdk/plugin_api` → `crates/ntd-plugin-api`.
- Put the quality policy (§12) and CI (§13) in place. The new `deny` lints will surface existing
  non-compliant code: fix it in dedicated PRs, **without** behaviour change, one area at a time.
- Write `AGENTS.md` and the contribution policy (§14).
- **Exit**: one lockfile, `cargo build --workspace` and `cargo test --workspace` green, no
  behaviour change.

### Phase 2: Extract the pure leaves

In order: `ntd-core` → `ntd-tablets` → `ntd-pipeline`.

- For `ntd-tablets`: replace the use of `engine::state::LockRecoveryExt` in the Wacom parsers with
  parser-local state (a parser should not need a lock) or, failing that, move the helper into
  `ntd-core`.
- For `ntd-pipeline`: introduce `trait Filter`, break the cycle with plugins.
- Point `qa/fuzz` at these crates directly and add the parser targets (§15.4).
- Replace `include_dir` + JSON parsing at startup with `ntd-codegen` (§15.1).
- **Exit**: these crates have **no** I/O dependency (checked with `cargo tree`), golden files
  identical, latency within tolerance (+5 %).

### Phase 3: Extract the I/O adapters

`ntd-hid`, `ntd-output`, `ntd-interop`, `ntd-plugin-host`, `ntd-settings`, `ntd-platform`, each
behind a trait with a fake in `ntd-test-support`.

- `ntd-settings` receives `Paths` as a parameter; no global `get_*_dir()` function remains.
- Every OS adapter gets the `windows` / `linux` / `macos` / `unsupported` structure of §16.3, with
  `macos` returning `Unsupported`.
- `ntd-plugin-host` implements ABI v3 and refuses v2 plugins with a clear message.
- **Exit**: the `desktop` feature no longer exists; `apps/sdk` no longer depends on `ntd-engine`;
  the workspace builds (`cargo check`) on every target of §16.2, including those not implemented
  yet.

### Phase 4: The engine

- `ntd-engine` with `EngineHandle` + `EngineEvent` (§4.5).
- Split `polling.rs` (715 lines) and `owner.rs`: reading, parsing, pipeline, output, buttons.
- Telemetry moved out of the engine (events).
- **Exit**: `ntd-tester` (§15.2) passes on every capture; latency within tolerance.

### Phase 5: Services and application

- `ntd-services` (websocket v3 + v2 compatibility, autoupdate, telemetry, supervisor) subscribes
  to `EngineEvent`s.
- SDK v3 and its migration guide (§5.4).
- `apps/desktop` becomes wiring only: about 300 lines in total, no business logic.
- **Exit**: `apps/desktop/src` contains no logic; each command is ≤ 10 lines.

### Phase 6: v3 contract and frontend

- Split `Config` by domain, typed bindings, typed errors, `Channel` for real time (§6.3).
- Automatic v2 → v3 profile migration (§8, `ntd-settings`).
- **Exit**: E2E green, real v2 profiles imported without loss.

### Phase 7: Release

- Update channels, release workflows, `3.0.0-beta.1` on the pre-release channel.
- **4-week** public beta, then `3.0.0`.
- No 2.x maintenance afterwards (§2).

---

## 8. Rules per crate

### `ntd-core`
- No I/O, no threads, no `unsafe`, no heavy dependencies.
- Value types (`Copy` when possible); units explicit in names or newtypes (`Millimeters`,
  `TabletUnits`, `Pixels`).

### `ntd-tablets`
- A parser = a function `fn parse(&mut self, report: &[u8]) -> Result<Option<TabletReport>, ParseError>`.
- **Never panic** on input: any malformed report returns an error (it is fuzzed).
- No `log::` in the hot path: return a typed error and let the caller decide.
- Every new tablet comes with its golden files.
- Descriptors are generated by `ntd-codegen` (§15.1).

### `ntd-hid`
- The only crate that talks to `hidapi`.
- Handles reconnection, but **does not parse**: it delivers raw bytes.

### `ntd-pipeline`
- Deterministic: same input + same clock = same output.
- **No allocation** in the hot path (pre-allocate at configuration time).
- Time is injected (`Clock`), never read with `Instant::now()` directly.

### `ntd-plugin-api`
- **Stable, versioned C ABI.** v3 introduces **ABI v3**: target triple in the manifest,
  fixed-width types only. **v2 plugins are not supported** in v3; the host refuses them with a
  clear message, and `docs/plugin-migration-v3.md` explains how to port a plugin.
- Within a major version, the ABI does not break. The next incompatible change is ABI v4, with v4.
- `#[repr(C)]` types only. No `String`, `Vec`, `Box` or Rust trait crosses the boundary.
- **Published on crates.io** as `ntd-plugin-api` so authors can `cargo add ntd-plugin-api`. Its
  major version follows the ABI version. It is the only published crate.

### `ntd-plugin-host`
- Trust verification (minisign) happens **before** `dlopen`, never after.
- A plugin that panics or returns an error is disabled; the pipeline continues without it.

### `ntd-output`
- A `OutputSink` trait; one implementation per OS; a fake that records events.

### `ntd-settings`
- Every persisted file has a `version` field. Migrations are pure `v2 → v3` functions tested with
  **real** v2 files as fixtures.
- Atomic writes (temporary file + rename).
- A corrupted file → back up the corrupted file, load defaults, tell the user. A profile is
  never silently lost.

### `ntd-engine`
- The only owner of the engine threads.
- API = `EngineHandle` (commands) + `Receiver<EngineEvent>`. No exposed `Arc<Mutex<…>>`.

### `ntd-services`
- Every network feature (telemetry, update, websocket) can be **turned off**.
- **Telemetry is on by default (opt-out)**, as in v2. It is anonymous, contains no personal data
  (user paths, profile names, machine names), and the toggle is visible in Settings. v2 profiles
  keep the user's current choice on migration.
- The websocket binds to `127.0.0.1` by default and serves protocol v3 and v2 (§5.4).

### `ntd-ipc`
- Serialisable types and TS generation (`tauri-specta`) only. No logic.

### `apps/desktop`
- Wiring: build `Paths`, start `Engine` and services, register commands, tray, updater. Each
  command = translate DTO → call `EngineHandle` → DTO.

---

## 9. What NOT to do

> This section is **normative**, for humans and LLMs alike.
> If a task seems to require breaking one of these rules: **stop and ask**.

### 9.1 Project, repository, strategy

1. **DO NOT** create a new repository for v3. v2 users' updater points here (§2).
2. **DO NOT** change the updater URL, repository name or GitHub owner without first publishing a
   release that points to the new location.
3. **DO NOT** do a big-bang rewrite on a branch that diverges for months.
4. **DO NOT** mix moving code and changing behaviour in the same PR. A PR that moves code changes
   **nothing** else (diff readable with `git diff -M`).
5. **DO NOT** add features during the rewrite. Features wait until the relevant phase is done.
6. **DO NOT** rewrite the frontend.
7. **DO NOT** delete Git history, squash existing history, or restart from `git init`.
8. **DO NOT** break `main`: every PR leaves the application working.

### 9.2 Architecture and split

9. **DO NOT** add a dependency from a low crate to a high crate (see §4.3). If you need one, a
   trait or an event is missing: invert the dependency.
10. **DO NOT** create a cycle between crates, even through an optional feature.
11. **DO NOT** create a catch-all crate such as `ntd-common`, `ntd-utils`, `ntd-shared` or
    `ntd-misc`. Every function has an owner; if none fits, it is `ntd-core` and it must be pure.
12. **DO NOT** split finer than needed: **no** crate per tablet vendor, **no** crate per filter,
    **no** crate per OS. A module is enough.
13. **DO NOT** reintroduce a Cargo feature to "remove" parts of a crate (like `desktop`). If a
    consumer does not want part of a crate, that part is another crate.
14. **DO NOT** use Cargo features to pick the OS. Use `cfg(target_os = …)`.
15. **DO NOT** add global state (`static`, `lazy_static`, business `OnceLock`,
    `get_settings_dir()`) in a library crate. Dependencies are passed as parameters.
16. **DO NOT** expose `Arc<Mutex<State>>` outside `ntd-engine`. Communicate through commands and
    events.
17. **DO NOT** call telemetry, the network or the file system from `ntd-core`, `ntd-tablets` or
    `ntd-pipeline`.
18. **DO NOT** put business logic in `apps/desktop` or in Tauri commands.
19. **DO NOT** make `apps/sdk` depend on `ntd-engine`, `ntd-services` or Tauri.
20. **DO NOT** duplicate a type in two crates "to avoid a dependency". It belongs to the lowest
    crate that needs it.
21. **DO NOT** keep several `Cargo.lock` files. One workspace, one lockfile.
22. **DO NOT** declare a dependency version in a member crate: `foo.workspace = true`.

### 9.3 Rust: code and errors

23. **DO NOT** use `unwrap()`, `expect()` or `panic!` on data coming from outside (HID, file,
    network, plugin, frontend). Only proven internal invariants may `expect`, with a message
    explaining the invariant.
24. **DO NOT** use `anyhow` (or `Box<dyn Error>`) in library crates. Use `thiserror` enums.
    `anyhow` is tolerated only in `xtask`.
25. **DO NOT** convert an error to `String` to carry it. Keep the type.
26. **DO NOT** swallow errors with `let _ = …` or `.ok()` without a comment justifying why the
    failure does not matter (v2 does `let _ = app.emit(…)` everywhere).
27. **DO NOT** use `unsafe` outside `ntd-hid`, `ntd-output`, `ntd-interop`, `ntd-plugin-host`,
    `ntd-plugin-api` and `ntd-platform`. Every `unsafe` block has a `// SAFETY:` comment.
28. **DO NOT** disable a clippy lint globally to make CI pass. Use a local `#[allow]` with a
    reason, or fix the code.
29. **DO NOT** write files longer than about 400 lines without justifying it in the PR. This is a
    review guideline, not a CI check.
30. **DO NOT** use `String` for a finite set of values (mode, binding, curve type, log level). Use
    a serialised `enum`.
31. **DO NOT** leave numbers without units: tell tablet units, millimetres, pixels and seconds
    apart with newtypes or explicit names.
32. **DO NOT** make things `pub` by default. `pub(crate)` first; a crate's public API is
    deliberate and documented.

### 9.4 Latency, concurrency and the hot path

33. **DO NOT** allocate (`Vec::new`, `String`, `format!`, `Box`, `clone()` of collections) in the
    read → parse → pipeline → output loop.
34. **DO NOT** log in the hot path (not even `debug!`: formatting costs). Counters or sampling only.
35. **DO NOT** take a blocking lock shared with the UI in the hot path. Configuration arrives
    through a channel or an atomic swap (`arc-swap`), read without blocking.
36. **DO NOT** use `async`/Tokio in the engine. The engine is dedicated threads and channels;
    async is reserved for `ntd-services` if needed.
37. **DO NOT** call `Instant::now()` or `sleep` directly in the pipeline. Inject a `Clock`.
38. **DO NOT** send every HID report to the frontend. Decimate in the backend (≈ 60 Hz).
39. **DO NOT** merge a PR that makes the reference latency benchmark more than **5 %** slower
    than the baseline without an explicit maintainer decision recorded in the PR.

### 9.5 Tablets and parsers

40. **DO NOT** rewrite a parser "more cleanly" without golden files proving byte-for-byte parity.
41. **DO NOT** remove a bit mask, shift or special case "that looks useless" from a parser. Check
    `git blame` first: it is almost always a hardware fix.
42. **DO NOT** add a tablet without a descriptor in `tablets/` **and** golden files.
43. **DO NOT** let a parser panic on a short or unknown report: return an error or `None`.
44. **DO NOT** hardcode a VID/PID in Rust code: it belongs in the descriptors.
45. **DO NOT** copy descriptors or code from other drivers (e.g. OpenTabletDriver) without
    checking their licence and required attribution (`tablets/OpenTabletDriver` is a git
    submodule of their repository: check its licence against the project's MIT licence).

### 9.6 Plugins, ABI and security

46. **DO NOT** load (`dlopen`/`LoadLibrary`) a library before verifying its signature.
47. **DO NOT** change `ntd-plugin-api` incompatibly within a major version. v3 introduces ABI v3
    and drops v2 plugins; the next break is ABI v4, with v4.
48. **DO NOT** let Rust types cross the ABI boundary (`String`, `Vec`, `Box`, non-`repr(C)`
    `Option<&T>`, traits, panics). Panics must be caught (`catch_unwind`) on both the plugin and
    the host side.
49. **DO NOT** weaken, bypass or make trust verification optional "for dev" in production code.
    Dev mode is explicit and visible in the UI.
50. **DO NOT** commit a private key, a token, or a development key in place of a real public key
    (see §11).
51. **DO NOT** expose the websocket beyond `127.0.0.1` by default.
52. **DO NOT** put personal data in telemetry (user paths, profile file names, machine names) and
    **DO NOT** make it impossible to disable. It is on by default (opt-out) and the toggle lives in
    Settings.

### 9.7 Settings and persistence

53. **DO NOT** change the format of a persisted file without a `version` field and a tested
    migration.
54. **DO NOT** break users' v2 profiles: v3 imports them automatically and keeps a backup of the
    original.
55. **DO NOT** write a configuration file in place. Temporary file, then atomic rename.
56. **DO NOT** silently replace an unreadable file with defaults without keeping a copy and
    telling the user.
57. **DO NOT** bring back a single `Config` object that holds everything. One type per domain
    (§6.3).

### 9.8 IPC contract and frontend

58. **DO NOT** hand-write in TypeScript a type that exists in Rust. It is generated.
59. **DO NOT** edit `bindings.ts` by hand. It is regenerated and checked in CI.
60. **DO NOT** import `@tauri-apps/api` anywhere but `frontend/src/lib/infrastructure/tauri/`.
61. **DO NOT** use string literals for command or event names in components.
62. **DO NOT** send the whole state on every change (`set_config(everything)`). One command per
    domain, validated by the backend.
63. **DO NOT** rely on the frontend for validation. The backend validates everything and returns
    the real state.
64. **DO NOT** return errors as text the frontend has to parse. Typed errors.
65. **DO NOT** use `setTimeout` to "wait until it's ready". Use an explicit state.
66. **DO NOT** compare objects with `JSON.stringify`.
67. **DO NOT** use JSON event `emit` for high-frequency streams. Use `Channel`.
68. **DO NOT** change frontend framework, design system or folder structure during the rewrite.
69. **DO NOT** keep two libraries for the same job (`@lucide/svelte` and `lucide-svelte`).

### 9.9 Tests and CI

70. **DO NOT** delete, ignore (`#[ignore]`, `.skip`) or weaken a test to get a PR through.
71. **DO NOT** lower the thresholds in `coverage-thresholds.json`.
72. **DO NOT** write tests that need a real tablet or real OS injection to test logic. Use the
    fakes in `ntd-test-support`.
73. **DO NOT** update snapshots / golden files in bulk without reviewing each difference and
    explaining why in the PR.
74. **DO NOT** test implementation details ("private function X is called"). Test observable
    behaviour.
75. **DO NOT** merge with red CI "because it's flaky". A flaky test is a bug to fix.

### 9.10 Release and update

76. **DO NOT** publish a release whose signature was not verified with the public key actually
    embedded in the application.
77. **DO NOT** publish v3 as `latest` before the 4-week beta on the pre-release channel.
78. **DO NOT** remove from the transition 2.x release (P0.2) the ability to auto-update to v3.

### 9.11 Specific to LLMs and coding assistants

79. **DO NOT** invent APIs, crates, Tauri commands or configuration fields. Check the code
    (`grep`) before using or citing them.
80. **DO NOT** "simplify" parser, HID mask or timing code without understanding it.
81. **DO NOT** generate replacement code for a whole module when the task asks for a move.
    Moving = `git mv` + fix paths, nothing else.
82. **DO NOT** add an external dependency without declaring it in `[workspace.dependencies]`,
    checking that it passes `cargo deny`, and justifying why std or an existing dependency is not
    enough.
83. **DO NOT** sidestep a rule of this document by claiming it "doesn't really apply here". Ask.
84. **DO NOT** declare a task done without running `cargo fmt --check`, `cargo clippy --workspace
    --all-targets --locked`, `cargo test --workspace`, `cargo xtask lint-style`, `cargo machete`,
    `typos`, and on the frontend `npm run format:check`, `npm run check`, `npm run lint`,
    `npm test`.
85. **DO NOT** produce large "refactor everything" commits. One PR = one intent.
86. **DO NOT** leave a `TODO` without a linked issue, or commented-out code "just in case".
87. **DO NOT** include an AI model identifier, AI session link or AI co-author trailer in
    commits, code or documentation.
88. **DO NOT** open a pull request as an agent, even if asked to. Prepare the branch and the
    description; a human opens the PR (§14.3).

### 9.12 Code quality and comments

89. **DO NOT** exceed 100 characters per line in a `.rs` file, comments and strings included
    (§12.1).
90. **DO NOT** write a comment that paraphrases the code (`// increment i`, `// return the
    result`). A comment explains a **why**: a choice, a hardware constraint, an invariant, a pitfall
    (§12.2).
91. **DO NOT** decorate comments: no banners (`// =====`, `// -----`, `/*****/`), no emojis, no
    ASCII art, no all-caps section titles, no signature or date.
92. **DO NOT** leave commented-out code. Git history keeps it.
93. **DO NOT** write `#[allow(...)]` without `reason = "..."`. The reason is the comment.
94. **DO NOT** use `println!`, `eprintln!`, `print!` or `dbg!`. Everything goes through `log`.
    Tools (`tools/*`, `xtask`) that must print declare it with a local
    `#[expect(clippy::print_stdout, reason = "...")]`.
95. **DO NOT** use `unreachable!()`: return an error, or restructure so the case cannot exist in
    the types.
96. **DO NOT** write an `unsafe` block without a `// SAFETY:` comment proving the invariant.
97. **DO NOT** declare a `pub` item without `///` docs, a public function returning `Result`
    without an `# Errors` section, or a function that can panic without `# Panics`.
98. **DO NOT** declare an unused dependency (cargo-machete fails).
99. **DO NOT** downgrade a lint from `deny` to `warn` or `allow` at workspace level without a
    maintainer decision recorded in the PR and in the `reason`.

### 9.13 Platforms and architectures

100. **DO NOT** pin a specific Rust version in `rust-toolchain.toml` or in a workflow. The
     toolchain is `stable` (§12.6). When a new version breaks CI, fix the code; do not go back to
     a pinned version.
101. **DO NOT** edit a file in `crates/ntd-tablets/src/generated/` by hand. Edit the JSON in
     `tablets/` or the generator, then regenerate.
102. **DO NOT** commit regenerated files unrelated to the PR. A codegen run can touch other files:
     keep the ones you need and revert the rest.
103. **DO NOT** invent or retouch an `ntd-recorder` capture. A capture comes from a real tablet; if
     it is wrong, record a new one.
104. **DO NOT** put personal data in a capture (machine name, paths, full serial number):
     `ntd-recorder` strips them, and you check before committing.
105. **DO NOT** delete a fuzz corpus entry that crashed the code: it becomes a regression test.
106. **DO NOT** assume 64 bits. No `usize`/`isize` in persisted formats, the plugin ABI, the
     interop protocol or the IPC contract: use explicit `u32`/`u64`. No `as usize` on a `u64`:
     use `usize::try_from`.
107. **DO NOT** write `cfg(not(windows))` to mean "Linux". Always an explicit
     `cfg(target_os = "...")`, and an unhandled OS falls into a branch that returns
     `Unsupported`, never into a silent or empty branch.
108. **DO NOT** put OS-specific code outside `ntd-hid`, `ntd-output`, `ntd-interop`,
     `ntd-platform` and `apps/desktop`.
109. **DO NOT** read HID or network bytes with `from_ne_bytes`, `transmute` or a pointer cast.
     Always an explicit `from_le_bytes` / `from_be_bytes`.
110. **DO NOT** use SIMD intrinsics or architecture-specific instructions without an equivalent
     portable version, tested on every target.
111. **DO NOT** load a plugin built for another target: the manifest declares its target triple
     and the host refuses a plugin that does not match.
112. **DO NOT** advertise (README, website, release notes) a tier 3 platform as supported.
113. **DO NOT** let a tier 3 platform stop compiling: `check-targets` is a blocking check.
114. **DO NOT** assume Windows in the frontend (title bar, `Ctrl` shortcuts, window button
     placement): go through `infrastructure/os.ts`, which exposes the platform.
115. **DO NOT** promise 32-bit macOS: it no longer exists (macOS has not run 32-bit applications
     since 10.15).

### 9.14 Language, releases and integrations

116. **DO NOT** write code, comments, commit messages, PR titles or descriptions, `AGENTS.md` or
     `docs/` in any language other than English.
117. **DO NOT** publish any 2.x release after the transition release (P0.2), not even a security
     fix. The freeze lasts until `3.0.0-beta.1`.
118. **DO NOT** remove or change the websocket v2 protocol during 3.x. It is removed in 4.0 only.
119. **DO NOT** break the SDK API or `ntd-plugin-api` within 3.x.

### 9.15 Frontend quality

120. **DO NOT** use `any`, or `@ts-ignore` / `@ts-expect-error` without a description explaining
     why.
121. **DO NOT** use `console.*` outside `frontend/src/lib/infrastructure/log.ts`.
122. **DO NOT** leave an ESLint or `svelte-check` warning: warnings fail CI like errors.
123. **DO NOT** exceed 100 characters per line in `.ts` and `.svelte` files; Prettier is the
     reference.

---

## 10. PR checklist

- [ ] The PR has **a single intent** (move **or** change behaviour, not both).
- [ ] No crate dependency points up (§4.3); `cargo tree -p <crate>` is compliant.
- [ ] No new global state, no `unwrap` on external data, no `unsafe` outside allowed crates.
- [ ] Parser golden files identical (or differences justified one by one).
- [ ] Latency benchmark within +5 % of the baseline if the hot path is touched.
- [ ] `bindings.ts` regenerated with no diff if the contract is touched.
- [ ] Migration and fixtures if a persisted format is touched.
- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`,
      `cargo deny check` green.
- [ ] `npm run format:check`, `npm run check`, `npm run lint`, `npm test`, `npm run e2e` green.
- [ ] Documentation (`docs/`) updated if visible behaviour changes.
- [ ] No line over 100 characters, no decorative or paraphrasing comment (§12).
- [ ] `cargo machete` and `typos` green (§13).
- [ ] Everything is written in English (rule 116).
- [ ] The description says **what**, **why**, **how it was tested**, and whether AI wrote a
      significant part of the code (§14).
- [ ] If the PR finishes a roadmap task, the box is ticked and §17.1 updated (§17.2).

---

## 11. Issues found outside the architecture

These points are handled **in phase 0**, independently of the rewrite:

1. **Placeholder updater public key.** In `src-tauri/tauri.conf.json`, `plugins.updater.pubkey` is
   `dW51c2VkX3B1YmtleV9mb3JfZGV2ZWxvcG1lbnRfcHVycG9zZXM=`, which decodes to
   `unused_pubkey_for_development_purposes`. Releases are signed in CI
   (`.github/workflows/release.yml`, `TAURI_SIGNING_PRIVATE_KEY`), but the application cannot
   verify that signature with this key. Depending on the updater's behaviour, auto-update either
   fails or is unprotected. **Fix this before anything else**, since the whole v2 → v3 migration
   relies on auto-update; it ships in the transition release (P0.2).
2. **Licence of `tablets/OpenTabletDriver`**: it is a git submodule of the OpenTabletDriver
   repository. Identify its licence, check compatibility with the project's MIT licence, and add
   the required attribution.
3. **Two icon libraries** in `frontend/package.json` (`@lucide/svelte` and `lucide-svelte`).

---

## 12. Code quality

These rules apply to all Rust code in the workspace (§12.1 to §12.6) and to the frontend
(§12.7). They are **checked by CI**: a rule no tool checks always ends up being ignored.

### 12.1 Line width: 100 characters, strict

- `rustfmt.toml`: `max_width = 100` (already the case in v2).
- **rustfmt limitation**: on the stable toolchain, rustfmt wraps neither comments nor long
  strings (`wrap_comments`, `comment_width` and `error_on_line_overflow` are unstable). Clippy has
  no line length lint. A dedicated check (`cargo xtask lint-style`, §12.5) therefore fails CI as
  soon as a `.rs` line exceeds 100 characters, comments and strings included.
- A long string is split with `concat!` or a trailing `\` continuation. It is not worked around
  with `#[rustfmt::skip]`.
- Editor: `.vscode/settings.json` keeps `"editor.rulers": [80, 100]` (already the case). 80 is a
  visual guide, 100 is the limit. `.editorconfig` adds `max_line_length = 100` for `[*.rs]`,
  `[*.ts]` and `[*.svelte]`.

Target `rustfmt.toml` (stable options only):

```toml
edition = "2024"
max_width = 100
newline_style = "Unix"
use_field_init_shorthand = true
use_try_shorthand = true
```

### 12.2 Comments

**Principle**: the code says **what**, the comment says **why**. If a comment repeats what the
code says, delete it. If the code is not clear without a comment, rename or split first, and
comment only what remains.

A comment is justified when it explains:
- a **choice** between several options ("bounded channel rather than unbounded: we would rather
  drop a report than build up latency");
- a **hardware or external constraint** ("the Intuos Pro sends pressure on 13 bits; the top 3 bits
  are noise");
- an **invariant** the types do not express;
- a **pitfall** ("do not move this call before `flush`: Windows drops the event");
- the proof of an `unsafe` block (`// SAFETY:`, mandatory).

Forbidden:
- paraphrase (`// loop over the reports`, `// create the parser`);
- decoration: `// ======`, `// ------`, `/*******/` banners, all-caps titles, emojis, ASCII art,
  signatures, dates;
- commented-out code;
- `TODO`s without an issue: the only accepted form is `// TODO(#123): ...`.

```rust
// Bad
// ===== PARSING =====
// Get the pressure
let pressure = u16::from_le_bytes([r[6], r[7]]) & 0x1FFF; // mask 🎯

// Good
// On this model the top 3 bits carry the side button state, not
// pressure: see the intuos_pro_* golden files.
let pressure = u16::from_le_bytes([r[6], r[7]]) & 0x1FFF;
```

**Public documentation is mandatory** (`missing_docs = "deny"`):
- every `pub` item has a short, factual `///` comment: what it is, not how it is implemented;
- every public function returning `Result` has an `# Errors` section (`missing_errors_doc = "deny"`);
- every public function that can panic has a `# Panics` section (`missing_panics_doc = "deny"`),
  which should be rare given §9.3;
- every crate has a `//!` header in `lib.rs` stating its responsibility and what it does not do.

`unreachable_pub = "deny"` completes this rule: an item that is not really exported is written
`pub(crate)` and therefore needs no public docs.

### 12.3 Strict Cargo.toml

Lints are defined **once** in `[workspace.lints]` of the root `Cargo.toml`; every crate has
`[lints] workspace = true` and nothing else.

```toml
[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "deny"
unreachable_pub = "deny"
dead_code = "deny"

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"

[workspace.lints.clippy]
# Groups: everything blocks.
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
cargo = { level = "deny", priority = -1 }

# Error handling: no external data may make the driver panic.
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
exit = "deny"
lossy_float_literal = "deny"

# Output: everything goes through `log`.
print_stdout = "deny"
print_stderr = "deny"
dbg_macro = "deny"

# unsafe and FFI.
undocumented_unsafe_blocks = "deny"
multiple_unsafe_ops_per_block = "deny"
uninit_assumed_init = "deny"
uninit_vec = "deny"
cast_ptr_alignment = "deny"
ptr_as_ptr = "deny"
fn_to_numeric_cast_any = "deny"

# Documentation.
missing_errors_doc = "deny"
missing_panics_doc = "deny"

# Every #[allow] must carry its justification: #[allow(lint, reason = "...")].
allow_attributes_without_reason = "deny"

# Exceptions, carried over from v2.
# HID reports mix u8/u16/i16/f32: the conversions are intended and bounded.
cast_precision_loss = "allow"
cast_possible_truncation = "allow"
cast_sign_loss = "allow"
# x/y, tilt_x/tilt_y: close names are the domain vocabulary.
similar_names = "allow"
# Parsers are long flat `match`es over bytes; splitting them hurts readability.
too_many_lines = "allow"
assert_is_empty = "allow"
# Duplicate versions are handled by cargo-deny (bans) with a reviewed exception list.
multiple_crate_versions = "allow"
```

Notes:
- **`unsafe_code`**: `deny` (not `forbid`) so that the allowed FFI crates (`ntd-hid`,
  `ntd-output`, `ntd-interop`, `ntd-plugin-host`, `ntd-plugin-api`, `ntd-platform`) can put
  `#![allow(unsafe_code, reason = "...")]` at the root of the relevant module.
  `cargo xtask lint-style` fails if that `allow` appears in another crate.
- **`clippy::cargo`** requires complete metadata: every crate inherits from `[workspace.package]`
  (`license.workspace = true`, `repository.workspace = true`…) and declares its own
  `description`. Wildcard versions (`"*"`) are forbidden.
- **`nursery` as `deny`**: these lints change between Rust versions. With the `stable` toolchain
  (§12.6), a new Rust release can therefore fail CI without any code change. That is intended:
  the new lints are fixed in a dedicated PR, and a lint is never downgraded to get CI green again
  (rule 99).
- **Tools** (`tools/*`, `xtask`) follow the same lints. Where a CLI tool must print or stop on an
  error, it says so locally with `#[expect(..., reason = "...")]` on the function concerned, never
  with a global `allow`.
- **Tests**: `clippy.toml` keeps `allow-unwrap-in-tests`, `allow-expect-in-tests`,
  `allow-panic-in-tests` and `allow-indexing-slicing-in-tests`. A test that panics is a failing
  test, which is the intended behaviour.
- Every cargo command in CI uses `--locked`.

### 12.4 Error handling

- One `thiserror` error enum per crate (`TabletError`, `PipelineError`, `EngineError`…). Errors
  from a lower crate are wrapped in the upper one with `#[from]`, without losing the source.
- `?` everywhere. An `Option` → `Result` conversion is `.ok_or(Error::Variant)`, never `.unwrap()`.
- The only `expect`s tolerated outside tests are internal invariants, with
  `#[allow(clippy::expect_used, reason = "invariant: ...")]`. The reason is reviewed.

### 12.5 `cargo xtask lint-style`

A task in `xtask/` checks what rustfmt and clippy cannot. It runs in CI and locally:
- `.rs` lines over 100 characters (excluding generated files);
- decorative comments: banners (`//` followed by 4 or more `=`, `-`, `*` or `#`), emojis;
- `TODO`/`FIXME` without `(#number)`;
- `#![allow(unsafe_code` outside the allowed crates;
- `#[rustfmt::skip]` without a justifying comment right above it.

### 12.6 Toolchain: `stable`

v3 follows the **latest stable Rust release** instead of a pinned version as in v2 (`1.99.0`).

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-analyzer", "rust-src"]
```

Consequences and rules:
- **No specific version** in `rust-toolchain.toml` or in CI workflows. Jobs install the toolchain
  from that file without redefining it.
- **`rust-version`** (the MSRV, minimum supported version) is still declared once, in
  `[workspace.package]`. It documents the oldest version that builds the project. It is not the
  version CI uses. It is raised when a new language feature is adopted.
- **A new stable release can break CI** (new `pedantic`/`nursery` lints, `rustc` warnings). Fix
  it in a dedicated PR titled `chore: fix lints for Rust 1.xx` that contains nothing else.
- **Anticipate**: a weekly scheduled job runs `clippy` and the tests on the `beta` toolchain. It
  is not blocking, but opens an issue on failure, which leaves about six weeks to fix things
  before the release reaches stable.
- **Release traceability**: builds are no longer reproducible down to the `rustc` version. The
  release job therefore writes the output of `rustc -Vv` into the release notes and artifacts.
- `Cargo.lock` stays committed and every command uses `--locked`: only the toolchain and lints
  follow `stable`; dependencies do not move without a PR.

### 12.7 Frontend

The frontend follows the same spirit as the backend: same comment rules (§12.2), same 100-column
limit, and everything that can be checked is checked in CI.

- **Formatting**: Prettier with `prettier-plugin-svelte`, `printWidth: 100`. CI runs
  `npm run format:check` (`prettier --check .`).
- **TypeScript**: `strict: true` plus `noUncheckedIndexedAccess` (the frontend equivalent of
  `indexing_slicing`) and `noImplicitOverride`.
- **ESLint**, warnings fail CI (`eslint . --max-warnings 0`):
  - `@typescript-eslint/no-explicit-any`: error;
  - `@typescript-eslint/ban-ts-comment`: error, `@ts-expect-error` allowed only with a
    description;
  - `no-console`: error, except in `infrastructure/log.ts`, the frontend's only logger;
  - `no-restricted-imports`: `@tauri-apps/*` only from `infrastructure/tauri/` and
    `infrastructure/os.ts` (§6.3);
  - existing Svelte rules, as errors.
- **`svelte-check`**: `--fail-on-warnings`.
- **Comments**: same rules as §12.2 (why, not what; no decoration). Public functions and types
  exported from `infrastructure/` and stores have a short JSDoc comment.

---

## 13. CI/CD

### 13.1 Principles

- **A single required check** on `main`: `ci-success`, which depends on every job. Jobs can be
  added without touching branch protection rules.
- **CI per touched crate**: a `plan` job computes the crates changed by the PR and their
  dependents (`cargo metadata` + `git diff` against the base). Lints, tests and builds only run on
  those. A **full run** happens on `main`, on tags, and whenever a global file changes (root
  `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `deny.toml`,
  `.github/**`).
- `RUSTFLAGS="-D warnings"`, `--locked` everywhere, `concurrency` with `cancel-in-progress` on PRs.
- Actions **pinned by SHA**, `permissions: contents: read` by default. Write permissions only for
  release jobs.
- Signing secrets and the crates.io token are only available in a protected **GitHub `release`
  environment**, never in PR jobs.

### 13.2 `ci.yml`: tests and quality (PRs, push to `main`)

| Job | Content |
|---|---|
| `plan` | `cargo xtask ci-plan`: touched crates, or full run (§15.6) |
| `fmt` | `cargo fmt --all --check` |
| `style` | `cargo xtask lint-style` (§12.5) |
| `typos` | `crate-ci/typos` on code, comments and `docs/` (`typos.toml`, excluding `tablets/` and generated files) |
| `machete` | `cargo machete`: no unused dependency |
| `clippy` | `cargo clippy --locked --all-targets --all-features` on touched crates |
| `clippy-release` | same with `--release` (code behind `cfg(not(debug_assertions))`) |
| `doc` | `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings` |
| `test` | `cargo nextest run` + doctests, Windows x64 and Linux x64 matrix |
| `golden` | parser golden files (§7, phase 0) |
| `bindings` | `cargo xtask gen-ts && git diff --exit-code` |
| `codegen` | `cargo run --locked -p ntd-codegen && git diff --exit-code` (§15.1) |
| `tester` | `cargo run --locked -p ntd-tester` on every capture (§15.2) |
| `fuzz` | each fuzz target for 60 s on PRs touching the relevant crates; longer in the nightly workflow |
| `coverage` | `cargo llvm-cov` + `coverage-thresholds.json` thresholds (full run) |
| `bench` | latency benchmark against the baseline, fails above +5 %, on `main` and on PRs labelled `perf` |
| `frontend` | `npm run format:check`, `check`, `lint`, `test`, `build` (§12.7) |
| `e2e` | Playwright |
| `build` | application builds on tier 1 and tier 2 targets (§16.2) |
| `check-targets` | `cargo clippy` on tier 3 targets (§16.2): they must build even if not implemented yet |
| `ci-success` | aggregates every job above |

Outside `ci.yml`, a weekly scheduled `toolchain-beta.yml` workflow runs clippy and the tests on
the `beta` toolchain. It is not blocking and opens an issue on failure (§12.6).

### 13.3 `security.yml`: security (PRs, `main`, and daily)

- `cargo deny check`: advisories, licences, bans (duplicates), sources. A single run thanks to the
  single lockfile, instead of one audit per lockfile as in v2.
- `npm audit --omit=dev` on `frontend/`.
- CodeQL (already exists).
- `actions/dependency-review-action` on PRs.
- Secret scanning and push protection enabled in the repository settings.

### 13.4 `prerelease.yml`: pre-release (tag `v3.0.0-beta.N` / `-rc.N`)

1. Check that the tag points to a `main` commit with green CI, and that the version in the
   `Cargo.toml` files and `tauri.conf.json` matches the tag.
2. Build the tier 1 and tier 2 targets.
3. Sign, then **verify the signature with the public key actually embedded** in
   `tauri.conf.json`. The job fails if the key is missing or a placeholder (§11).
4. Publish a **GitHub pre-release** with SHA-256 checksums.
5. Publish the **beta channel** manifest (`beta.json`) to **GitHub Pages**. The current endpoint
   `releases/latest/download/latest.json` ignores pre-releases, so the beta channel needs its own
   stable URL. Users subscribe to it with a "receive betas" option in Settings.

### 13.5 `nightly.yml`: nightly build

- Every night, **only if there were commits** on `main` since the last nightly.
- Builds the full matrix, signs, and publishes under a `nightly` tag that is **overwritten**
  (GitHub pre-release, marked as unsupported).
- Long fuzzing (e.g. 30 min per target) and `ntd-stress` on the produced build.
- Separate, opt-in `nightly` update channel, with its manifest (`nightly.json`) on **GitHub Pages**.
- Runs only on the main repository
  (`if: github.repository == 'Next-Tablet-Driver/NextTabletDriver'`), never on forks.

### 13.6 `release.yml`: release (tag `vX.Y.Z`)

Same steps as the pre-release, plus:
- publication as `latest` and update of `latest.json` (stable channel);
- release notes generated from PR titles (already normalised by `pr-title.yml`), including
  `rustc -Vv`;
- winget publication (`winget-release.yml` exists).

`ntd-plugin-api` is published to **crates.io** by a separate job, triggered by a
`plugin-api-vX.Y.Z` tag, in the `release` environment. The job checks that the crate's major
version matches the ABI version declared in the code.

---

## 14. Contributions, AI and AGENTS.md

### 14.1 `AGENTS.md`

- **Root `AGENTS.md`**, short: what the project is, the workspace layout (§4.1), the commands to
  run before handing over a task (§9, rule 84), and a pointer to this document stating that §9 is
  normative and §17 is the roadmap. Add `CLAUDE.md` as a symlink to `AGENTS.md` so every tool
  reads the same file.
- **One `AGENTS.md` per sensitive area**, with area-specific details:
  - `crates/ntd-tablets/AGENTS.md`: adding a tablet, golden files, never removing a mask without
    `git blame`;
  - `crates/ntd-plugin-api/AGENTS.md`: ABI stability, `repr(C)`, versioning, crates.io;
  - `crates/ntd-engine/AGENTS.md`: hot path rules (§9.4);
  - `crates/ntd-ipc/AGENTS.md`: regenerating the bindings, never editing `bindings.ts`;
  - `tools/ntd-codegen/AGENTS.md`: adding a descriptor field.
- If a maintainer says something different from `AGENTS.md`, follow the maintainer, **except**
  for rule 14.3.

### 14.2 AI usage policy

- The author of a PR is responsible for **every line**, whatever tool wrote it. Before opening the
  PR, they have read the whole diff, run the code, and can answer review questions without asking
  a model.
- The description says whether AI wrote a significant part of the code or the description.
  Removing that statement from an agent-drafted description is grounds for closing.
- Maintainers may close without review a PR they consider generated and unchecked.
- A closed PR is not reopened or recreated identically: discuss it first.
- Commits carry no AI co-author trailer, session link or model identifier (rule 87).

### 14.3 Agents do not open PRs

An AI agent prepares the branch, the commits and the description text. It **never** opens the
pull request itself, even if asked to: a human opens it after reviewing. An agent never pushes to
`main`.

### 14.4 Review and merge

- `main` is protected: no direct push, `ci-success` is the only required check.
- **The maintainer may merge their own PRs** once `ci-success` is green and the PR checklist (§10)
  is complete. No second reviewer is required while there is a single maintainer.
- **External PRs** require the maintainer's approval.

### 14.5 Closing without review

A PR may be closed without a full review if:
- the description is empty, only repeats the title, or does not say how the change was tested;
- it mixes unrelated changes (reformatting untouched code, lockfile changed without reason,
  generated files without the change that justifies them);
- it changes a parser without golden files, or changes visible behaviour without a screenshot;
- it duplicates an open PR without saying how it differs;
- the author cannot explain their change;
- it hides or removes the AI usage statement (§14.2);
- it is not written in English (rule 116);
- CI fails and the author does not fix it, or stops answering the review.

### 14.6 PR description template

`.github/pull_request_template.md`:

```markdown
## What

## Why

## Impact (users, latency, persisted formats, ABI, IPC contract, websocket/SDK)

## How it was tested

## Known limitations

## AI usage
<!-- None / partial (say what) / most -->

## Roadmap
<!-- Task ID from docs/v3-rewrite.md §17, if any -->
```

---

## 15. Internal tools

These tools are inspired by PumpkinMC's (`tools/pumpkin-codegen`, `tools/pumpkin-tester`,
`tools/pumpkin-fuzzer`, the Extractor and `.github/scripts/ci_packages.py`), adapted to a tablet
driver. They are binaries in `tools/`, members of the workspace, and follow the workspace lints
(§12.3).

### 15.1 `ntd-codegen`: tablets become code

**v2 problem**: `src/drivers/config_loader.rs` embeds `tablets/` with `include_dir` and parses the
JSON **at startup**. An invalid JSON file is only discovered at runtime, on the user's machine.

**What the tool does**:
- reads `tablets/**/*.json` and validates it (required fields, unique VID/PID, referenced parser
  exists, consistent dimensions, optional per-OS fields of §16.3);
- generates static Rust tables in `crates/ntd-tablets/src/generated/`, with a "generated by
  ntd-codegen, do not edit" header;
- the generated files are **committed**, as at Pumpkin: diffs are reviewable and the build does
  not depend on a `build.rs`.

**Commands**: `cargo run --locked -p ntd-codegen` regenerates everything. CI regenerates and fails
on any diff.

**Consequences**: no more `serde_json` or `include_dir` on the startup path, no more descriptor
errors at runtime, and VID/PID matching becomes a static table lookup.

### 15.2 `ntd-tester`: the real engine, without a tablet

**Equivalent** of `pumpkin-tester` (real server + bots + scenarios).

**What the tool does**:
- starts the **real** `ntd-engine` with a `HidBackend` that replays a capture (§15.3), an
  `OutputSink` that records what would be injected, a simulated clock and `Paths` in a temporary
  directory;
- for each scenario, compares the output (positions, pressure, clicks, keys) with an expected
  file, with an explicit tolerance for floats;
- covers what unit tests do not see: profiles (mapping, rotation, relative mode), filter chain and
  official plugins, button bindings, unplugging and replugging mid-capture.

**Scenario format**: `qa/scenarios/<name>/` holds the capture, the profile and the expected
output. Adding a scenario requires no code.

**Command**: `cargo run --locked -p ntd-tester` (or `-- <name>` for a single scenario). It exits
non-zero on the first mismatch and prints a readable diff.

### 15.3 `ntd-recorder`: capturing a tablet

**Equivalent** of Pumpkin's Extractor, which produces reference data from the game. Here the
reference data comes from the user's tablet.

**What the tool does**:
- lists HID devices; the user picks their tablet;
- records the HID report descriptor, device information (VID, PID, product strings, interfaces)
  and a session of timestamped raw reports while the user follows on-screen instructions ("hover
  without touching", "press lightly then hard", "click button 1", "tilt the pen"…);
- writes a single versioned file (`.ntdcap`, JSON or binary), **without personal data**
  (truncated serial number, no machine name or path);
- ships as a standalone binary documented in `docs/adding-a-tablet.md`. The `tablet_support.yml`
  issue template asks for this file.

**Uses**:
1. Adding a tablet: the capture is used to write the descriptor, then becomes a golden file and a
   `ntd-tester` scenario.
2. Bug reports: replay the reporter's capture instead of guessing.
3. Initial fuzzing corpus (§15.4).

### 15.4 Fuzzing the parsers and the protocol

**v2 state**: `qa/fuzz/fuzz_targets/` only covers maths (`rotate_point`, `normalized_to_screen`,
`physical_to_normalized`, `apply_relative_delta`). **No HID parser is fuzzed**, although the
parsers are the surface that receives external bytes.

**To add** (cargo-fuzz):
- one target per parser family (`wacom`, `huion`, `xp_pen`, `uclogic`, `veikk`…) that feeds
  arbitrary bytes to the parser and checks it returns `Ok`/`Err`/`None` **without panicking**, and
  that produced values stay within the descriptor's bounds;
- one target for decoding the `ntd-interop` command protocol and one for incoming websocket
  messages (v2 and v3);
- one target for loading profiles and the v2 → v3 migration (`ntd-settings`).

**Corpus**: seeded with `ntd-recorder` captures; every crash found is added to the corpus **and**
becomes a unit test.

### 15.5 `ntd-stress`: websocket and IPC stress

**Equivalent** of `pumpkin-fuzzer`, which hammers a running server with random packets.

**What the tool does**:
- connects to a running NTD (local websocket, protocols v2 and v3, and the interop command
  protocol);
- modes: random messages, valid but out-of-range messages, corrupted or truncated frames, burst
  connections and disconnections, slow clients that never read;
- options: duration, concurrency, mode;
- checks at the end that the process is alive, memory is back to normal and pipeline latency has
  not moved.

It runs in the nightly workflow (§13.5), not on every PR: it is a long test.

### 15.6 `cargo xtask ci-plan`: CI per touched crate

**Inspired by** Pumpkin's `.github/scripts/ci_packages.py`, but **written in Rust in `xtask`**
rather than Python: one language for tooling, and it reuses `cargo metadata`.

**What the command does**:
- computes the files changed between the PR base and head;
- maps them to crates, then adds **every crate that depends on them** (reverse graph from
  `cargo metadata`);
- switches to a full run if a global file changes (list in §13.1), on `main`, on tags and on
  nightly;
- writes the crate list and the test matrix to `$GITHUB_OUTPUT` (Linux and Windows x64 on PRs,
  full matrix with ARM64 on full runs).

The `clippy`, `test` and `build` jobs read this output and pass `-p <crate>` for each crate. The
command has its own unit tests (fake dependency graph).

---

## 16. Target platforms

### 16.1 Goal

Eventually, NextTabletDriver must run **reliably** on **Windows, Linux and macOS**, on every
common architecture of each: x86_64 (Intel/AMD 64-bit), 32-bit x86, 64-bit ARM and, on Linux,
32-bit ARM; on macOS, Intel and Apple Silicon.

**This is not the goal of 3.0.** v2 only handles Windows and Linux (`cfg(windows)` and
`cfg(target_os = "linux")` in `engine/injector`, `engine/interop`…), and supporting a new platform
is real work (permissions, injection, packaging). The rewrite must, however, **prepare** that
work: each new platform must be an implementation added in places that already exist, not another
redesign.

### 16.2 Matrix and support tiers

| Platform | Rust target | Tier in 3.0 | Later |
|---|---|---|---|
| Windows x64 | `x86_64-pc-windows-msvc` | **1** | — |
| Linux x64 | `x86_64-unknown-linux-gnu` | **1** | — |
| Windows ARM64 | `aarch64-pc-windows-msvc` | **2** | 1 |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | **2** | 1 |
| macOS Apple Silicon | `aarch64-apple-darwin` | 3 | 2, then 1 |
| macOS Intel | `x86_64-apple-darwin` | 3 | 2 (universal binary with Apple Silicon) |
| Windows 32-bit x86 | `i686-pc-windows-msvc` | 3 | 2 |
| Linux 32-bit x86 | `i686-unknown-linux-gnu` | 3 | 2 |
| Linux 32-bit ARM | `armv7-unknown-linux-gnueabihf` | 3 | 2 |

Out of scope: 32-bit macOS (it no longer exists) and 32-bit Windows ARM (abandoned by Microsoft).

Tier definitions:
- **Tier 1**: complete features, tests and `ntd-tester` in CI on a native runner, shipped in every
  release, bugs block a release.
- **Tier 2**: complete features, build and tests in CI (native runner, or QEMU / `cross`
  emulation for 32-bit ARM and 32-bit x86 on Linux), shipped in releases marked "experimental".
- **Tier 3**: **prepared, not implemented**. The workspace builds for this target (`cargo clippy
  --target …` blocking in CI), the OS modules exist and return `PlatformError::Unsupported`.
  Nothing is shipped.

A platform moves up a tier by explicit decision, when it meets the criteria; never "because it
compiles".

### 16.3 What the rewrite must prepare now

**OS adapter structure.** Every crate that touches the OS (`ntd-hid`, `ntd-output`,
`ntd-interop`, `ntd-platform`) has the same shape:

```
src/
  lib.rs          # public trait + implementation chosen by target_os
  windows.rs
  linux.rs
  macos.rs        # in 3.0: returns PlatformError::Unsupported
  unsupported.rs  # any other OS: explicit error, still compiles
```

```rust
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::Backend;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::Backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::Backend;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub use unsupported::Backend;
```

**Capabilities rather than assumptions.** Platforms do not all offer the same things (pressure
injection, tilt, absolute pointer). Traits expose capabilities:

```rust
pub struct OutputCapabilities {
    pub absolute_pointer: bool,
    pub pressure: bool,
    pub tilt: bool,
    pub relative_pointer: bool,
}

pub trait OutputSink {
    fn capabilities(&self) -> OutputCapabilities;
    // ...
}
```

The engine and the frontend read these capabilities: the frontend hides or disables an option
that is not available instead of showing it with no effect.

**Architecture independence**:
- fixed-width types in everything serialised or crossing a boundary (rule 106);
- bytes read with an explicit byte order (rule 109);
- every `Atomic*` used must exist on every target of §16.2: check
  `cfg(target_has_atomic = "64")` before using `AtomicU64` in a shared crate;
- tier 3 CI catches these problems early, before a platform is even implemented.

**Plugins and SDK**:
- the `ntd-plugin-api` ABI uses only fixed-width, `repr(C)` types: the same ABI works for 32 and
  64 bits;
- a plugin's manifest declares its target triple; the host refuses a plugin for another target
  (rule 111). The plugin distribution format allows **several binaries** (one per target) from 3.0
  on, even if only two are published;
- `apps/sdk` and its bindings (C, C#) are built per target, with the same artifact naming as the
  application.

**Tablet descriptors**: the `ntd-codegen` schema (§15.1) has an optional per-OS field for cases
where the HID interface or collection to open differs by system.

**Update and packaging**:
- the update manifests (`latest.json` and the beta/nightly channels) use the Tauri updater
  platform keys (`windows-x86_64`, `linux-aarch64`, `darwin-aarch64`…); the script that generates
  them takes the target list as input instead of hardcoding it;
- artifact names follow a single format: `NextTabletDriver_<version>_<os>_<arch>.<ext>`.

**Frontend**: `infrastructure/os.ts` exposes the platform (OS, architecture) and capabilities;
components that depend on them (`TitleBar.svelte`, keyboard shortcuts, menus) never test the OS
themselves (rule 114).

### 16.4 `ntd-platform`

A small crate for system integration that is neither HID nor injection, and that is scattered
today:

| Function | Windows | Linux | macOS (later) |
|---|---|---|---|
| Autostart | `Run` registry key | `.desktop` in `~/.config/autostart` | LaunchAgent |
| Single instance | named mutex | socket or file lock | socket or file lock |
| Polling thread priority | `SetThreadPriority` | `sched_setscheduler` | thread QoS |
| High-resolution timer (`force_high_resolution_timer`) | `timeBeginPeriod` | n/a | n/a |
| Directories (config, plugins, logs) | via `directories`, passed as `Paths` (§4.4) | same | same |

### 16.5 Per-platform concerns (for later)

These points are **not** handled in 3.0, but the architecture must not make them hard.

**macOS**:
- **Permissions**: reading HID requires the "Input Monitoring" permission and injection requires
  "Accessibility". The application must detect their absence and guide the user, which requires
  typed `PermissionDenied` errors in `ntd-hid` and `ntd-output` from now on.
- **Exclusive access**: to stop the system driver from also moving the cursor, the tablet must be
  opened in exclusive mode (IOKit). `HidBackend` must therefore accept an "exclusive" option from
  3.0, even if only one platform uses it.
- **Signing and notarisation** are mandatory (Apple Developer account, Hardened Runtime). The
  Hardened Runtime blocks loading libraries signed by anyone else: loading third-party plugins
  requires the `com.apple.security.cs.disable-library-validation` entitlement. This must be
  decided explicitly at porting time, and minisign verification (§8) stays mandatory in every case.
- **Shared memory**: POSIX `shm_open` names are limited to 31 characters on macOS; `ntd-interop`
  must not assume longer names.
- **UI**: window buttons on the left, global menu bar, `Cmd` shortcuts.
- **Universal binary** (Intel + Apple Silicon) rather than two downloads.

**Linux**:
- udev rules for `hidraw` and `uinput` access without root, shipped in the packages;
- X11 and Wayland: `uinput` injection works on both, but screen detection differs and must go
  through a trait;
- package formats per architecture (deb, rpm, AppImage), WebKitGTK available on each target.

**32-bit (Windows x86, Linux x86, 32-bit ARM)**:
- check that Tauri, WebView2 (Windows x86) and WebKitGTK are available for the target;
- more limited memory: no "large by default" buffers in the pipeline.

**Windows ARM64**: check that every official plugin and the SDK are published for ARM64, not
just the application.

---

## 17. Roadmap

### 17.1 Current state

| | |
|---|---|
| **Current phase** | Phase 0: Safety nets |
| **Last completed task** | none |
| **Next task** | P0.1 |
| **Release freeze** | not started (starts after P0.2) |
| **Last updated** | 2026-10-31 |

This table is updated by **every** PR that ticks a task.

### 17.2 How to use the roadmap

**Legend**:
- `[ ]` to do
- `[~]` in progress: add the branch or PR and the author, e.g. `[~] (branch refactor/p2-3-tablets, @handle)`
- `[x]` done: add the link to the PR that completed it
- `[-]` dropped: add the reason and a link to the decision

**Rules** (for humans and LLMs):
1. **This file, on `main`, is authoritative.** No issue, conversation or agent memory replaces a
   box ticked here.
2. **Which task to take**: the **first** `[ ]` task of the current phase whose dependencies
   ("Depends on") are all `[x]`. Two tasks of the same phase with no dependency between them can
   progress in parallel, by different people.
3. **No skipping phases.** A phase starts when the previous phase's **gate** is ticked. A gate is
   ticked only when all its criteria are verified, with the commands to prove it.
4. **Before starting**: check that no open PR or branch already handles the task, then set it to
   `[~]` (small roadmap PR, or first commit of the branch).
5. **One task = one PR**, unless marked "(several PRs)". The PR that completes the task ticks it
   **in the same commit** and updates §17.1.
6. **"Done when" is a contract.** Do not tick if a single criterion is not verified. A half-done
   task stays `[~]`.
7. **A badly scoped task** (too big, missing dependency) is fixed first in a PR that only changes
   the roadmap, then work resumes.
8. **For an LLM**: work on **one** task at a time, the one rule 2 designates or the one a
   maintainer named; reread the referenced section (§…) and the "DO NOT" rules (§9) before
   starting; never tick a box without running the commands of rule 84; do not open a PR
   (rule 88).
9. **Every PR** also goes through the PR checklist (§10).

---

### Phase 0: Safety nets

Goal: be able to detect any regression **before** touching the architecture (§7, phase 0).

- [ ] **P0.1** Fix the updater public key (§11.1)
  - Depends on: —
  - Done when: `tauri.conf.json` contains the real public key matching
    `TAURI_SIGNING_PRIVATE_KEY`; a signed test update is accepted by the application and an
    update signed with another key is refused.
- [ ] **P0.2** Transition 2.x release, then start the freeze (§2)
  - Depends on: P0.1
  - Done when: a 2.x release with the real key is published and installed v2 builds update to it;
    the README and release notes announce the freeze until `3.0.0-beta.1`; §17.1 says the freeze
    has started. No 2.x release is published after this (rule 117).
- [ ] **P0.3** Check the licence of `tablets/OpenTabletDriver` (§11.2)
  - Depends on: —
  - Done when: the upstream licence is identified, the required attribution is added (or
    incompatible files are replaced), and the decision is written in the PR.
- [ ] **P0.4** Remove the duplicate icon library (§11.3)
  - Depends on: —
  - Done when: a single Lucide library in `frontend/package.json`; `npm run check`, `npm test` and
    `npm run e2e` green.
- [ ] **P0.5** Write `ntd-recorder` v1 (§15.3)
  - Depends on: —
  - Note: it lives in `tools/ntd-recorder` with its own `Cargo.lock` **temporarily**; it joins the
    workspace in P1.2. This is the only exception to rule 21.
  - Done when: the tool lists devices, captures descriptor + guided reports, writes a versioned
    `.ntdcap` without personal data; `docs/adding-a-tablet.md` and
    `.github/ISSUE_TEMPLATE/tablet_support.yml` ask for this file.
- [ ] **P0.6** Golden file harness (`qa/golden`)
  - Depends on: P0.5
  - Done when: `cargo test` replays every capture through the v2 parser and compares it with a
    committed snapshot; changing one byte in a parser fails the test.
- [ ] **P0.7** Golden files for the supported tablets (several PRs)
  - Depends on: P0.6
  - Done when: every tablet with a real capture (or real reports in the existing tests) has golden
    files; tablets without a capture are listed in `qa/golden/MISSING.md` (rule 103: captures are
    never invented).
- [ ] **P0.8** Latency baseline
  - Depends on: —
  - Done when: a criterion benchmark covers the parse → pipeline → (fake) output path; the results
    and the measuring machine are recorded in `docs/perf-baseline.md`; the 5 % tolerance is
    written next to them.
- [ ] **P0.9** Generate the v2 contract with `tauri-specta` (§5.2, step 1)
  - Depends on: —
  - Done when: the 26 current commands and 5 events have types and typed functions generated from
    Rust in `frontend/src/lib/infrastructure/tauri/bindings.ts`, **with no behaviour change**; a CI
    job regenerates and fails on any diff.
- [ ] **P0.10** The frontend uses the generated bindings
  - Depends on: P0.9
  - Done when: `commands.ts` and `events.ts` have no hand-written DTO interface left;
    `npm run check`, `npm test`, `npm run e2e` green.
- [ ] **GATE 0**
  - Done when: P0.1 to P0.10 are `[x]`, CI is green, the freeze has started and
    `docs/perf-baseline.md` exists.

---

### Phase 1: Workspace, quality, CI, contribution rules

Goal: one workspace and the whole quality policy in place, **with no behaviour change** (§7,
phase 1; §12, §13, §14).

- [ ] **P1.1** Create the workspace and move the applications
  - Depends on: GATE 0
  - Done when: `[workspace]` at the root; `src-tauri` → `apps/desktop`, `sdk` → `apps/sdk`,
    `sdk/plugin_api` → `crates/ntd-plugin-api`, official plugins are members; **one**
    `Cargo.lock`; `cargo build --workspace` and `cargo test --workspace` green; pure move diff
    (`git diff -M`).
- [ ] **P1.2** Bring `tools/ntd-recorder` into the workspace
  - Depends on: P1.1
  - Done when: its separate `Cargo.lock` is deleted.
- [ ] **P1.3** `[workspace.dependencies]` and `[workspace.package]`
  - Depends on: P1.1
  - Done when: no member crate declares a dependency version (rule 22); metadata is inherited.
- [ ] **P1.4** `stable` toolchain (§12.6)
  - Depends on: P1.1
  - Done when: `rust-toolchain.toml` on `channel = "stable"`, no Rust version in workflows,
    `rust-version` declared once in `[workspace.package]`.
- [ ] **P1.5** Formatting and editor (§12.1)
  - Depends on: P1.1
  - Done when: `rustfmt.toml` and `.editorconfig` match §12.1; `cargo fmt --all --check` green.
- [ ] **P1.6** Introduce `[workspace.lints]` as `warn` (§12.3)
  - Depends on: P1.3
  - Done when: the §12.3 block is in place with the groups as `warn` **temporarily**; the number
    of warnings per crate is recorded in the PR.
- [ ] **P1.7** Fix lint warnings (several PRs, one area per PR)
  - Depends on: P1.6
  - Done when: zero warnings on the workspace; each PR only fixes lints, with no behaviour change
    (golden files and E2E identical).
- [ ] **P1.8** Switch the lints to `deny`
  - Depends on: P1.7
  - Done when: `[workspace.lints]` is exactly the one in §12.3.
- [ ] **P1.9** `xtask` and `cargo xtask lint-style` (§12.5)
  - Depends on: P1.1
  - Done when: the command checks the 5 points of §12.5, has tests, and passes on the repository.
- [ ] **P1.10** `typos` and `cargo-machete`
  - Depends on: P1.1
  - Done when: `typos.toml` exists and both tools pass on the repository.
- [ ] **P1.11** `cargo xtask ci-plan` (§15.6)
  - Depends on: P1.9
  - Done when: the command produces the list of touched crates + dependents and the matrix, with
    tests on a fake graph.
- [ ] **P1.12** Frontend quality tooling as warnings (§12.7)
  - Depends on: —
  - Done when: Prettier (`printWidth: 100`), the TypeScript options and the ESLint rules of §12.7
    are configured, temporarily as warnings; `npm run format:check` exists; the number of
    warnings is recorded in the PR.
- [ ] **P1.13** Fix frontend warnings (several PRs)
  - Depends on: P1.12
  - Done when: zero ESLint, `svelte-check` and TypeScript warnings; Prettier applied; no behaviour
    change (E2E identical); `infrastructure/log.ts` is the only place using `console`.
- [ ] **P1.14** Switch the frontend rules to errors
  - Depends on: P1.13
  - Done when: `eslint --max-warnings 0`, `svelte-check --fail-on-warnings` and
    `prettier --check` are in place as in §12.7.
- [ ] **P1.15** Rework `ci.yml` (§13.1, §13.2)
  - Depends on: P1.8, P1.9, P1.10, P1.11, P1.14
  - Done when: the §13.2 jobs that already have something to check exist (fmt, style, typos,
    machete, clippy, clippy-release, doc, test, golden, bindings, coverage, bench, frontend, e2e,
    build); actions pinned by SHA; `ci-success` aggregates everything.
- [ ] **P1.16** Rework `security.yml` (§13.3)
  - Depends on: P1.1
  - Done when: a single `cargo deny check`, `npm audit`, dependency review on PRs; secret scanning
    and push protection enabled.
- [ ] **P1.17** `toolchain-beta.yml` (§12.6)
  - Depends on: P1.15
  - Done when: weekly non-blocking job that opens an issue on failure.
- [ ] **P1.18** Protect `main` (§14.4)
  - Depends on: P1.15
  - Done when: `ci-success` is the only required check; direct pushes to `main` are forbidden;
    the maintainer can merge their own PRs after green CI.
- [ ] **P1.19** Root `AGENTS.md` and `CLAUDE.md` (§14.1)
  - Depends on: P1.1
  - Done when: `AGENTS.md` describes the project, layout and commands, and points to §9 and §17;
    `CLAUDE.md` is a symlink to it.
- [ ] **P1.20** Contribution: language, AI policy, review, closing criteria, PR template
  (§14.2 to §14.6)
  - Depends on: —
  - Done when: `.github/CONTRIBUTING.md` and `.github/pull_request_template.md` are up to date and
    in English.
- [ ] **GATE 1**
  - Done when: P1.1 to P1.20 are `[x]`; CI green; golden files, latency (within +5 %) and E2E
    identical to phase 0.

---

### Phase 2: Extract the pure crates

Goal: `ntd-core`, `ntd-tablets`, `ntd-pipeline` without any I/O (§7, phase 2).

- [ ] **P2.1** Extract `ntd-core`
  - Depends on: GATE 1
  - Done when: `core/input`, `core/math` and the really shared types are in `crates/ntd-core`;
    `cargo tree -p ntd-core` shows no I/O dependency.
- [ ] **P2.2** Remove the `drivers` → `engine::state` dependency
  - Depends on: P2.1
  - Done when: `parsers/wacom/intuos_v1.rs` and `pl.rs` no longer import `engine::state`
    (parser-local state, or helper moved to `ntd-core`); golden files identical.
- [ ] **P2.3** Extract `ntd-tablets`
  - Depends on: P2.2
  - Done when: parsers, models and descriptor loading are in `crates/ntd-tablets`; no I/O; golden
    files run against it.
- [ ] **P2.4** `ntd-codegen` (§15.1)
  - Depends on: P2.3
  - Done when: `tablets/*.json` is validated and generates `crates/ntd-tablets/src/generated/`;
    `include_dir` and JSON parsing at startup are gone; CI `codegen` job;
    `tools/ntd-codegen/AGENTS.md` and `crates/ntd-tablets/AGENTS.md` written.
- [ ] **P2.5** Extract `ntd-pipeline` with `trait Filter` (§4.4)
  - Depends on: P2.1
  - Done when: `engine/pipeline` and the filter chain are in `crates/ntd-pipeline`; the
    `filters` ↔ `engine::plugins` cycle no longer exists; latency within +5 %.
- [ ] **P2.6** Clock injected into the pipeline
  - Depends on: P2.5
  - Done when: no `Instant::now()` or `sleep` in `ntd-pipeline` (rule 37).
- [ ] **P2.7** Parser fuzzing (§15.4)
  - Depends on: P2.3
  - Done when: one target per parser family, corpus seeded with captures, CI `fuzz` job of 60 s
    per target.
- [ ] **GATE 2**
  - Done when: P2.1 to P2.7 are `[x]`; `cargo tree` confirms no I/O in the three crates; golden
    files identical; latency within +5 %.

---

### Phase 3: Extract the I/O adapters

Goal: every OS access behind a trait, with a fake and room for each platform (§7, phase 3; §16.3).

- [ ] **P3.1** `ntd-test-support`
  - Depends on: GATE 2
  - Done when: fakes of `HidBackend`, `OutputSink`, `Clock` usable by every crate as a
    dev-dependency.
- [ ] **P3.2** `ntd-platform` (§16.4)
  - Depends on: P3.1
  - Done when: autostart, single instance, thread priority and high-resolution timer live in this
    crate, with the `windows` / `linux` / `macos` / `unsupported` structure.
- [ ] **P3.3** `ntd-hid`
  - Depends on: P3.1
  - Done when: `HidBackend` trait with an "exclusive" option and a `PermissionDenied` error; the
    only crate depending on `hidapi`; §16.3 structure.
- [ ] **P3.4** `ntd-output`
  - Depends on: P3.1
  - Done when: `OutputSink` trait with `capabilities()`; Windows and Linux implementations;
    `macos` returns `Unsupported`.
- [ ] **P3.5** `ntd-interop`
  - Depends on: P3.1
  - Done when: pure protocol separated from transport; shared memory names ≤ 31 characters;
    protocol fuzz target.
- [ ] **P3.6** `ntd-settings`
  - Depends on: P3.1
  - Done when: `Paths` injected (no global `get_*_dir()` left); versioned files; atomic writes; a
    corrupted file is backed up and reported (§8).
- [ ] **P3.7** `ntd-plugin-host` and ABI v3
  - Depends on: P2.5, P3.6
  - Done when: ABI v3 defined in `ntd-plugin-api` (target triple in the manifest, fixed-width
    types); signature verified before loading; other targets and v2 plugins refused with a clear
    message; a panicking plugin is disabled without stopping the pipeline; official plugins
    rebuilt for ABI v3; `docs/plugin-migration-v3.md` and `crates/ntd-plugin-api/AGENTS.md`
    written.
- [ ] **P3.8** Remove the `desktop` feature
  - Depends on: P3.3, P3.4, P3.5
  - Done when: the feature no longer exists; `apps/sdk` depends only on `ntd-core`,
    `ntd-tablets`, `ntd-hid`, `ntd-pipeline`.
- [ ] **P3.9** CI `check-targets` job (§16.2)
  - Depends on: P3.2, P3.3, P3.4, P3.5
  - Done when: `cargo clippy` passes on every tier 3 target, and the job is blocking.
- [ ] **GATE 3**
  - Done when: P3.1 to P3.9 are `[x]`; no OS-specific code outside the allowed crates (rule 108);
    golden files, latency and E2E identical.

---

### Phase 4: The engine

Goal: `ntd-engine` driven only through `EngineHandle` and `EngineEvent` (§4.5; §7, phase 4).

- [ ] **P4.1** `ntd-engine` skeleton
  - Depends on: GATE 3
  - Done when: `Engine::start`, `EngineHandle`, `EngineEvent`, `EngineDeps` exist as in §4.5; no
    `Arc<Mutex<…>>` exposed outside the crate.
- [ ] **P4.2** Split `polling.rs` and `owner.rs` (several PRs)
  - Depends on: P4.1
  - Done when: reading, parsing, pipeline, output and buttons are separate modules; any file over
    about 400 lines is justified in its PR (rule 29).
- [ ] **P4.3** Lock-free configuration in the hot path
  - Depends on: P4.1
  - Done when: configuration arrives through a channel or atomic swap (rule 35); no allocation in
    the loop (rule 33), checked by a test or an allocation counter.
- [ ] **P4.4** Telemetry moved out of the engine
  - Depends on: P4.1
  - Done when: `ntd-engine` no longer references telemetry; it publishes `EngineEvent`s.
- [ ] **P4.5** `ntd-tester` (§15.2)
  - Depends on: P4.1, P3.1
  - Done when: every capture has at least one scenario in `qa/scenarios/`; CI `tester` job;
    `crates/ntd-engine/AGENTS.md` written.
- [ ] **GATE 4**
  - Done when: P4.1 to P4.5 are `[x]`; `ntd-tester` green on every capture; latency within +5 %.

---

### Phase 5: Services and application

Goal: `apps/desktop` only does wiring (§7, phase 5).

- [ ] **P5.1** `ntd-services`
  - Depends on: GATE 4
  - Done when: autoupdate, telemetry (on by default, can be turned off, no personal data) and
    supervisor live in this crate and consume `EngineEvent`s.
- [ ] **P5.2** Websocket v3 and v2 compatibility (§5.4)
  - Depends on: P5.1
  - Done when: the websocket binds to `127.0.0.1` by default, serves the versioned v3 protocol and
    the unchanged v2 protocol; both have tests and a fuzz target; the v3 protocol is documented in
    `docs/integrations.md`.
- [ ] **P5.3** `ntd-ipc`
  - Depends on: GATE 4
  - Done when: the bindings generated in P0.9 come from this crate; `crates/ntd-ipc/AGENTS.md`
    written.
- [ ] **P5.4** SDK v3 (§5.4)
  - Depends on: GATE 4
  - Done when: `apps/sdk` exposes the v3 API, its C and C# bindings are regenerated, and
    `docs/sdk-migration-v3.md` explains the changes.
- [ ] **P5.5** `apps/desktop` as wiring only
  - Depends on: P5.1, P5.2, P5.3
  - Done when: no business logic in `apps/desktop`; each command is at most 10 lines.
- [ ] **P5.6** Virtual root manifest
  - Depends on: P5.5
  - Done when: the root `Cargo.toml` no longer has a `[package]`; the root `src/` directory no
    longer exists.
- [ ] **P5.7** `ntd-stress` (§15.5)
  - Depends on: P5.2
  - Done when: the tool covers the §15.5 modes on both websocket protocols and does not bring down
    a running NTD.
- [ ] **GATE 5**
  - Done when: P5.1 to P5.7 are `[x]`; the `cargo tree` graph matches §4.3; E2E, `ntd-tester` and
    golden files green.

---

### Phase 6: v3 contract and frontend

Goal: the contract split and typed, and the frontend data layer fixed (§5, §6.3).

- [ ] **P6.1** Split `Config` by domain in the backend
  - Depends on: GATE 5
  - Done when: `Mapping`, `PenSettings`, `Bindings`, `FilterChain`, `AppPreferences`,
    `WebSocketSettings` each have their `get_*` / `update_*` commands; `set_config(everything)` no
    longer exists.
- [ ] **P6.2** Strong types in the contract
  - Depends on: P6.1
  - Done when: bindings, modes and curve types are `enum`s; commands return a typed
    `Result<T, IpcError>`.
- [ ] **P6.3** Real-time stream through `Channel`
  - Depends on: P6.1
  - Done when: `tablet-event` goes through `tauri::ipc::Channel`, decimated in the backend to
    about 60 Hz.
- [ ] **P6.4** A single gateway to Tauri in the frontend
  - Depends on: P6.1
  - Done when: the ESLint `no-restricted-imports` rule is active; no more `invoke` in
    `MenuBar.svelte` or `listen` in `TabletVisualizer.svelte`.
- [ ] **P6.5** Stores per domain
  - Depends on: P6.2, P6.4
  - Done when: one store per domain; no `JSON.stringify` comparison or synchronisation
    `setTimeout` left; the frontend shows the state returned by the backend.
- [ ] **P6.6** Capabilities and platform in the UI
  - Depends on: P6.5
  - Done when: `os.ts` exposes OS, architecture and capabilities; an unavailable option is hidden
    or disabled; `TitleBar.svelte` and shortcuts no longer test the OS themselves.
- [ ] **P6.7** v2 → v3 profile migration
  - Depends on: P6.1
  - Done when: real v2 files as fixtures are imported without loss, including the telemetry
    choice; the original is backed up; profile loading fuzz target.
- [ ] **GATE 6**
  - Done when: P6.1 to P6.7 are `[x]`; E2E green; real v2 profiles imported without loss.

---

### Phase 7: Release 3.0

Goal: ship v3 without leaving a single v2 user behind (§2, §13).

- [ ] **P7.1** `beta` and `nightly` update channels on GitHub Pages
  - Depends on: GATE 6
  - Done when: `beta.json` and `nightly.json` are served from GitHub Pages at stable URLs; the
    "receive betas" option exists in Settings; the manifest generator takes the target list as
    input (§16.3).
- [ ] **P7.2** `prerelease.yml` (§13.4)
  - Depends on: P7.1
  - Done when: the workflow checks the tag, the version and the signature against the embedded
    key, then publishes a pre-release with SHA-256 checksums and updates `beta.json`.
- [ ] **P7.3** `nightly.yml` (§13.5)
  - Depends on: P7.1
  - Done when: nightly build only if there were commits, `nightly` tag overwritten, long fuzzing
    and `ntd-stress` run, `nightly.json` updated.
- [ ] **P7.4** `release.yml` and crates.io publication (§13.6)
  - Depends on: P7.2
  - Done when: publication as `latest`, multi-target `latest.json`, notes generated with
    `rustc -Vv`, winget; the `plugin-api-v*` tag publishes `ntd-plugin-api` to crates.io and checks
    its major version against the ABI version.
- [ ] **P7.5** `3.0.0-beta.1` and a 4-week beta
  - Depends on: P7.2, P7.4
  - Done when: the beta is published (this ends the freeze); after 4 weeks there is no open
    blocking bug on tier 1 platforms.
- [ ] **P7.6** `3.0.0`
  - Depends on: P7.5
  - Done when: the release is published and reaches 2.x installs through auto-update; README and
    `docs/` are up to date, with the §16.2 support tiers (rule 112), the websocket v2 deprecation
    notice, the plugin and SDK migration guides.
- [ ] **P7.7** Announce the end of 2.x
  - Depends on: P7.6
  - Done when: the release notes and README state that 2.x is no longer maintained.
- [ ] **GATE 7: v3.0 done**

---

### After 3.0: platforms (§16)

These tasks have no imposed order between them. Each moves a platform up a tier according to the
§16.2 criteria.

- [ ] **P8.1** Windows ARM64 and Linux ARM64: tier 2 → 1
- [ ] **P8.2** macOS Apple Silicon: tier 3 → 2 (permissions, exclusive access, injection, signing
  and notarisation, decision on third-party plugins, §16.5)
- [ ] **P8.3** macOS Intel: tier 3 → 2 (universal binary with P8.2)
- [ ] **P8.4** Windows 32-bit x86: tier 3 → 2
- [ ] **P8.5** Linux 32-bit x86: tier 3 → 2
- [ ] **P8.6** Linux 32-bit ARM: tier 3 → 2
- [ ] **P8.7** macOS: tier 2 → 1
