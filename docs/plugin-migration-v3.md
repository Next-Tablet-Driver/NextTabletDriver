# Port a plugin to ABI v3

NextTabletDriver v3 loads plugins built for **ABI v3** only. A plugin built for an older interface
(the v2 app used interface version 1) is **refused**; it is not loaded and the Console says:

```
Incompatible plugin ABI version: this app loads ABI v3 plugins, this one was built for ABI v1; ask
its author for a v3 build (see docs/plugin-migration-v3.md)
```

The five official plugins are already rebuilt. This page is for the authors of other plugins.

## What changed

| | ABI v1 / v2 | ABI v3 |
| --- | --- | --- |
| Interface version (`NTD_PLUGIN_ABI_VERSION`) | 1 | **3** |
| Handshake value | `0x4E540001` | **`0x4E540003`** |
| Manifest | no target | new field **`target`**: the Rust target triple the library was built for |
| `PluginPacket.is_down` | `bool` | **`u8`**: `1` while the tip touches, else `0` |
| Return value of the functions | `NtdPluginStatus`, a C `enum` | **`int32_t`**: `0` is success, any other value is an error |
| A plugin that returns an error from `process` | the packet went on, the plugin was called again | the plugin is **disabled** until the app restarts; the pipeline goes on without it |

Why: a `bool` or an `enum` that holds a value the type does not allow is undefined behaviour in
Rust, and a plugin is foreign code. Every type that crosses the boundary now has a fixed width, and
the host reads a status as a plain integer. The target is checked before the OS loader gives a
cryptic message for a library of another architecture.

The packet layout is unchanged: 32 bytes, 8-byte alignment, `is_down` still at offset 20.

## Port a Rust plugin

1. Update the `ntd_plugin_api` dependency to a commit (or version) that has ABI v3, and rebuild.
2. In the `PluginManifest` you return, add `target: String::new()`. `export_ntd_plugin!` writes the
   real target triple into the manifest for you; whatever you put there is replaced.
3. If you read or write `packet.is_down`, use the `u8` (`1` or `0`) instead of a `bool`. The host
   ignores a change to this field.
4. Nothing else: the macro exports the same seven functions with the new return type.

Build once per target you support (`cargo build --release --target <triple>`). A library only runs
on the operating system and processor it was built for. The supported triples are:

| Target triple | Platform |
| --- | --- |
| `x86_64-pc-windows-msvc` | Windows, 64-bit Intel/AMD |
| `aarch64-pc-windows-msvc` | Windows on ARM |
| `x86_64-unknown-linux-gnu` | Linux, 64-bit Intel/AMD |
| `aarch64-unknown-linux-gnu` | Linux, ARM64 |

## Port a plugin in C or another language

1. Return `(0x4E540000 & 0xFFFF0000) | (3 & 0x0000FFFF)` from `ntd_plugin_handshake`.
2. Add `"target":"<triple>"` to the manifest JSON. Use the triple of the platform the library is
   built for (see the table above). The host compares it with its own, character for character.
3. Declare `is_down` as `uint8_t`, not `bool`.
4. Declare the status as `int32_t` and return `0` (success) or `1` (error).

The tested example is [`apps/sdk/examples/c_plugin`](../apps/sdk/examples/c_plugin).

## What the host checks, in order

1. The library is **trusted**: its SHA-256 is in the user's list, or it has a valid official
   `.minisig` signature. An untrusted library is **never opened** (opening it would run its start-up
   code).
2. The handshake: the magic, then the interface version (a v1 or v2 library stops here).
3. The manifest, then its `target`.
4. The seven exported functions.

A plugin that returns an error or panics while processing a packet is disabled and logged once. The
plugin SDK catches panics for you (`catch_unwind` in every exported function); a plugin in another
language must not let an exception or a signal escape.

## Not changed

Properties, their JSON format and the manifest fields other than `target` are the same. The
settings a user saved for your plugin keep working as long as you keep the `id` and the property
ids.
