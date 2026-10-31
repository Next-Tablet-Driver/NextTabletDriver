<!--
The title of this pull request becomes the commit message on `main` (squash merge). It must follow
Conventional Commits: `type(scope): description`, in lowercase. Example: `fix(hid): handle a disconnected tablet`.
Types: feat, fix, perf, refactor, docs, test, build, ci, chore, style, revert. Add `!` for a breaking change.
Everything here is written in English. A pull request that is empty, mixes unrelated changes, or whose
author cannot explain it may be closed without a full review (see CONTRIBUTING.md).
-->

## What

<!-- What does this change? -->

## Why

<!-- Why is it needed? Link the issue it closes: "Closes #123". -->

## Impact

<!-- Users, latency, persisted formats, ABI, IPC contract, websocket or SDK. Write "none" if none. -->

## How it was tested

<!-- The commands you ran, the tablets and systems you tried, and screenshots for changes to the interface. -->

## Known limitations

<!-- What this does not do, or does not check. Write "none" if none. -->

## AI usage

<!-- None / partial (say what) / most. Do not remove this section: see CONTRIBUTING.md. -->

## Roadmap

<!-- Task ID from docs/v3-rewrite.md section 17, if any. -->

## Checklist

- [ ] The title follows Conventional Commits.
- [ ] I read the whole diff and can answer review questions about every line, whatever tool wrote it.
- [ ] I added or updated tests for the behavior that changed (golden files for a parser, a screenshot for a visible change).
- [ ] I added no `unwrap()`, `expect()`, `panic!` or indexing on a runtime path, and I handled locks safely.
- [ ] I updated the docs (README, `docs/`, or the interface) if a user or a developer can see the change.
- [ ] If this touches how installed copies update (tag format, release files, installer, update address or key), I read `docs/releasing.md`.
