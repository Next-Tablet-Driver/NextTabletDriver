# Plugin development guide

A plugin is a small dynamic library that changes pen data before NextTabletDriver turns it into a cursor position. A typical plugin smooths the pen path. This guide shows how to write, test, debug and share one.

If you only want to *use* plugins, read [Plugins](plugins.md) instead.

| | |
| --- | --- |
| Languages | Rust (easiest, a macro does the hard part), or any language that can export C functions. |
| What you build | A `.dll` on Windows, a `.so` on Linux. |
| Interface version | 1 |
| Where it runs | Inside NextTabletDriver, for every packet, on the thread that reads the tablet. |
| Start from | [`apps/sdk/examples/rust_plugin`](../apps/sdk/examples/rust_plugin) (Rust) or [`apps/sdk/examples/c_plugin`](../apps/sdk/examples/c_plugin) (C). Both are tested. |

## What a plugin can and cannot do

A plugin can:

- change the **position** of the pen (`u` and `v`),
- change the **pressure** and the **tilt**,
- keep its own state between packets (to smooth, predict or measure),
- show **settings** in the app: numbers, switches, lists and text,
- read the size of the active area and of the display area.

A plugin cannot:

- change whether the pen tip counts as touching (the click). The `is_down` field is only there to read.
- change how the cursor moves in **relative mode**. In relative mode the engine moves the cursor from the unfiltered pen movement, so a change to `u` and `v` is ignored. Plugins that change the position only work in absolute mode.
- see raw sensor values, the tablet model or the USB ID.
- draw its own screen, write to the Console tab, or call back into the app. The app has no logging function for plugins.
- run when the tablet is not sending packets.

Pressure and tilt that a plugin changes only reach other programs on Linux. On Windows the app sends position and a click only (see [Architecture](architecture.md#what-each-system-receives)).

## Quick start with Rust

You need Rust 1.99 or newer. `ntd_plugin_api` requires it.

1. **Copy the template.** Copy the folder [`apps/sdk/examples/rust_plugin`](../apps/sdk/examples/rust_plugin) to a new place.
2. **Name it.** In `Cargo.toml`, change `name`. In `src/lib.rs`, change the manifest: a new `id`, `name`, `author` and `description`. If your folder is outside this repository, replace the dependency on `ntd_plugin_api` with a git dependency, as the comment in `Cargo.toml` shows, and pin a commit.
3. **Test it.** Run `cargo test`. The template has tests that you can reuse.
4. **Build it.** Run `cargo build --release`. The library is in `target/release/`: `simple_smoothing.dll` on Windows, `libsimple_smoothing.so` on Linux.
5. **Try it in the app.** See the next section.

## Try it in the app

The app is slower to test with than a unit test, so do most of your work with `cargo test`. When you want to see the real thing, follow these steps. Order matters, for the reasons explained in [the life of a plugin](#the-life-of-a-plugin).

1. **Quit NextTabletDriver** (choose **Quit** in the tray menu). On Windows, a loaded `.dll` cannot be replaced, so you cannot copy your new build over the old one while the app runs.
2. **Copy** your library into the `plugins` folder (see [Where your files are](getting-started.md#where-your-files-are)).
3. **Start** the app. Open the **Filters** tab. Your plugin is listed as *waiting for approval*, because its file is new. Choose **Review and trust**. Every new build is a new file, so you approve every build again.
4. **Choose Reload Engine** in the tray menu. The engine only builds its list of plugins when it starts, so a plugin that was approved after the start does nothing until the engine restarts.
5. **Select** your plugin on the Filters tab and tick **Enable Plugin**. Use absolute mode.
6. **Move the pen** and change the settings. Watch the **Console** tab for messages from `Plugins`. A successful load looks like this:

   ```text
   Loaded plugin 'Simple Smoothing' v1.0.0 by You from C:\...\plugins\simple_smoothing.dll
   ```

Instead of steps 2 and 3, you can start the app and use the **Add Plugin** button on the Filters tab. It copies the file and approves it in one step. You still need step 4. The app must not be running with the old build loaded when you replace the file, so quit it first when you rebuild.

If something does not work, see [Debugging](#debugging-and-common-errors).

## The life of a plugin

| When | What happens |
| --- | --- |
| The app starts | It scans the `plugins` folder (not its subfolders) for `.dll` (Windows) or `.so` (Linux) files. A file that you have not approved is skipped. |
| A file is loaded | The operating system loads the library, and **any start-up code in it runs** (`DllMain`, static constructors). Then the engine calls `ntd_plugin_handshake`, then `ntd_plugin_manifest`, and looks up the other functions. Any failure is logged and the file is skipped. |
| The engine starts | For each loaded plugin, it creates **one instance** with `ntd_plugin_create` and applies the **saved** settings with `ntd_plugin_set_property`. Plugins are sorted by name, and run in that order. The tray command **Reload Engine** repeats this step: the old instances are destroyed and new ones are created. |
| Every packet | If the plugin is enabled, the engine calls `ntd_plugin_process` with the packet. A disabled plugin is not called. |
| The pen leaves range, or the tablet is not connected | The engine calls `ntd_plugin_reset` on **every** plugin (enabled or not), once for each packet that says so. It can be called many times in a row. |
| The user changes any setting in the app | Within about 50 ms the engine sends **all saved settings of all plugins** again with `ntd_plugin_set_property`, even the ones that did not change, and even if the change was not about plugins (for example moving the active area). It also updates the enabled switch. |
| The engine stops | `ntd_plugin_destroy` is called for each instance. |

What follows from this:

- **`set_property` is called often, with the same value.** Make it cheap, and do not reset your state when the value did not change.
- **`reset` is called often.** Make it cheap and safe to call twice.
- **Settings the user never touched are never sent.** Your plugin starts with its own defaults, so they must match the defaults in your manifest.
- **A plugin that is not enabled still receives `set_property` and `reset`.**
- **New files do not take effect until the engine restarts**, and a library that is already loaded is not replaced by *Reload* in the Filters tab. After you replace a file, restart the app.
- The engine calls one instance from one thread at a time. The calls do not always come from the same thread over the whole life of the instance, so do not rely on thread-local state.
- Keep the state in the instance, not in global variables. Then every new instance starts clean, for example after **Reload Engine**.

## The manifest

The manifest describes your plugin. In Rust you write it in the `manifest()` method. In other languages you return it as JSON text (see [the C interface](#writing-a-plugin-in-another-language)).

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | text | Yes | A unique name for your plugin, for example `simple_smoothing`. Use lowercase letters, digits and `_`. The user's settings are saved under it, so **never change it** after you publish. If two files have the same `id`, only one is loaded. |
| `name` | text | Yes | The name the user sees. Plugins run in the alphabetical order of this name. |
| `version` | text | Yes | Your version, such as `1.2.0`. It is shown as a badge. The engine does not read it. |
| `author` | text | Yes | Shown as "By ...". |
| `description` | text | Yes | A sentence or two. |
| `icon` | text or null | No | Not used by the app yet. Use `None`/`null`. |
| `properties` | list | Yes | The settings. The list can be empty. |

A setting (`PropertyDescriptor`) has an `id` (unique inside your plugin, and also never to be changed), a `name` for the label, an optional `tooltip` that shows when the user hovers, and a `kind`.

### Setting kinds

The app builds the control from the `kind`. The value reaches `set_property` as JSON text.

| Kind | Control in the app | `value_json` | Notes |
| --- | --- | --- | --- |
| `Float { min, max, step, unit, default }` | Number box | `0.5` | `min`, `max` and `step` are hints for the box. The user can still type a value outside the range, so **always clamp**. |
| `Int { min, max, step, unit, default }` | Number box | `3` | The box can send `3.5`. Read a number, then round it and clamp it. |
| `Bool { default }` | Checkbox | `true` or `false` | |
| `Choice { options, default_index }` | Drop-down list | `1` | The number of the chosen option, counting from 0. |
| `String { default }` | Text box | `"hello"` | JSON text, so the value has quotes. |

The value can also be `null`. The app sends `null` when a number box is empty. **Ignore any value that you cannot read.** If you cannot read a value, do nothing.

The user's choices are stored in the profile like this, under your `id`:

```json
"plugins": {
  "simple_smoothing": { "enabled": true, "properties": { "strength": 0.7 } }
}
```

## The packet

`process` receives a `PluginPacket` that it may change, and a `PluginContext` that is read-only.

| Field | Type | Meaning |
| --- | --- | --- |
| `u`, `v` | `f32` | The position of the pen inside the **active area**. `0.0, 0.0` is the top-left corner and `1.0, 1.0` the bottom-right corner. The active area's rotation is already applied. The values can be below 0 or above 1 when the pen is outside the active area. The engine clamps the final screen position, so you do not have to. |
| `pressure` | `f32` | Pressure after the pressure curve, from 0.0 to 1.0. |
| `tilt_x`, `tilt_y` | `i32` | Tilt in degrees. |
| `is_down` | `bool` | The tip is touching. Read it, for example to reset your state on a new stroke. Changing it has no effect. |
| `timestamp_ns` | `u64` | Reserved. Always 0 for now. If you need time, read your own clock. |

The context has `active_area_width_mm`, `active_area_height_mm`, `target_area_width_px` and `target_area_height_px`. They let you work in real units:

```rust
let x_mm = packet.u * context.active_area_width_mm;     // millimeters on the tablet
let x_px = packet.u * context.target_area_width_px;     // pixels on the screen
```

Keep `u` and `v` finite numbers. Never output `NaN` or infinity.

## Rules for `process`

`process` runs for every packet, up to about a thousand times per second, on the thread that reads the tablet. Anything slow here is slow for every packet.

- **No memory allocation.** No `Vec` growth, `String`, `Box` or `format!`. Allocate in `Default::default()` or in `set_property`.
- **No waiting.** No locks that can block, no file or network access, no sleeping.
- **Keep it short.** A few arithmetic operations are ideal.
- **No panics.** In Rust, the macro catches a panic, so the app does not crash, but the packet may be left half changed. Do not set `panic = "abort"` in your `Cargo.toml`. With it, a panic would stop the whole app. In C++ and other languages, **never let an exception or a panic cross the boundary**.
- Do heavy work (a server, a log file, a big calculation) on a **thread of your own**, and exchange data through a channel. The `hand_speed` plugin does this.

Here is how much time our plugins use, to give you a sense of scale. Release builds, measured through the plugin interface on an AMD Ryzen 5 5500, 20 million packets with changing positions:

| Plugin | Time per packet |
| --- | --- |
| Flip X (C example) | 3 ns |
| Simple Smoothing (Rust example) | 5 ns |
| Kalman Smoothing | 9 ns |
| Hawku Smoothing | 24 ns |
| Devocub Antichatter | 41 ns |
| Radial Follow Smoothing | 125 ns |

A tablet that reports 1000 times per second sends a packet every 1,000,000 ns, so even the slowest plugin above uses a tiny part of that time. Still, the engine itself needs a few hundred nanoseconds per packet, and the time of every plugin is added to it. We suggest staying under 1 µs, and measuring if you are unsure.

## Testing

A plugin is plain Rust code, so test the trait methods directly with `cargo test`. You do not need the app. The template has tests for the points below. Reuse them.

- The first packet passes through unchanged (or as you expect).
- A known input gives a known output. Use numbers you can calculate by hand.
- `reset()` forgets the state, and calling it twice is harmless.
- `set_property` clamps values, and ignores `null`, text where a number is expected, invalid JSON, and unknown keys.
- The defaults in `Default` match the defaults in the manifest.
- The output is always finite, for strange inputs too (for example `u = -5.0` or `u = 1e9`). [`proptest`](https://crates.io/crates/proptest) is good for this.

To check speed, write a [Criterion](https://bheisler.github.io/criterion.rs/book/) benchmark that calls `process` in a loop on realistic packets, as in [`qa/benches`](../qa/benches/README.md).

Test the real loading at least once with the steps in [Try it in the app](#try-it-in-the-app). A unit test cannot find a missing export or a wrong interface version.

## Debugging and common errors

Open the **Console** tab and look for messages from `Plugins` and `Pipeline`. The most common ones:

| Message | Meaning and fix |
| --- | --- |
| `Skipping untrusted plugin library ... approve it in the Filters tab to load it` | The user has not approved this exact file. Choose **Review and trust**. A new build is a new file. |
| `Failed to load potential plugin library at ...: Dynamic library load error: ...` | The operating system could not load the file. Common reasons: the file is built for another CPU type (x64 or ARM64), it needs another library that is not found, or (Linux) it needs a newer C library than the system has. The text after the colon is the system's message. |
| `Missing required plugin FFI symbol: 'ntd_plugin_...'` | One of the seven functions is not exported. In C++, use `extern "C"`. In C on Windows, use `__declspec(dllexport)`. Check the exports with `dumpbin /exports file.dll` (Windows) or `nm -D --defined-only file.so` (Linux). |
| `Invalid plugin magic identifier: expected 0x4E540000, found ...` | `ntd_plugin_handshake` returns the wrong value. See [the handshake](#the-handshake). |
| `Incompatible plugin ABI version: expected 1, found N` | The library was built for another interface version. Rebuild it with a matching version of the API. |
| `Failed to parse plugin manifest: ...` | The manifest is not valid JSON, or it misses a required field. The text shows the position of the problem. |
| `ntd_plugin_manifest returned a null pointer` | Return a pointer to a string. |
| `Failed to create instance of plugin '...'` | `ntd_plugin_create` returned null. |
| `Duplicate plugin ID '...' discovered at ...; skipping in favor of earlier match` | Two files use the same `id`. Remove one, or change the `id`. |
| `Failed to apply saved property '...' on plugin '...'` | `set_property` returned an error status. Return OK even for values that you ignore. |

Other problems:

- **The plugin is in the list but does nothing.** Check that it is enabled, that you are in **absolute mode**, that the plugin changes `u` and `v` in a way you can see, and that you chose **Reload Engine** after you added it.
- **The new build does not change anything.** The old library may still be loaded. Quit the app, replace the file, and start the app again.
- **You cannot copy the file.** On Windows the app has the old `.dll` open. Quit the app first.

## Writing a plugin in another language

The macro in `ntd_plugin_api` is only a convenience. The real interface is seven C functions. Any language that can export functions with the C calling convention can make a plugin: C, C++, Zig, Go (with `cgo`'s `c-shared` mode), Pascal, and more. The tested C example is [`apps/sdk/examples/c_plugin`](../apps/sdk/examples/c_plugin).

Build for the same CPU type as the app (x64 or ARM64) and the same operating system.

### The seven functions

All names must be exported exactly like this, with no decoration.

```c
uint32_t        ntd_plugin_handshake(void);
const char     *ntd_plugin_manifest(void);
void           *ntd_plugin_create(void);
NtdPluginStatus ntd_plugin_process(void *instance, NtdPluginPacket *packet,
                                   const NtdPluginContext *context);
NtdPluginStatus ntd_plugin_set_property(void *instance, const char *key,
                                        const char *value_json);
NtdPluginStatus ntd_plugin_reset(void *instance);
NtdPluginStatus ntd_plugin_destroy(void *instance);
```

| Function | Called | Rules |
| --- | --- | --- |
| `ntd_plugin_handshake` | Once, when the file is loaded. | Return the [handshake value](#the-handshake). |
| `ntd_plugin_manifest` | Once, when the file is loaded, before any instance exists. | Return a null-terminated UTF-8 JSON string. Make it a constant that stays valid, and never `NULL`. The engine reads it at once. It can be called on any thread. |
| `ntd_plugin_create` | When the engine starts. | Allocate and return the instance. Its settings must start at the manifest defaults. Return `NULL` on failure. |
| `ntd_plugin_process` | For every packet, when enabled. | Change `*packet`. The return value is ignored. Never block or allocate. |
| `ntd_plugin_set_property` | Often. | `key` and `value_json` are null-terminated strings that are only valid during the call. Copy what you need to keep. Return `OK` even when you ignore the value. |
| `ntd_plugin_reset` | Often. | Forget your running state. |
| `ntd_plugin_destroy` | When the engine stops. | Free the instance. |

The engine ignores the return values of `process`, `reset` and `destroy`. For `set_property`, an error status only causes a warning in the Console.

### The handshake

The value is the "magic" `0x4E540000` in the upper 16 bits, and the interface version in the lower 16 bits. With version 1 it is **`0x4E540001`**. The engine refuses a file when the upper part or the version is different.

```c
return (0x4E540000u & 0xFFFF0000u) | (1u & 0x0000FFFFu);
```

### The structures

All structures use the C layout. These offsets and sizes are checked against the engine.

`NtdPluginPacket`, 32 bytes, 8-byte alignment:

| Field | C type | Offset |
| --- | --- | --- |
| `u` | `float` | 0 |
| `v` | `float` | 4 |
| `pressure` | `float` | 8 |
| `tilt_x` | `int32_t` | 12 |
| `tilt_y` | `int32_t` | 16 |
| `is_down` | `bool` (1 byte) | 20 |
| (padding) | | 21 to 23 |
| `timestamp_ns` | `uint64_t` | 24 |

`NtdPluginContext`, 16 bytes: four `float` fields at offsets 0, 4, 8 and 12: `active_area_width_mm`, `active_area_height_mm`, `target_area_width_px`, `target_area_height_px`.

`NtdPluginStatus`: a 4-byte `enum`. `NTD_PLUGIN_OK = 0`, `NTD_PLUGIN_ERROR = 1`.

### The manifest as JSON

```json
{
  "id": "c_flip_x",
  "name": "Flip X (C)",
  "version": "1.0.0",
  "author": "You",
  "description": "Mirrors the horizontal axis. Written in C.",
  "icon": null,
  "properties": [
    {
      "id": "flip",
      "name": "Flip horizontally",
      "tooltip": "Turn the horizontal axis around.",
      "kind": { "type": "Bool", "config": { "default": true } }
    }
  ]
}
```

The `kind` of a setting is an object with a `type` and a `config`:

```json
{ "type": "Float",  "config": { "min": 0.0, "max": 1.0, "step": 0.01, "unit": "", "default": 0.5 } }
{ "type": "Int",    "config": { "min": 1, "max": 10, "step": 1, "unit": "px", "default": 3 } }
{ "type": "Bool",   "config": { "default": true } }
{ "type": "Choice", "config": { "options": ["Fast", "Smooth"], "default_index": 0 } }
{ "type": "String", "config": { "default": "text" } }
```

### Build the C example

```bash
# Windows, in a Visual Studio developer prompt
cl /O2 /LD plugin.c /Fe:c_flip_x.dll

# Linux
cc -O2 -shared -fPIC plugin.c -o libc_flip_x.so
```

See the [README of the example](../apps/sdk/examples/c_plugin/README.md).

## Share your plugin

- Build one file for each system and CPU type you support (Windows x64, Windows ARM64, Linux x86_64, Linux ARM64), and say which is which. On Linux, build on the oldest system you want to support. A plugin built on a very new system may need a C library that older systems do not have.
- Tell users which NextTabletDriver version you tested with.
- **Publish the SHA-256 hash** of each file next to the download, so users can check that they have the file you made. The app asks users to approve a plugin by its hash.
- Publishing the source code makes it easier for people to trust you.
- Keep `id` and the setting ids stable between versions. A user's saved settings belong to them. Raise `version` in the manifest for each release.
- Be honest about what the plugin does. A plugin runs with the rights of the app. It should not read files, use the network or start other programs unless that is its purpose, and then it should say so in the description.
- Keep start-up work small. Your library's initialization code runs as soon as the file loads, before the user enables anything.
- Plugins with a signature from the project's official key load without approval. That key belongs to the project, so third-party plugins are approved by users.

## Reference

The Rust API is documented in the code of the [`ntd_plugin_api`](../crates/ntd-plugin-api/src/lib.rs) crate. To read it as web pages:

```bash
cargo doc --manifest-path crates/ntd-plugin-api/Cargo.toml --open
```

| Item | What it is |
| --- | --- |
| `NextTabletPlugin` | The trait to implement: `manifest`, `process`, `set_property`, `reset`. It requires `Send + Sync + 'static`, so the plugin cannot hold types such as `Rc` or `RefCell`. |
| `export_ntd_plugin!(Type)` | Writes the seven exported functions. Call it once, at the top level of the crate, with the name of a type that implements `NextTabletPlugin` and `Default`. |
| `PluginManifest`, `PropertyDescriptor`, `PropertyKind` | The description of the plugin and its settings. |
| `PluginPacket`, `PluginContext`, `PluginStatus` | The data that crosses the C interface. |
| `NTD_PLUGIN_MAGIC`, `NTD_PLUGIN_ABI_VERSION` | The handshake constants. |

The macro builds the manifest by creating a temporary instance with `Default::default()` and calling `manifest()` on it. Both must be cheap and must not panic.

## About the interface version

The interface version is 1. The engine accepts exactly this version and refuses all others. If the structures or the functions ever change in an incompatible way, this number changes, and plugins have to be rebuilt.

The plugin API crate has its own crate version (`0.1.0`), which is not the interface version. It is also not published on crates.io. Depend on it with a git dependency, and pin a commit.

## Questions

**Can a plugin work in the SDK?** The [SDK](integrations.md#sdk) loads approved plugins too, but it starts with default settings and cannot enable them, so they do nothing there.

**Can a plugin add its own screen?** No. The app builds the settings screen from the manifest.

**Can a plugin change the click?** No. The click depends on the raw pressure and the tip threshold, before plugins run.

**Can several plugins work together?** Yes. They run one after the other in alphabetical order of their names. Each one sees the result of the one before.

**Does it work on macOS?** NextTabletDriver supports Windows and Linux only.
