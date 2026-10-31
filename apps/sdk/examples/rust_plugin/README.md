# Rust plugin example

A complete NextTabletDriver plugin in Rust. It is a simple smoothing filter with one setting, **Strength**. Copy this folder to start your own plugin.

The guide that explains everything is [Plugin development guide](../../../../docs/plugin-development.md).

## What is in this folder

| File | What it is |
| --- | --- |
| `Cargo.toml` | A `cdylib` crate that depends on `ntd_plugin_api` and `serde_json`. |
| `src/lib.rs` | The plugin: the manifest, `process`, `set_property`, `reset`, and the tests. |
| `Cargo.lock` | The exact versions of the dependencies. |

## Use it

You need Rust 1.99 or newer. Run these commands in this folder.

```bash
cargo test                # run the tests (no tablet needed)
cargo build --release     # build the plugin
```

The plugin is in `target/release/`: `simple_smoothing.dll` on Windows, `libsimple_smoothing.so` on Linux.

To try it in the app, follow [Try it in the app](../../../../docs/plugin-development.md#try-it-in-the-app). In short: quit the app, copy the file into the `plugins` folder, start the app, approve the plugin on the **Filters** tab, choose **Reload Engine** in the tray menu, and enable the plugin. It works in absolute mode.

## Make it your own

1. Change `name` in `Cargo.toml`.
2. In `src/lib.rs`, change the manifest: `id`, `name`, `version`, `author`, `description`. Choose an `id` that nobody else uses, and never change it after you publish.
3. Replace the logic in `process`, and the settings in `manifest` and `set_property`.
4. Keep the tests, and change them to match. They check the things that often go wrong: the first packet, `reset` called twice, bad setting values, and defaults that match the manifest.
5. If you move the folder out of this repository, replace the `ntd_plugin_api` path in `Cargo.toml` with a git dependency, as the comment there shows.

Do not set `panic = "abort"` in your profile. The plugin interface relies on panics being caught.

The CI formats, lints, tests and builds this example on every pull request, and checks that the library exports the seven plugin functions. If you change the plugin interface, this example tells you what you broke.
