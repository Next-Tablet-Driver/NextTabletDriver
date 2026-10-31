# Architecture

This page explains how NextTabletDriver works inside. You do not need it to use the app. Read it if you want to understand the code, fix a bug, or add a feature.

## The big picture

A tablet sends small data packets over USB many times per second. NextTabletDriver reads each packet, turns it into a screen position, and tells the operating system to move the cursor.

```text
 Tablet
   │  USB (HID reports)
   ▼
 drivers      find the tablet, read packets, decode them        → TabletData
   │
   ▼
 engine::pipeline   map the pen to the screen
   │   Transformer   raw sensor units → millimeters
   │   Normalizer    millimeters → position inside the active area (0 to 1)
   │   pressure curve
   │   filters       plugins can change the position (smoothing, ...)
   │   Projector     position → screen pixels (absolute) or movement (relative)
   ▼
 engine::injector   tell the operating system
       Windows: SendInput (mouse)
       Linux:   uinput (virtual pen tablet + virtual mouse + virtual keyboard)
```

The desktop app is a thin shell around this engine. It adds a window (the user interface), a tray icon, settings, and updates.

## Parts of the repository

| Path | What it is |
| --- | --- |
| `/` (`Cargo.toml`) | The core library, `next_tablet_driver`. All the engine code lives here. |
| `apps/desktop/` | The desktop app. A [Tauri](https://tauri.app) program that starts the engine and shows the interface. |
| `frontend/` | The interface. [Svelte](https://svelte.dev) 5, TypeScript and Vite. See [`frontend/README.md`](../frontend/README.md). |
| `apps/sdk/` | `ntd_sdk`, a library that lets other programs (games, tools) run the engine inside their own process. See [Integrations](integrations.md) and [`apps/sdk/README.md`](../apps/sdk/README.md). |
| `crates/ntd-core/` | `ntd-core`, the pure types and maths (button bindings, coordinate transforms, `MappingConfig`). It does no input or output. |
| `crates/ntd-tablets/` | `ntd-tablets`, the tablet definitions, the report parsers and the packet types. It does no input or output. |
| `crates/ntd-plugin-api/` | `ntd_plugin_api`, the interface that plugins implement. See [Plugin development](plugin-development.md). |
| `plugins/` | Source code of the five plugins we maintain. |
| `tablets/OpenTabletDriver/` | A git submodule with tablet definitions from [OpenTabletDriver](https://github.com/OpenTabletDriver/OpenTabletDriver). |
| `qa/` | Benchmarks, fuzzers and property tests. |
| `packaging/`, `nix/`, `scripts/` | Packaging recipes, Nix modules, helper scripts, udev rules. |
| `.github/` | CI workflows, issue and pull request templates, repository rules. |

The repository is one Cargo workspace with a single `Cargo.lock`. The root crate, `apps/desktop`, `apps/sdk`, `crates/ntd-plugin-api`, each plugin and the example plugin are its members, so `cargo build --workspace` builds all of them into one `target/` folder (the checks of the root crate use `-p next_tablet_driver`). `tools/ntd-recorder` is a member too.

## Life of a pen packet

This is what happens for every packet, in order. The names are the Rust modules in `src/`.

1. **Find the tablet** (`drivers::detection`). The engine lists USB HID devices with the `hidapi` library. It compares each device's vendor ID and product ID (VID:PID) with the tablet definitions. When one matches, it opens the device and sends the start-up reports that some tablets need.
2. **Read a packet** (`engine::tablet_manager::polling`). A loop reads raw bytes from the tablet. This loop runs on a thread with raised priority (`TIME_CRITICAL` on Windows, `nice -11` on Linux).
3. **Decode it** (`ntd_tablets::generic`, `ntd_tablets::parsers`). Each tablet family sends bytes in its own layout. A *parser* knows one layout and returns a common `TabletData`: position, pressure, tilt, buttons and status. The definition file says which parser to use.
4. **Run the pipeline** (`engine::pipeline`). `Pipeline::process` turns `TabletData` into a `ProcessedFrame`:
    1. `Transformer` converts raw sensor units into millimeters on the tablet surface.
    2. `Normalizer` applies the active area (size, position, rotation) and returns `u` and `v`, a position from 0 to 1 inside that area.
    3. The pressure curve reshapes the pressure.
    4. `FilterPipeline` (`filters`) runs the enabled plugins on `u`, `v`, pressure and tilt.
    5. `Projector` converts the result. In absolute mode it maps `u` and `v` into the target area on screen, in pixels. In relative mode it turns the movement into pixel deltas, using the sensitivity setting.
    6. The tip threshold decides if the pen counts as touching.
5. **Inject** (`engine::injector`). The frame goes to the operating system. See [what each system receives](#what-each-system-receives).
6. **Publish**. The engine saves the latest state for the interface and the WebSocket server, updates statistics, and publishes a snapshot to shared memory for other processes (see [One owner per tablet](#one-owner-per-tablet)).

The math for steps 4.1, 4.2 and 4.5 lives in `ntd_core::math`. These functions are small and pure, so they are covered by property tests (`qa/tests`), benchmarks (`qa/benches`) and fuzzers (`qa/fuzz`).

### What each system receives

| | Windows | Linux |
| --- | --- | --- |
| How | `SendInput` | A virtual device made with `/dev/uinput` |
| Cursor position | Yes | Yes |
| Tip touch as a click | Left mouse button | `BTN_TOUCH` on the virtual pen, and `BTN_LEFT` on the virtual mouse for apps that ignore `BTN_TOUCH` |
| Pressure and tilt | **No.** Measured, but not sent to other apps. | Yes (`ABS_PRESSURE`, `ABS_TILT_X`, `ABS_TILT_Y`) |
| Pen buttons and tablet keys | Mouse buttons and keys, as the user bound them (see [Button mapping](button-mapping.md)) | The same, on the virtual mouse and the virtual keyboard |
| Eraser | Not sent yet | Not sent yet |
| Relative mode | Mouse movement | A virtual mouse |

On Linux the engine creates three devices, named `NextTabletDriver Virtual Pen`, `NextTabletDriver Virtual Mouse` and `NextTabletDriver Virtual Keyboard`. The kernel treats them as real hardware, so they work on X11, Wayland and XWayland.

The injector is called directly from the polling thread. This is a deliberate choice. A separate thread would need two different ways of passing data (positions can be dropped, button events cannot), and nobody has measured a problem. The reasoning is written at the top of `src/engine/injector/mod.rs`.

## Threads

| Thread | Started by | Job |
| --- | --- | --- |
| Engine | `ThreadSupervisor::spawn_engine` | Detect, read, decode, process and inject. Restarted after one second if it panics. |
| WebSocket | `spawn_websocket` | Serves live pen data to local clients. See [Integrations](integrations.md). |
| Saver | `spawn_saver` | Writes the settings file 500 ms after the last change, so fast edits cause one write. |
| Event bridge | `apps/desktop/src/lib.rs` | Sends pen data to the interface at about 60 Hz. It does nothing when every window is hidden, so the tray mode costs almost no CPU. |
| Tray watchers | `apps/desktop/src/lib.rs` | Two small loops that update the tray menu when the tablet or the mode changes. |
| Interface | Tauri | The webview that shows the Svelte app. |

The threads share one `SharedState` (`engine::state`). It holds the current settings (`ConfigState`), the latest pen data and statistics (`PipelineState`), the connected device (`DeviceState`), the app life cycle (`LifecycleState`) and the plugin manager. Each part is behind a lock or an atomic. The code uses helper methods that recover from a poisoned lock instead of panicking.

## How the interface talks to the engine

The interface runs in a webview and cannot touch the engine directly. It calls Tauri *commands* (`apps/desktop/src/commands.rs`): `get_config`, `set_config`, `save_config`, `load_profile`, `get_available_plugins`, `install_plugin`, `list_themes`, and so on. The TypeScript side of each command is in `frontend/src/lib/infrastructure/tauri/commands.ts`.

- `set_config` replaces the settings in memory and increases a version number. The engine notices the new version in its loop and reloads its pipeline. Changes apply right away.
- `save_config` sends the settings to the saver thread, which writes them to disk.
- The engine sends events back, such as `tablet-event` (pen data) and `device-changed`.

The webview is not trusted with file paths. File pickers (for plugins and themes) open in the Rust side, and the frontend only receives the result.

## One owner per tablet

Two programs cannot read the same tablet at the same time. NextTabletDriver solves this with a small protocol in `engine::interop`:

- A named lock, held by the OS for the whole machine, picks one **HID owner**. The operating system releases it if the owner crashes, so another process can take over at once.
- The owner publishes its state in a shared memory segment. Other processes (**readers**) copy it out without blocking the owner.
- A reader that wants to change a setting (mode or active area) sends a command to the owner over a local socket.

This is what lets a game that embeds the [SDK](integrations.md#sdk) run next to the desktop app without fighting for the device. Whoever starts first becomes the owner. In the desktop app the two roles are `tablet_manager::owner` and `tablet_manager::reader`.

## Settings and files

Settings are a `MappingConfig` (`ntd_core::config::models`), saved as JSON. Reading is forgiving: missing fields get defaults, and `validate_and_repair` fixes impossible values and reports what it changed. The folder layout is described in [Getting started](getting-started.md#where-your-files-are).

## Plugins

A plugin is a native library (`.dll` or `.so`) that exports a small C interface. The engine loads it, checks a handshake, reads its manifest, and calls `process` on every packet. A plugin only loads after the user approves its SHA-256 hash.

The engine builds its list of plugin instances **once, when it starts** (`filters::FilterPipeline::from_manager`). After that, only the enabled switch and the saved settings of those instances change (`FilterPipeline::update_config`). A plugin added later is listed in the interface, but it is only used after the engine restarts. The filters run on the pen position in absolute mode only: relative mode moves the cursor from the unfiltered movement.

See [Plugins](plugins.md) for users and [Plugin development](plugin-development.md) for authors.

## Platform differences in the code

Code that only makes sense on one system is behind `#[cfg(windows)]` or `#[cfg(target_os = "linux")]`:

- `engine::injector::windows` and `engine::injector::linux`
- `engine::interop::lock`, `shm` (one file per system)
- `drivers::hid_caps` (Windows only: asks the OS for the real report length)
- `startup`: autostart (registry on Windows, a desktop entry on Linux) and the high-resolution timer (Windows only)

The Cargo feature `desktop` (on by default) switches on HID polling and injection. The SDK turns it off and runs its own engine loop that never injects input.

## Safety rules in the code

- `unwrap`, `expect`, `panic!`, `todo!` and direct indexing are denied by Clippy in production code (`[lints.clippy]` in `Cargo.toml`). Use `?`, `get()`, and pattern matching.
- `unsafe` blocks need a `// SAFETY:` comment.
- The engine loop and every plugin call catch panics, so one bad packet or plugin cannot stop the whole app.
- Network input is limited: the WebSocket server only listens on `127.0.0.1`, accepts at most 10 clients, and checks the browser `Origin` header. Theme files are data only and have size limits.

## Parts that are not connected yet

These exist in the code but do not do anything in the current app:

- **Telemetry** (`application::telemetry`). It can send anonymous usage events and crash reports, but nothing starts it in the Tauri app. No data is sent.
- **Translations** (`i18n`). The Rust side can load `en.json` and `fr.json`, but the interface text is written in English and does not use them.
- **Several pen settings** are saved but not used by the engine. See [Known limitations](troubleshooting.md#known-limitations).

## Glossary

| Term | Meaning |
| --- | --- |
| HID | Human Interface Device. The standard way USB mice, keyboards and tablets send data. |
| Report | One packet of bytes that a HID device sends. |
| VID:PID | USB vendor ID and product ID. Together they identify a device model. |
| Parser | Code that decodes the report bytes of one tablet family. |
| Active area | The part of the tablet surface that is used, in millimeters. |
| Target area | The part of the desktop that the active area maps to, in pixels. |
| Absolute mode | A point on the tablet always maps to the same point on the screen. |
| Relative mode | The pen moves the cursor like a mouse. |
| Injector | The code that gives the result to the operating system. |
| uinput | A Linux kernel feature that lets a program create a virtual input device. |
| udev | The Linux service that manages device files and their permissions. |
| Plugin | A native library that changes pen data before it is mapped to the screen. Shown as a *filter* in the app. |
| HID owner | The one process that has the tablet open. Others read its published state. |
