# ntd_plugin_api

The interface for writing NextTabletDriver plugins. A plugin is a dynamic library (`.dll` on Windows, `.so` on Linux) that changes pen data before it is turned into a cursor position.

This crate gives you:

- the `NextTabletPlugin` trait to implement,
- the data types that cross the C interface (`PluginPacket`, `PluginContext`, `PluginStatus`),
- the manifest types that describe your plugin and its settings (`PluginManifest`, `PropertyDescriptor`, `PropertyKind`),
- the macro `export_ntd_plugin!`, which writes the seven exported C functions for you.

## Start here

- **[Plugin development guide](../../docs/plugin-development.md)**: how plugins work, how to test and debug them, and the C interface for other languages.
- **[`examples/rust_plugin`](../../apps/sdk/examples/rust_plugin)**: a complete plugin to copy.

## Use it in your plugin

This crate is not on crates.io. Depend on it with a git dependency, and pin a commit:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
ntd_plugin_api = { git = "https://github.com/Next-Tablet-Driver/NextTabletDriver", rev = "<commit>" }
serde_json = "1.0"
```

`serde_json` is needed because the macro uses it. You need Rust 1.99 or newer.

## Read the API documentation

```bash
cargo doc --manifest-path crates/ntd-plugin-api/Cargo.toml --open
```

## Interface version

The plugin interface version is `NTD_PLUGIN_ABI_VERSION` (currently 1). The engine accepts exactly this version. It is separate from the version of this crate.
