# Repository rulesets

This folder is the source of truth for the GitHub rulesets of this repository. A ruleset is a set of rules that GitHub enforces on a branch or on tags.

| File | Protects | Rules |
| --- | --- | --- |
| `main.json` | The default branch, `main` | A pull request is required. Squash merge only. Linear history. The checks `CI success` and `Conventional PR title` must pass, on a branch that is up to date. Review conversations must be resolved. No force-push, no deletion, no bypass. |
| `release-tags.json` | Tags that start with `v` | Only repository admins can create, move or delete release tags. |

## Change a ruleset

Only admins can do this. Edit the JSON file, then send it to GitHub. Get the ruleset id from `gh api repos/{owner}/{repo}/rulesets`.

```bash
# create
gh api -X POST repos/Next-Tablet-Driver/NextTabletDriver/rulesets --input .github/rulesets/main.json

# update
gh api -X PUT repos/Next-Tablet-Driver/NextTabletDriver/rulesets/<id> --input .github/rulesets/main.json
```

## Required checks

A required check is named after a job:

- `CI success` is in `.github/workflows/ci.yml`.
- `Conventional PR title` is in `.github/workflows/pr-title.yml`.

You never need to touch the ruleset when you add a job to the CI. Add the job to the `needs` list of `ci-success` instead, and `CI success` will wait for it.

## Release tags

The tag rule protects the release process. A new tag that starts with `v` starts the `Release Build` workflow. Read [Releasing](../../docs/releasing.md) before you create one.
