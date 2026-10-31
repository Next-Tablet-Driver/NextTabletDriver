# Development

This guide shows how to build NextTabletDriver from source, run it, test it, and send a change. To learn how the code is organized, read [Architecture](architecture.md) first.

## What you need

- **Git**, with submodule support.
- **Rust 1.99 or newer.** Install it with [rustup](https://rustup.rs). The file `rust-toolchain.toml` selects the stable toolchain.
- **Node.js 22 or newer**, with npm. The interface is built with Vite.
- **Windows:** the Visual Studio Build Tools with the "Desktop development with C++" workload, and the [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) (already part of Windows 11).
- **Linux:** development packages for Tauri, `hidapi` (`libudev`) and X11. On Debian and Ubuntu:

  ```bash
  # to build the desktop app (from the "Tauri build" job in .github/workflows/ci.yml)
  sudo apt-get install -y build-essential pkg-config patchelf \
    libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
    libudev-dev libxdo-dev libssl-dev

  # to check and test the root crate (from the "Rust Quality Check" job)
  sudo apt-get install -y pkg-config libudev-dev libgtk-3-dev libglib2.0-dev \
    libgdk-pixbuf2.0-dev libpango1.0-dev libcairo2-dev libatk1.0-dev libx11-dev \
    libxrandr-dev libxfixes-dev libxrender-dev libx11-xcb-dev libxkbcommon-dev \
    libgl1-mesa-dev libegl1-mesa-dev libgles2-mesa-dev libfuse2 libxdo-dev
  ```

- **NixOS or Nix users:** run `nix develop`. It gives you the toolchain and the libraries.

Keep the path of your clone **short** on Windows. Windows limits paths to 260 characters, and Rust build folders are deep. A very long path makes the linker fail with `LNK1104: cannot open file`.

## Get the code

The tablet definitions are a git submodule, and the build embeds them. Clone with submodules:

```bash
git clone --recurse-submodules https://github.com/Next-Tablet-Driver/NextTabletDriver.git
cd NextTabletDriver
```

If you cloned without them, run `git submodule update --init`. Without the submodule, the build stops with an error about `tablets/OpenTabletDriver`.

## Run the app

Install the frontend packages once, then start the app. It rebuilds when you change the code.

```bash
npm --prefix frontend ci
npm run tauri-dev
```

`npm run tauri-dev` starts the Vite server on port 5173 and the Tauri app that loads it.

### Work on the interface without a tablet

You can run the interface alone in your browser. It uses a fake backend (`frontend/src/dev/tauri-mock.ts`) that returns sample data:

```bash
npm --prefix frontend run dev
```

Open `http://localhost:5173/?mock`. The `?mock` part switches on the fake backend. This mode is removed from production builds.

### Build a release app

This builds the app without an installer. It is the same command that the CI uses:

```bash
node frontend/node_modules/@tauri-apps/cli/tauri.js build --no-bundle \
  --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

The program is `target/release/app` (`app.exe` on Windows). The option `createUpdaterArtifacts: false` is needed on your computer because the update files must be signed with a private key that only the release process has. Use a Bash-like shell for the quotes, or escape them in PowerShell.

To make installers, see [Releasing](releasing.md).

## Run the checks

The CI runs these checks on every pull request. Run them before you push.

**The root crate** (the engine):

```bash
cargo fmt --all -- --check
cargo xtask lint-style                          # lines over 100 characters, banners, TODOs
typos                                           # cargo install --locked typos-cli
cargo machete                                   # cargo install --locked cargo-machete
cargo clippy -p next_tablet_driver --all-targets --all-features -- -D warnings
cargo nextest run -p next_tablet_driver --all-features     # cargo install --locked cargo-nextest
cargo test -p next_tablet_driver --all-features --doc
```

We use [nextest](https://nexte.st) because a few tests touch things that belong to the whole process: the panic hook, the log level, and machine-wide names (a named pipe, a mutex and a shared-memory segment). `cargo nextest` runs each test in its own process, so they cannot disturb each other. **Close NextTabletDriver before you run the tests**, because the tests use the same machine-wide names.

**The desktop app crate:**

```bash
cargo clippy --manifest-path apps/desktop/Cargo.toml --all-targets -- -D warnings
```

**One crate on its own.** The commands above already cover the whole workspace (the desktop app, the SDK, the plugin API, the plugins and the Rust example plugin are members). To check a single member, for example after changing only a plugin:

```bash
for manifest in crates/ntd-plugin-api/Cargo.toml apps/sdk/Cargo.toml plugins/*/Cargo.toml apps/sdk/examples/rust_plugin/Cargo.toml; do
  cargo fmt --manifest-path "$manifest" -- --check
  cargo clippy --manifest-path "$manifest" --all-targets -- -D warnings
  cargo nextest run --manifest-path "$manifest" --no-tests=pass
done
```

The CI also builds the two example plugins as real libraries, and checks that each one exports the seven functions that a plugin needs. On Linux you can do the same:

```bash
cargo build --release --manifest-path apps/sdk/examples/rust_plugin/Cargo.toml
cc -std=c11 -O2 -Wall -Wextra -Werror -shared -fPIC apps/sdk/examples/c_plugin/plugin.c -o libc_flip_x.so
nm -D --defined-only libc_flip_x.so | grep ntd_plugin_
```

**The frontend:**

```bash
npm --prefix frontend run format:check      # Prettier, printWidth 100 (`run format` fixes it)
npm --prefix frontend run lint              # ESLint, a warning fails
npm --prefix frontend run check             # TypeScript and Svelte types, a warning fails
npm --prefix frontend run test -- --run    # unit tests (Vitest)
```

**End-to-end tests.** These run the whole interface in a browser with the fake backend, and fail on any `console.error`:

```bash
npx --prefix frontend playwright install chromium    # once
npm --prefix frontend run e2e
```

When a test fails, run `npx playwright show-trace` on the file in `frontend/test-results/`. A smoke test of the real built app also runs in the CI (`frontend/e2e-native/`, with `tauri-driver` on Linux and `scripts/smoke-windows.ps1` on Windows).

**Documentation of the code.** Warnings are errors:

```bash
RUSTDOCFLAGS="-D warnings -A rustdoc::private_intra_doc_links" cargo doc -p next_tablet_driver --no-deps --all-features
```

**Dependency policy** (licenses, banned crates, sources, known security problems), with [cargo-deny](https://embarkstudios.github.io/cargo-deny/):

```bash
cargo deny check bans licenses sources
cargo deny check advisories
```

Run it once: the workspace has a single lockfile. The policy is in `deny.toml`.

### Test coverage

The `Coverage` job measures how much of the production code the tests run, for Rust (without the test code itself) and for the frontend. It writes a report on the pull request. The build fails when coverage falls below the floors in `coverage-thresholds.json` (Rust, frontend, and the two together).

```bash
cargo llvm-cov nextest -p next_tablet_driver --all-features --lcov --output-path lcov.info
npm --prefix frontend run test -- --run --coverage
python scripts/coverage_gate.py --lcov lcov.info --frontend frontend/coverage/coverage-summary.json
```

The floors only go up. When the report says that coverage is well above a floor, raise the floor in your pull request. Lowering one needs a good reason in the review.

### Git hooks

[cargo-husky](https://github.com/rhysd/cargo-husky) installs two hooks into `.git/hooks` the first time you build the tests. They come from `.cargo-husky/hooks/`:

- **pre-commit:** `cargo fmt --check` and `cargo clippy`.
- **pre-push:** the same, plus `cargo nextest run` and the doctests.

If `cargo-nextest` is missing, the pre-push hook tells you how to install it.

## Code style

- Format with `rustfmt` (`rustfmt.toml`) and keep Clippy clean. The root crate turns on `pedantic` and `nursery` lints, and treats warnings as errors in the CI.
- In production code, **do not use** `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `process::exit` or direct indexing like `data[i]`. Clippy denies them. Use `?`, `get()`, `let ... else`, and slice patterns. They are allowed in tests.
- Every `unsafe` block needs a `// SAFETY:` comment that says why it is correct.
- Document public items. A function that can fail needs an `# Errors` section, and one that can panic needs `# Panics`.
- Comments explain *why*, not *what*. Say what the code does only when it is not obvious.
- Put unit tests next to the code in a `#[cfg(test)] mod tests`. Name a test after the behavior it checks. Tests that only check that a call does not panic start with `smoke_`. Prefer tests that return `Result` and use `?`.
- Keep the engine's hot path free of allocations and locks that can block. If you change code in `engine::pipeline`, `core::math` or `filters`, run the benchmarks (see below).
- Frontend: TypeScript with Svelte 5 runes. `npm run lint` and `npm run check` must pass. See [`frontend/README.md`](../frontend/README.md).
- If you add or change a setting, update `core::config::models` (with a default and a repair rule in `validate_and_repair`), the generated TypeScript bindings (see below), and the fake backend in `frontend/src/dev/tauri-mock.ts`. Old settings files must keep loading, so give every new field a default.

## The TypeScript bindings

The commands, events and types the interface exchanges with the backend are described once, in Rust. `frontend/src/lib/infrastructure/tauri/bindings.ts` is generated from them with [tauri-specta](https://github.com/specta-rs/tauri-specta). Never edit it by hand.

When you add or change a command (`apps/desktop/src/commands.rs`, registered in `specta_builder()` in `apps/desktop/src/lib.rs`), an event (`apps/desktop/src/events.rs`) or a type they use (derive `specta::Type`, behind the `specta` feature for the core crate), regenerate the file:

```bash
UPDATE_BINDINGS=1 cargo test --manifest-path apps/desktop/Cargo.toml --test bindings
```

The CI runs the same test without the variable and fails when the committed file is out of date.

## Which crates does my change touch?

```bash
cargo xtask ci-plan --base origin/main
```

It lists the crates your branch changes and every crate that depends on them, which are the ones the CI has to check. It runs everything instead when a global file changed (`Cargo.toml`, `Cargo.lock`, the toolchain and lint files, `deny.toml`, `.github/`, `.cargo/`) or when the event is not a pull request. In the CI it also writes `crates`, `packages` (`-p name` arguments), `matrix` and `full` to `$GITHUB_OUTPUT`.

## Benchmarks and fuzzing

These live in `qa/`:

- **Benchmarks** measure how fast the engine processes a packet: [`qa/benches`](../qa/benches/README.md).
- **Fuzzers** feed the math functions with strange inputs to look for crashes and broken rules: [`qa/fuzz`](../qa/fuzz/README.md).
- **Property tests** (`qa/tests`) check rules of the coordinate math with thousands of random inputs. They run with the normal tests.

## Nix

If you change `frontend/package-lock.json`, update the npm hash in `nix/default.nix`, or the Nix build fails. Run `nix build .#default`. It stops with the hash that it expected, and you paste that hash in the file. The `Nix` workflow checks the modules and the dev shell.

## The CI

A pull request only runs the jobs that its files need. A first job, *Changed paths*, reads the list of changed files, and each job below runs only when a file it builds or checks has changed.

| Job | Runs when you change |
| --- | --- |
| Rust Quality Check | `src/`, `qa/`, `tablets/`, `resources/`, `crates/ntd-plugin-api/`, the root `Cargo.toml` or `Cargo.lock`, the toolchain or lint files (`rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.cargo/`) |
| SDK, plugin API, plugins and examples | `apps/sdk/`, `crates/`, `plugins/`, `tools/`, `src/`, `tablets/`, `resources/` (the SDK embeds the root crate), the root `Cargo.lock` (the only lockfile), any `Cargo.toml`, the toolchain or lint files |
| Frontend, E2E | `frontend/` |
| Coverage | `src/`, `qa/`, `tablets/`, `resources/`, `crates/ntd-plugin-api/`, the root `Cargo.*`, `frontend/`, `coverage-thresholds.json`, `scripts/coverage_gate.py`, the toolchain files |
| Tauri build | `apps/desktop/`, `frontend/`, and what the root crate is built from (`src/`, `tablets/`, `resources/`, `crates/ntd-plugin-api/`, the root `Cargo.*`), the scripts the build uses (`scripts/assert-gui-subsystem.ps1`, `scripts/smoke-windows.ps1`, `scripts/99-nexttabletdriver.rules`), the toolchain files. It does **not** run for `qa/` |
| Dependency policy | any `Cargo.toml` or `Cargo.lock`, `deny.toml` |

So a change only to the documentation, the licences or the issue templates runs none of them; a change only to a plugin runs only the SDK job; a change only to `apps/desktop/` runs only the Tauri build. Changing `.github/workflows/ci.yml` runs everything.

A push to `main` and a manual run always run everything. A job that is skipped counts as passed. The single required check is **CI success**: it passes when every job that ran has passed.

The filter is the `changes` job in `.github/workflows/ci.yml`. If you add a folder that holds code, add it there, or its changes will not trigger the jobs.

| Job | What it checks |
| --- | --- |
| Rust Quality Check (Ubuntu, Windows) | Format, `cargo check`, Clippy, `cargo doc`, tests and doctests of the root crate. |
| SDK, plugin API, plugins and examples | Format, Clippy and tests of these workspace members, the Rust example plugin included. It also builds both example plugins (Rust and C) and checks that each exports the seven plugin functions. |
| Frontend | Lint, types, unit tests and the production build. |
| E2E (Playwright) | Every screen in a browser, with the fake backend. |
| Coverage | The coverage floors. |
| Dependency policy (cargo-deny) | Licenses, bans and sources, for every crate. |
| Tauri build (Ubuntu, Windows) | The full app builds, and a smoke test starts the real app. On Windows it also checks that the app has no console window. |

Other workflows: **CodeQL** (code scanning), **Security** (weekly check of known security problems in dependencies), **Conventional PR title**, **PR labels**, **Nix**, **Release Build** (on a version tag, see [Releasing](releasing.md)) and **Publish to WinGet**.

## Common build problems

| Problem | Fix |
| --- | --- |
| An error about `tablets/OpenTabletDriver/...` | Run `git submodule update --init`. |
| `LNK1104: cannot open file` on Windows | Your path is too long. Clone to a shorter path, for example `C:\dev\ntd`. |
| The tests in `engine::interop` or `engine::tablet_manager` fail | NextTabletDriver is running and holds the machine-wide names that the tests need. Close it. |
| `tauri build` asks for a signing key | Add `--config '{"bundle":{"createUpdaterArtifacts":false}}'`. |
| The pre-push hook says `cargo-nextest is required` | `cargo install --locked cargo-nextest` |

## Send your change

See [`CONTRIBUTING.md`](../.github/CONTRIBUTING.md) for the pull request rules.
