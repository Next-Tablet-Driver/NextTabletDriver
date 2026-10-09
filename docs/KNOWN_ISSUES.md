# Known issues and remaining work

Findings from the production-readiness audit that are **not fixed yet**, so they are not
forgotten. Items already fixed are not listed; see the git history of the
`fix/security-hardening`, `chore/remove-egui`, `chore/tauri-release` and
`fix/frontend-backend-cleanup` branches.

Severity: **High** (fix before relying on it), **Medium**, **Low**.

---

## Release and distribution

| Sev. | Item | Notes |
|---|---|---|
| High | Updater signing keys are not provisioned | `plugins.updater.pubkey` in `src-tauri/tauri.conf.json` is still a placeholder; the `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets must exist or the release build fails. Generate with `tauri signer generate`. |
| High | The release workflow has never run | Trigger `Release Build` manually (dry run) and check all four platforms; Linux and ARM64 were never built. |
| Medium | Users of the old (Inno Setup / egui) installer must update by hand once | The Tauri updater cannot reach them. The new NSIS installer removes the old install silently (`src-tauri/windows/hooks.nsh`), but that hook has not been exercised on a real machine. |
| Medium | winget manifest | The installer type changes from Inno to NSIS; the first automated submission may need a manual manifest update. |
| Medium | Nix package is untested | `npmDeps.hash` in `nix/default.nix` is `lib.fakeHash`; the `build-package` job is `continue-on-error` until it is filled in. |
| Low | `NTD_PLUGIN_PUBKEY` is not provisioned | Needed only for first-party plugins that should load without a prompt; see `docs/PLUGINS_SYSTEM.md`. |
| Low | Linux injector build is unverified locally | `src/engine/injector/linux.rs` was changed (fallible constructor) but could only be checked by CI. |

## Engine / backend

| Sev. | Item | Notes |
|---|---|---|
| Low | Hot path: remaining cost per packet | Clock reads were the dominant cost and are fixed (306 -> 122 ns per packet, see `qa/benches/RESULTS_hot_path.md`). Left: `Projector::project_relative` still reads the clock once per packet (about 30 ns in relative mode); the shared-memory publication and the real OS injection are not benchmarked. Lock-free shared state was judged not worth it: locks were not a measurable cost. |
| Medium | HID read buffer is fixed at 64 bytes | Longer reports are truncated silently. Verify against tablets with long reports and size the buffer from the device's report descriptor. |
| Medium | Plugin trust is checked before loading, not atomically with it | A file replaced between hashing and `LoadLibrary` is still loaded. Same-user malware can do worse anyway, but copying the verified bytes to a private directory and loading from there would close the gap. |
| Medium | A trusted plugin is not sandboxed | Trust only decides *who* may run native code in the process. A panic in a plugin is contained by `catch_unwind` in the API macro, but a crash or hang is not. Out-of-process plugins would be the real fix. |
| Medium | Existing plugins become "pending" after the update that introduced trust | Users must approve them once in the Filters tab. Mention in the release notes. |
| Medium | Telemetry is opt-out | PostHog (EU host). Consider opt-in with an explicit first-run prompt. |
| Medium | Corrupt `app_preferences.json` is overwritten with defaults | Unlike `last_session.json` it is not set aside first (`src/settings/app_preferences.rs`). |
| Low | Crash-report path anonymization is naive | `anonymize_path` replaces the user name wherever it occurs, so a very short name mangles the message; replace path prefixes instead. |
| Low | Shared-memory segment permissions | Check the Windows security descriptor (`CreateFileMappingW`, default DACL) and the Linux mode so other local users cannot write to it. |
| Low | Two interop tests are environment-sensitive | `interop::lock::only_one_owner_at_a_time` and `interop::command::listener_dispatches_commands_to_handler` use machine-wide named objects and fail while another instance of the driver is running. |
| Low | Settings folder is named `NextTabletReader` | Historical project name (`directories::ProjectDirs`); renaming needs a migration. |
| Low | `ureq` is on 2.x; MSRV is 1.99 | Consider `ureq` 3 and whether the high MSRV is intended. |

## Frontend

| Sev. | Item | Notes |
|---|---|---|
| Medium | Test coverage is thin | Covered: the Tauri/OS/event wrappers, `MappingState` (load, debounce, undo/redo, save), and two components. Not covered: the page components, the visualizers, the pen-settings and filters UIs. |
| Low | `src-tauri` is not covered by `cargo fmt --check` | Its sources contain trailing whitespace and unformatted code. The crate is linted (`unwrap`/`expect`/`panic` denied, clippy in CI) but not formatted. |
| Low | `plugin-updater` is both statically and dynamically imported | Vite warns (`INEFFECTIVE_DYNAMIC_IMPORT`); harmless but noisy. |

## Theming

The design system and user themes are in place (`docs/THEMES.md`). What is **not** done or **not verified**:

| Sev. | Item | Notes |
|---|---|---|
| Medium | Import through the native dialog was not run end to end | The flow is covered by unit tests with the backend mocked, and by the dev mock in a browser; the real Tauri file picker and `Settings/Themes/` on disk were not exercised. Try: import `docs/themes/examples/paper.json`, restart, check the theme is kept without a flash. |
| Medium | Not every built-in theme was reviewed screen by screen | Dark (the default) is verified identical to before by computed-style snapshots of all 7 pages. Latte, and the design-system gallery in Macchiato, were checked by eye; Light, Frappe and Mocha were only checked through the automated token-completeness test. |
| Low | `color-mix()` is required | Used for status tints and derived colors. Fine on WebView2 and recent WebKitGTK (>= 2.38); an older WebKitGTK would lose those tints. |
| Low | The visual regression tooling is local only | `window.__ntdSnapshot` and `/?mock` (`frontend/src/dev/`) compare computed styles by hand; they are not wired into CI. |
| Low | Theme list is read on demand | Editing a file in the themes folder needs the **Reload** button; there is no file watcher. |
| Low | Contrast check covers four pairs | Text on the window, on panels, secondary text on panels, and text on the accent. It warns, never blocks. |
| Low | 11 overlay/shade alpha steps | `--overlay-3 ... --overlay-90`, `--shade-10 ... --shade-30` preserve the exact translucency the components used; a later pass could merge them into fewer steps. |
| Low | The select arrow and checkbox drawing are not themed beyond their colors | Shapes are fixed by design (themes change looks, not structure). |

## Repository hygiene

| Sev. | Item | Notes |
|---|---|---|
| Low | AI agent rule files are versioned | `.agents/rules/*` in the repository root. |
| Low | `sdk/Cargo.lock`, `src-tauri/Cargo.lock` and the root lock can drift | The root crate is not a Cargo workspace. Consider a workspace (members: root, `sdk`, `sdk/plugin_api`, `src-tauri`, `plugins/*`) once the Tauri/SDK build profiles are reconciled. |
