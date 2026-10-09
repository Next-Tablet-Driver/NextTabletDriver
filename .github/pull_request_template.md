<!--
The PR title becomes the commit message on `main` (squash merge) and must follow
Conventional Commits: `type(scope): description`, lowercase, e.g. `fix(hid): handle a disconnected tablet`.
Types: feat, fix, perf, refactor, docs, test, build, ci, chore, style, revert. Add `!` for a breaking change.
-->

## What and why

<!-- What does this change, and why? Link the issue it closes: "Closes #123". -->

## How it was tested

<!-- Commands run, tablets/OS tried, screenshots for UI changes. -->

## Checklist

- [ ] The PR title follows Conventional Commits
- [ ] Tests added or updated for the changed behaviour
- [ ] No new `unwrap()`/`expect()` on a runtime path, locks handled safely
- [ ] User-facing changes are reflected in the UI/README where relevant
