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
