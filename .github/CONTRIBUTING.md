# Contributing to NextTabletDriver

Thank you for your interest in contributing to **NextTabletDriver**! We welcome bug fixes, documentation improvements, new tablet profiles, and feature contributions.

To keep the driver fast, clean, and stable, please follow the steps below.

---

## Quick Start Contribution Loop

### 1. Set Up the Repository
Fork the repository on GitHub, clone it locally, and create a branch for your work:
```bash
git clone https://github.com/Next-Tablet-Driver/NextTabletDriver.git
cd NextTabletDriver
git checkout -b my-contribution-branch
```

### 2. Make Your Changes
Write your code, adding documentation and tests where necessary. Ensure your changes follow our coding standards (no `unwrap`, safe lock handling, etc.).

### 3. Run the Checks
Before pushing, make sure your changes pass the same checks as the CI:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-features   # cargo install --locked cargo-nextest
cargo test --workspace --all-features --doc
npm --prefix frontend ci
npm --prefix frontend run lint
npm --prefix frontend run check
npm --prefix frontend run test -- --run
```

End-to-end tests (optional locally, required in CI):
```bash
npx --prefix frontend playwright install chromium   # once
npm --prefix frontend run e2e                       # every screen, against the Tauri IPC mock
```
The Playwright suite (`frontend/e2e/`) runs the real Svelte app in a browser with the mock of `frontend/src/dev/tauri-mock.ts` (`/?mock` on the dev server), so it needs neither a tablet nor the native window. Each test also fails on any `console.error` or uncaught exception. When a test fails, run `npx playwright show-trace` on the file under `frontend/test-results/`; in CI the report and the traces are the `playwright-report` artifact. A smoke test of the real app runs in the `Tauri build` job: through `tauri-driver` on Linux (`npm --prefix frontend run e2e:native`, needs `tauri-driver` and a built app, see `.github/workflows/ci.yml`; it also works on Windows with `msedgedriver` and `NTD_SMOKE_LOCAL=1`), and with `scripts/smoke-windows.ps1` on the Windows runners.

> **Why nextest rather than `cargo test`?** A few Rust tests touch state that belongs to the whole process: the panic hook installed by `setup_panic_hook`, the `log` level, and the telemetry sender. `cargo nextest` runs every test in its own process, so they cannot affect each other; under `cargo test` (threads in one process) they may interfere. Tests that depend on this say so in a comment. Tests whose only purpose is to check that a call does not panic are named `smoke_*`.

### 4. Commit and Push
Push your branch to your fork:
```bash
git push origin my-contribution-branch
```

### 5. Open a Pull Request
Go to the original repository on GitHub, and open a Pull Request against `main`. Provide a clear explanation of what your change does, what testing you performed, and reference any issues resolved.

Pull requests are **squash-merged only**: the PR title becomes the single commit on `main`, so it must follow **Conventional Commits** (checked automatically):

`type(scope): description`, lowercase, e.g. `fix(websocket): handle bind error gracefully`.

| Type | Use for |
| --- | --- |
| `feat` | a new feature |
| `fix` | a bug fix |
| `perf` | a performance improvement |
| `refactor` | a change that neither fixes a bug nor adds a feature |
| `docs` | documentation only |
| `test` | adding or fixing tests |
| `build`, `ci`, `chore`, `style`, `revert` | tooling, pipelines, housekeeping, formatting, reverts |

Add `!` after the type or scope (`feat(api)!: ...`) for a breaking change. Labels are applied automatically from the title and from the files you changed.

A PR can only be merged when the **CI success** check is green and your branch is up to date with `main`.
