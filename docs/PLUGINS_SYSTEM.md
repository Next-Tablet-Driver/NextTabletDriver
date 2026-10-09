# Plugin System

Third-party developers can add filters (smoothing, anti-chatter, tracking, ...) to
NextTabletDriver without touching the core: build a native library, drop it in the plugins
folder, and the interface generates its settings automatically.

---

## 1. Overview

- A plugin is a **dynamic library** (`.dll` on Windows, `.so` on Linux) built as a Rust
  `cdylib` against the [`ntd_plugin_api`](../sdk/plugin_api) crate, which defines the C ABI
  and the `export_ntd_plugin!` macro.
- The driver loads trusted libraries at startup (and on **Reload**) with `libloading`
  ([`src/engine/plugins/`](../src/engine/plugins)).
- Each loaded plugin is a stage of the filter pipeline. Packets are processed **in place**
  after the physical-to-normalized mapping and before the final projection, on the engine
  thread: a plugin call is a plain function-pointer call, with no allocation or
  serialization on the hot path.
- Plugin settings live in the profile (`plugins` map in the configuration) and are edited
  in the **Filters** tab.

Example plugins live in [`plugins/`](../plugins): `devocub_antichatter`, `hand_speed`,
`hawku_smoothing`, `kalman_smoothing`, `radial_follow`.

---

## 2. The ABI

A plugin exports these C symbols (the macro generates them from a type that implements
`NextTabletPlugin`):

| Symbol | Purpose |
|---|---|
| `ntd_plugin_handshake() -> u32` | Magic number + ABI version. The host refuses a library whose magic or version does not match (`NTD_PLUGIN_MAGIC`, `NTD_PLUGIN_ABI_VERSION`). |
| `ntd_plugin_manifest() -> *const c_char` | JSON manifest: id, name, version, author, description, icon, and the list of properties. |
| `ntd_plugin_create() -> *mut c_void` | Creates an instance. |
| `ntd_plugin_process(instance, *mut PluginPacket, *const PluginContext) -> PluginStatus` | Filters one packet in place. |
| `ntd_plugin_set_property(instance, key, value_json) -> PluginStatus` | Applies a user setting. |
| `ntd_plugin_reset(instance) -> PluginStatus` | Clears internal state (pen left proximity, ...). |
| `ntd_plugin_destroy(instance) -> PluginStatus` | Frees the instance. |

`PluginPacket` carries the normalized position (`u`, `v`), pressure, tilt, whether the tip
is down and a timestamp; `PluginContext` carries the active-area and target-area sizes so a
filter can work in physical units.

### Manifest and generated UI

The manifest declares the settings; there is no UI code to write. Property kinds are
`Float`, `Int`, `String`, `Bool` and `Choice`, each with its range/step/default and an
optional tooltip. The **Filters** tab renders, from the manifest alone: the sidebar entry,
title, author and version badges, description, the enable toggle, one control per property
and a **Reset to defaults** button.

### Settings persistence

```rust
pub plugins: HashMap<String, DynamicPluginSettings>, // key = plugin id
```

Each entry stores `enabled` and a `properties` map. Values are passed to the plugin through
`ntd_plugin_set_property`; a plugin must therefore validate and clamp what it receives
(the host does not check values against the manifest ranges).

---

## 3. Installing and trusting plugins

A plugin runs **native code inside the driver process** with the same rights as the app, and
it sees every pen packet. Loading whatever library appears in the plugins folder would let
any program that can write there get code executed at the next launch, so the driver only
loads libraries it **trusts**:

1. **Installed through the app.** *Filters > +* opens a native file picker (the webview
   never supplies a path), shows a confirmation, copies the library into the plugins folder
   and records its **SHA-256** in `trusted_plugins.json`.
2. **Approved by the user.** A library found in the plugins folder that is not in the trust
   store is **not loaded**. The Filters tab lists it as pending, and *Review and trust*
   shows a native confirmation with the file name and hash. Approval is bound to that exact
   content hash: if the file changes afterwards, it becomes pending again.
3. **Officially signed.** If the build embeds a minisign public key
   (`NTD_PLUGIN_PUBKEY`) and `<library>.minisig` verifies against it, the library is
   trusted without a prompt. Intended for first-party plugins shipped with a release.

Trust is a decision about *who wrote the code*; the driver cannot sandbox it. Only trust
plugins from authors you know.

### Where things are

| Item | Location |
|---|---|
| Plugins folder | `Settings/plugins/` (**Filters > folder icon** opens it) |
| Trust store | `Settings/trusted_plugins.json` (hash -> file name) |

### Building during development

Every rebuild changes the library's hash, so a freshly built plugin starts pending. Either
approve it in the Filters tab, or let the helper script build and approve in one step:

```powershell
./scripts/build_plugins.ps1 -Trust
```

`-Trust` only records the hashes of libraries the script itself just built.

---

## 4. Writing a plugin

1. Create a `cdylib` crate depending on `ntd_plugin_api` (see
   [`plugins/hawku_smoothing`](../plugins/hawku_smoothing) for a small example).
2. Implement `NextTabletPlugin` (`manifest`, `process`, `set_property`, `reset`) and call
   `export_ntd_plugin!(YourType)`.
3. Build with `cargo build --release`, then install the library from the app.

Keep `process` allocation-free and fast: it runs for every packet on the real-time thread,
and a panic in a plugin must never cross the FFI boundary.

For embedding the whole engine in another application instead, see [`SDK.md`](SDK.md).
