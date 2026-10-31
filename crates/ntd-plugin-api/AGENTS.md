# AGENTS.md: `ntd-plugin-api`

The plugin ABI. Read the root [`AGENTS.md`](../../AGENTS.md) first; this file adds what is specific
to this crate. Third parties build native libraries against it, so a mistake here breaks plugins
that are not in this repository.

## What it is

- **ABI v3** (`NTD_PLUGIN_ABI_VERSION = 3`). A plugin is a `cdylib` that exports seven `extern "C"`
  functions; `export_ntd_plugin!` writes them. The host is `crates/ntd-plugin-host`.
- The handshake is `(NTD_PLUGIN_MAGIC & 0xFFFF_0000) | version`. Keep its layout: it is how the host
  recognises and refuses a library of another version with a clear message.
- The manifest is JSON across the boundary. `PluginManifest.target` is filled in by the macro from
  `TARGET_TRIPLE`; the host refuses a different value.

## Rules

- **Do not change the ABI inside a major version.** Any change to a `#[repr(C)]` type, to the
  exported functions or to the handshake is ABI v4: a new version, a migration guide, and the host
  refusing v3. If a task seems to need it: stop and ask.
- **Fixed-width, `#[repr(C)]` types only** (`u8`, `i32`, `u32`, `u64`, `f32`). No `bool`, no Rust
  `enum` across the boundary (a value outside the variants is undefined behaviour), no `String`,
  `Vec`, `Box`, `usize` or trait object. Statuses are `i32` (`NTD_STATUS_OK`, `NTD_STATUS_ERROR`).
- **Every exported function guards against panics** with `catch_unwind` and null pointers, including
  `ntd_plugin_manifest`. A panic that unwinds out of `extern "C"` aborts the whole app.
- The hot path (`process`) allocates nothing.
- Layout tests (`the_abi_types_have_a_fixed_layout`) must keep passing; if you must touch a size,
  you are making ABI v4.
- `docs/plugin-development.md`, `docs/plugin-migration-v3.md`, `apps/sdk/examples/c_plugin` and the
  five plugins in `plugins/` are the consumers: update them in the same pull request as an ABI
  change.
- Supported targets are listed in `target_triple()`; add one there and in the migration guide
  together. `"unknown"` never matches a host, so a plugin built for it is refused.
- `serde_json` is re-exported (`ntd_plugin_api::serde_json`) because the macro needs it.

## Checks

```bash
cargo test --locked -p ntd_plugin_api
cargo nextest run --locked -p ntd-plugin-host      # loads a real plugin if one has been built
cargo build --locked -p radial_follow              # build a plugin first for that test to run
```

Then the full list in the root `AGENTS.md`.
