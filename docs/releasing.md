# Releasing and versioning

How NextTabletDriver numbers and publishes releases, how an installed copy finds its updates, and the one historical exception that **must never be repeated**.

## Versioning

Since **2.0.0** the project follows [Semantic Versioning](https://semver.org): `MAJOR.MINOR.PATCH`, with an optional pre-release suffix (`2.1.0-beta.1`). A release tag is the version prefixed with `v` (`v2.0.0`) and must match the version in all of these places:

- `Cargo.toml`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `frontend/package.json`
- the `VERSION` constant in `src/lib.rs` (the app shows it)
- `.github/CITATION.cff`

The SDK (`sdk/Cargo.toml`) has its own version number, which changes only when the SDK changes.

The Tauri updater compares the `version` of `latest.json`, which comes from `tauri.conf.json`, not the tag. If the version is not bumped, `latest.json` advertises the version users already have and nobody sees an update. CI does not check this yet: verify it by hand before tagging.

### Legacy scheme (closed)

Releases up to and including `v1.26.1708.00` used `vMAJOR.YEAR.DDMM.PATCH` (`v1.26.1708.00` is 2026, 17 August, patch 0). That scheme is **closed**. No release may use it again, with the single exception of the [legacy bridge release](#the-legacy-bridge-release-one-time-exception).

## How an installed copy finds an update

| Installed version | Mechanism |
| --- | --- |
| 2.x (Tauri) | `tauri-plugin-updater`: reads `latest.json` from the endpoint in `tauri.conf.json`, verifies the minisign signature of the bundle against `plugins.updater.pubkey`, installs and relaunches. |
| up to `v1.26.1708.00` (egui) | A built-in updater that can no longer be changed: it is inside the binaries users already run. |

### Contract of the legacy updater

Read from the sources of `v1.26.0505.01`, `v1.26.0606.01`, `v1.26.2006.01` and `v1.26.1708.00`. The four earliest releases (`v1.26.0301.05`, `v1.26.0303.03`, `v1.26.1203.01`, `v1.26.1203.02`) were **not** audited.

1. It calls `GET /repos/Next-Tablet-Driver/NextTabletDriver/releases/latest`: the release GitHub marks **Latest**.
2. It parses `tag_name` as exactly four dot-separated numbers, `MAJOR.YEAR.DDMM.PATCH` (a leading `v` is allowed, `DDMM` must have four digits). Anything else is **silently ignored**: it logs "no new updates found or version format mismatch" and tells the user nothing. Versions are ordered by `(major, year, month, day, patch)`, so the month weighs more than the day.
3. On Windows it takes the **first** asset whose name ends in `.exe`. On Linux, the first `.AppImage`, then the first `.tar.gz`.
4. It requires an asset named `<that asset>.sha256` (mandatory since `v1.26.0606.01`, optional in `v1.26.0505.01`) whose first whitespace-separated token is the SHA-256 of the download, and aborts if it is missing or different.
5. On Windows it runs the downloaded file directly (`CreateProcess`, no elevation prompt) and exits. It passes no arguments up to `v1.26.2006.01`, and the Inno Setup switches `/SP- /SILENT /SUPPRESSMSGBOXES /NORESTART /CLOSEAPPLICATIONS /TASKS=desktopicon` in `v1.26.1708.00`. On Linux it marks the AppImage executable and starts it from the `updates/` folder of the settings directory; it does not replace the installed copy.
6. Its update dialog shows the release `body` (markdown) as the changelog (checked on `v1.26.1708.00`).

## Why 2.0.0 cannot simply be published

Moving to SemVer and Tauri breaks that contract in several independent ways at once. Fixing one is not enough:

- `v2.0.0` is not four numbers (point 2). Every installed copy would stay silent, with no error, and there is no other channel to tell users.
- The release pipeline publishes one `SHA256SUMS.txt` and `.sig` files, not a `.sha256` per asset (point 4).
- A release that holds both x64 and ARM64 installers makes "the first `.exe`" a coin toss (point 3).
- The Tauri NSIS installer in `perMachine` mode carries an administrator manifest (`RequestExecutionLevel admin`). Windows refuses to start such a file from a non-elevated process (`ERROR_ELEVATION_REQUIRED`), which is how the legacy updater starts it (point 5). `currentUser` mode does not have this problem.
- The Inno Setup switches mean nothing to NSIS (point 5).

Roughly 300 users were active when this was written. They would have had to notice on their own that a new version exists and download it again.

## The legacy bridge release (one-time exception)

To reach the copies that still run the legacy updater, the 2.0.0 migration publishes **two** releases for the same build:

| | Canonical release | Bridge release |
| --- | --- | --- |
| Tag | `v2.0.0` (SemVer) | legacy format, above `v1.26.1708.00`, for example `v1.26.1010.00` |
| For | Tauri installs, new users, WinGet, documentation | legacy installs only |
| Assets | everything: all platforms, `latest.json`, signatures, `SHA256SUMS.txt` | exactly one x64 `.exe` + its `.sha256`, exactly one x86_64 `.AppImage` + its `.sha256` |
| Marked Latest | no | yes |

The tag is only a label for the legacy updater. Tauri reads the version from `latest.json`, so the two numbering schemes do not conflict.

All of the following are required, otherwise the bridge does nothing and says nothing:

- The tag matches `MAJOR.YEAR.DDMM.PATCH` and sorts strictly above every legacy tag in the field (year, then month, then day).
- Its assets contain a single `.exe` and a single `.AppImage`, both for x64/x86_64. ARM64 builds only live in the canonical release.
- Each of them has a `<name>.sha256` next to it.
- The Windows installer can be started without elevation (an install mode other than `perMachine`, or a launcher that elevates itself) and does not fail when given the Inno switches above.
- The Tauri updater endpoint does not depend on Latest, otherwise Tauri installs would read the bridge's `latest.json`. It must be changed in the build that the bridge ships, because the endpoint is compiled into the installed app.
- The `body` is the announcement. It is what legacy users read in their update dialog: say what changed, that their profiles are kept and where to get help.
- The WinGet workflow ignores it: no WinGet version `1.26.…` is ever published for a 2.x build.
- It contains the same signed build as `v2.0.0`, never a different one.

Validate before publishing, with a dry run of `release.yml`: install the latest legacy build (and the oldest one still in use) on a clean Windows VM, point it at the bridge, and check that it updates, keeps its profiles and settings, and still starts at login if it did before (the legacy versions registered a Startup-folder shortcut, 2.x writes a `Run` registry entry).

Retire it when you have evidence that few legacy installs remain: stop marking it Latest, or delete it. Keep this section as history. The 2.x app does not send telemetry, so use other signals, such as the download counts of the bridge assets and the questions users ask.

> **Status (2026-10-10):** the bridge has not been published yet. This section lists what it must satisfy. Remove this note when it ships.

## This must never happen again

The 2.0.0 migration forces a legacy tag on a modern release, a second release and months of dual support. The cause is that the legacy updater's contract was never written down and the update path was never treated as an interface that has to stay stable. It is a **one-time exception**. The bridge tag is **not a precedent**: no other release may ever use the legacy format.

Rules:

1. **The update chain is frozen by every version already shipped.** A release is only valid if the oldest supported version can still find it, verify it, install it and keep its data. Updating is the only channel to users, so breaking it strands them.
2. **Never change any of the following without a tested compatibility bridge, shipped first:** tag format, asset names, checksum scheme, `latest.json` schema, updater endpoint, updater signing key, installer technology and its command-line switches, install scope (per-user or per-machine) and elevation level, install folder, executable name, location and layout of the data folder.
3. **The updater signing key is forever.** Losing the private key, or replacing `plugins.updater.pubkey` in `tauri.conf.json`, strands every installed copy exactly as this migration would have, and no bridge can fix it. Keep an offline backup of the private key and its password.
4. **An updater must never fail silently.** The legacy updater dropped a version it could not parse and told nobody; that single choice is what made this migration invisible. Anything the updater cannot interpret must be logged and shown to the user ("a newer version exists, download it from …").
5. **Test the update path from the oldest supported version on a clean machine before tagging**, with the dry run of `release.yml` (`workflow_dispatch`). Keep an installer of every supported version for that purpose.
6. **Announce breaking changes where users will see them.** The `body` of the Latest release is displayed by the legacy copies; keep it readable.

Enforcement today is code review only (`CODEOWNERS`). Nothing in CI checks these rules yet: checking that the tag matches the manifests, the shape of the release assets and the update test are open items. A pull request that touches `release.yml`, `winget-release.yml`, `tauri.conf.json` (`bundle`, `plugins.updater`), `src-tauri/windows/` or the update code is touching this document and must say so.
