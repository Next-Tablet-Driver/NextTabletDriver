# Repository rulesets

Source of truth for the GitHub rulesets of this repository.

| File | Protects | Summary |
| --- | --- | --- |
| `main.json` | the default branch | PR required, squash merge only, linear history, `CI success` and `Conventional PR title` must pass on an up-to-date branch, threads resolved, no force-push, no deletion, no bypass |
| `release-tags.json` | `v*` tags | only repository admins can create, move or delete release tags |

Apply a change (admin only), using the ruleset id from `gh api repos/{owner}/{repo}/rulesets`:

```bash
gh api -X POST  repos/Next-Tablet-Driver/NextTabletDriver/rulesets --input .github/rulesets/main.json
gh api -X PUT   repos/Next-Tablet-Driver/NextTabletDriver/rulesets/<id> --input .github/rulesets/main.json
```

The required checks are the names of the jobs `CI success` (`.github/workflows/ci.yml`) and
`Conventional PR title` (`.github/workflows/pr-title.yml`). Adding jobs to the CI never requires
touching the ruleset: add them to the `needs` list of `ci-success`.
