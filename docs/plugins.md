# Plugins

A plugin changes the pen data before it reaches your screen. The most common plugins smooth the pen path, to remove the small shakes of a hand or a noisy sensor. In the app, plugins are called **filters**.

## Where plugins run

Each plugin sees every packet, after the position is mapped inside your active area and after the pressure curve, and before the position is turned into screen pixels:

```text
tablet packet → position in the active area → pressure curve → PLUGINS → screen position → system
```

A plugin can change the position (`u` and `v`, from 0 to 1 inside the active area), the pressure (0 to 1), and the tilt. It also receives the size of the active and target areas.

Plugins change the cursor position in **absolute mode** only. In relative mode the cursor moves from the unfiltered pen movement, so a smoothing plugin has no visible effect there. Pressure and tilt that a plugin changes reach other programs on Linux only.

## Use a plugin

Open the **Filters** tab.

- The list on the left shows the plugins that are loaded.
- Select a plugin to read its description. Tick **Enable Plugin** to turn it on, and change its settings below.
- The three buttons above the list **add** a plugin from a file, **open the plugins folder**, and **reload** the plugins from disk.

Good to know:

- Plugins start **turned off**. You must enable them.
- **A plugin takes effect after the engine restarts.** The engine reads its list of plugins when it starts. A plugin that you add, or approve, while the app runs shows up in the Filters tab, but does nothing until you choose **Reload Engine** in the tray menu (or restart the app).
- Enabled plugins run in alphabetical order of their names. You cannot reorder them.
- Your choices are saved in your settings and profiles, so each profile can use different filters.
- The installers do not include any plugin yet. Build the ones in this repository (see below), or get a plugin file from an author you trust.

### Add a plugin

Choose the add button, then pick the plugin file: `.dll` on Windows, `.so` on Linux. The app asks you to confirm, copies the file to the `plugins` folder, and loads it.

You can also copy the file into the `plugins` folder by hand. The folder is `Settings/plugins` (see [Where your files are](getting-started.md#where-your-files-are)), and the folder button on the Filters tab opens it. Then choose reload. A plugin you copied yourself needs one more step: see the next section.

### Remove a plugin

Select the plugin and choose **Delete this plugin**. This deletes the file from the `plugins` folder.

## How plugins are trusted

A plugin is a program. It runs inside NextTabletDriver with the same rights as the app. It can read your files and use the network. For this reason, the driver **never loads a plugin you did not approve**.

- When you add a plugin with the add button, you confirm a warning, and the app approves that exact file for you.
- A file you copy by hand is *not* loaded. A banner on the Filters tab lists it as waiting for approval. Choose **Review and trust** to approve it.
- The approval is for the file's **SHA-256 hash**, a fingerprint of the exact bytes. If the file changes, for example after a rebuild or an update, the old approval no longer applies and you must approve it again. Approved hashes are in `trusted_plugins.json`.
- The code also supports a second path: a plugin file with a matching `.minisig` signature from the project's official key. Current builds do not contain that key, so every plugin needs your approval.

Only install plugins from authors you trust.

## Plugins in this repository

The `plugins/` folder has the source of five plugins.

| Plugin (`id`) | Shown as | What it does |
| --- | --- | --- |
| `hawku_smoothing` | Hawku Smoothing | A classic low-pass smoothing filter. It reduces hand tremor with little delay. |
| `kalman_smoothing` | Kalman Smoothing | A two-axis Kalman filter. It reduces sensor noise with little delay. |
| `radial_follow` | Radial Follow Smoothing | Smooths movement inside an inner and an outer circle, with a soft transition between them. |
| `devocub_antichatter` | Devocub Antichatter | Removes sensor jitter and can predict the pen position, so the cursor still reacts fast. |
| `hand_speed` | Hand Speed & Stats | Measures your pen speed and the total distance, and sends them to a WebSocket server for stream overlays. |

`hand_speed` has its own WebSocket server, separate from the one in **Settings**. By default it listens on `127.0.0.1:50550`, and sends messages like this:

```json
{ "handspeed": 312.4, "total_distance": 18452.7, "timestamp": 1760112345678 }
```

`handspeed` is in the unit you choose (millimeters per second by default), `total_distance` is in millimeters, and `timestamp` is in milliseconds since 1970.

### Build the plugins from this repository

You need Rust (see [Development](development.md#what-you-need)).

**Windows.** The script builds every plugin and copies the files to your `plugins` folder:

```powershell
./scripts/build_plugins.ps1
```

Add `-Trust` to approve the files you just built (do this only for code you built yourself, because every rebuild changes the hash). Add `-OutDir <folder>` to copy them somewhere else.

**Any system.** Build one plugin, then copy its library to the `plugins` folder:

```bash
cargo build --release --manifest-path plugins/hawku_smoothing/Cargo.toml
```

The library is in `plugins/hawku_smoothing/target/release/`. It is named `hawku_smoothing.dll` on Windows and `libhawku_smoothing.so` on Linux.

## Write your own plugin

You can write a plugin in Rust, or in any language that can export C functions. The [Plugin development guide](plugin-development.md) explains how plugins work, how to test and debug them, and how to share them. It comes with two complete, tested examples:

- [`sdk/examples/rust_plugin`](../sdk/examples/rust_plugin): a smoothing filter in Rust. Copy it to start.
- [`sdk/examples/c_plugin`](../sdk/examples/c_plugin): a plugin in C.
