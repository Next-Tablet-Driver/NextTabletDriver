<!--
The title of this pull request becomes the commit message on `main` (squash merge). It must follow
Conventional Commits: `type(scope): description`, in lowercase. Example: `fix(hid): handle a disconnected tablet`.
Types: feat, fix, perf, refactor, docs, test, build, ci, chore, style, revert. Add `!` for a breaking change.
-->

## What and why

<!-- What does this change, and why? Link the issue it closes: "Closes #123". -->

## How it was tested

<!-- The commands you ran, the tablets and systems you tried, and screenshots for changes to the interface. -->

## Checklist

- [ ] The title follows Conventional Commits.
- [ ] I added or updated tests for the behavior that changed.
- [ ] I added no `unwrap()`, `expect()`, `panic!` or indexing on a runtime path, and I handled locks safely.
- [ ] I updated the docs (README, `docs/`, or the interface) if a user or a developer can see the change.
- [ ] If this touches how installed copies update (tag format, release files, installer, update address or key), I read `docs/releasing.md`.
