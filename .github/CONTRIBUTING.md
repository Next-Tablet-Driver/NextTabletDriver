# Contributing to NextTabletDriver

Thank you for helping! Bug reports, new tablets, fixes, features, themes, plugins and better docs are all welcome. This page shows how to send a change. Please also read the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to help

| You want to... | Start here |
| --- | --- |
| Report a bug | [Troubleshooting](../docs/troubleshooting.md#report-a-bug), then open an [issue](https://github.com/Next-Tablet-Driver/NextTabletDriver/issues/new/choose). |
| Get a new tablet working | [Adding a tablet](../docs/adding-a-tablet.md) |
| Fix a bug or build a feature | The steps below, and [Development](../docs/development.md). |
| Write a plugin | [Plugin development](../docs/plugin-development.md) |
| Make a theme | [Themes](../docs/themes.md#write-your-own-theme) |
| Improve the docs | [Docs index](../docs/README.md), which also explains how we write them. |
| Test on your hardware | Tell us in an issue or in [Discussions](https://github.com/Next-Tablet-Driver/NextTabletDriver/discussions) what works with your tablet and system. |

## Before you start

- Search the [issues](https://github.com/Next-Tablet-Driver/NextTabletDriver/issues) and [discussions](https://github.com/Next-Tablet-Driver/NextTabletDriver/discussions). Someone may already work on it.
- For a big change, open an issue first so we can agree on the idea before you spend time on code.
- Small fixes and docs changes do not need an issue.

## The steps

### 1. Set up

Fork the repository on GitHub, clone your fork **with submodules**, and create a branch:

```bash
git clone --recurse-submodules https://github.com/<your-name>/NextTabletDriver.git
cd NextTabletDriver
git checkout -b my-change
```

[Development](../docs/development.md) lists what you need to build and run the project.

### 2. Make your change

- Keep the change focused: one idea per pull request.
- Follow the [code style](../docs/development.md#code-style). In short: no `unwrap`, `expect`, `panic!` or indexing in production code, and a `// SAFETY:` comment on every `unsafe` block.
- Add or update tests for the behavior you changed.
- Update the docs if you change something a user or a developer can see: a setting, a file path, a command, a screen.
- If your change touches how installed copies update themselves (the tag format, release files, the installer, the update address or key), read [Releasing](../docs/releasing.md) first. A mistake there can leave every existing user without updates.

### 3. Run the checks

The CI runs the same checks on your pull request, so run them first. [Development](../docs/development.md#run-the-checks) has every command. The short list for the engine and the interface:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo nextest run --locked --workspace --all-features
cargo test --locked --all-features --doc -p next_tablet_driver -p ntd-core -p ntd-tablets
cargo xtask lint-style
cargo machete
typos
npm --prefix frontend ci
npm --prefix frontend run format:check
npm --prefix frontend run check
npm --prefix frontend run lint
npm --prefix frontend run test -- --run
```

Close NextTabletDriver before you run the Rust tests. A git hook can run most of these for you when you commit and push (see [Git hooks](../docs/development.md#git-hooks)).

### 4. Push and open a pull request

```bash
git push origin my-change
```

Open a pull request against `main` on GitHub and fill in the template: what the change does and why, its impact, how you tested it, what it does not do, whether AI wrote a significant part of it, and the issue it closes. Add screenshots for changes you can see.

## Pull request rules

- **Squash merge only.** The title of your pull request becomes the one commit on `main`, so it must follow [Conventional Commits](https://www.conventionalcommits.org). A check tests this automatically.

  `type(scope): description`, in lowercase. Example: `fix(websocket): handle bind error gracefully`.

  | Type | Use for |
  | --- | --- |
  | `feat` | A new feature. |
  | `fix` | A bug fix. |
  | `perf` | A speed improvement. |
  | `refactor` | A change that neither fixes a bug nor adds a feature. |
  | `docs` | Documentation only. |
  | `test` | Adding or fixing tests. |
  | `build`, `ci`, `chore`, `style`, `revert` | Tooling, pipelines, housekeeping, formatting, reverts. |

  Add `!` after the type or scope (`feat(api)!: ...`) for a breaking change.
- Labels are added automatically from the title and from the files you changed.
- The **CI success** check must be green, your branch must be up to date with `main`, and every review conversation must be resolved before the merge.
- Nobody can push directly to `main` or rewrite its history.

A maintainer reviews every pull request. Please answer comments, and push new commits to the same branch.

## Language

Everything is written in **English**: code, comments, commit messages, pull request titles and descriptions, issues you open to propose a change, and the documentation. Translations of the interface live in `resources/locales/`.

## AI-assisted contributions

You may use AI tools. The rules are the same as for any other help:

- **You are responsible for every line**, whatever tool wrote it. Before you open the pull request, read the whole diff, run the code, and be ready to answer review questions without asking a model.
- **Say so in the description.** The "AI usage" section says whether AI wrote a significant part of the code or of the description. Removing that statement from a description an agent drafted is a reason to close the pull request.
- **Commits carry no AI co-author trailer, session link or model name.**
- **An AI agent never opens the pull request itself**, even if you ask it to. It can prepare the branch, the commits and the description text; you review them and open the pull request. An agent never pushes to `main`.
- The instructions that agents follow are in [`AGENTS.md`](../AGENTS.md).
- A maintainer may close without review a pull request they consider generated and unchecked. A closed pull request is not reopened or recreated identically: talk about it first.

## Review and merge

- `main` is protected: nobody pushes to it directly, and the checks must pass (see above).
- The maintainer may merge their own pull requests once the **CI success** check is green and the checklist of the template is complete. There is no second reviewer while there is a single maintainer.
- A pull request from anyone else needs the maintainer's approval.

## When a pull request is closed without a full review

To keep reviews useful, a pull request may be closed without a full review if:

- the description is empty, only repeats the title, or does not say how the change was tested;
- it mixes unrelated changes: reformatting code you did not touch, a lockfile that changed for no reason, generated files without the change that justifies them;
- it changes a parser without golden files, or changes visible behavior without a screenshot;
- it duplicates an open pull request without saying how it differs;
- the author cannot explain the change;
- it hides or removes the AI usage statement;
- it is not written in English;
- the CI fails and the author does not fix it, or stops answering the review.

## Releases and versioning

Releases follow [Semantic Versioning](https://semver.org). The details, and the update rules you must not break, are in [Releasing](../docs/releasing.md).

## License

When you contribute, you license your work under the project's [MIT License](../LICENSE), except for the report parsers in `crates/ntd-tablets/src/parsers/`: they are ports of OpenTabletDriver's parsers and stay under the LGPL-3.0 (see [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md)). Tablet definitions belong in [OpenTabletDriver](https://github.com/OpenTabletDriver/OpenTabletDriver), which uses the LGPLv3. See [Adding a tablet](../docs/adding-a-tablet.md#share-the-definition).
