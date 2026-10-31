# AGENTS.md: `ntd-tablets`

Read the root [`AGENTS.md`](../../AGENTS.md) first. This file only adds what is specific to this
crate (plan: [`docs/v3-rewrite.md`](../../docs/v3-rewrite.md) §4.2).

## What lives here

What the driver knows about tablets, without touching a device: the definitions of
`OpenTabletDriver`, the report parsers (`parsers/`), the types they share (`models`) and the
generic driver that routes a report to its parser. **No I/O**: no `hidapi`, no files, no network,
no clock. Finding the connected tablet and reading from it belongs to the I/O crates.

| Path | Role |
|---|---|
| `src/generated/` | **Generated** by `tools/ntd-codegen`: `TABLETS` (one `TabletDescriptor` per definition) and `BY_ID` (sorted VID:PID table). Never edit it. |
| `src/descriptor.rs` | The static types the generated code fills, `all()`, `find(vid, pid)` and the conversion to the owned `TabletConfiguration`. |
| `src/config.rs` | The owned, `Deserialize` form of a definition (also used for the local `tablets` folder by the root crate until `ntd-hid` exists). |
| `src/config_loader.rs` | `EMBEDDED_INDEX`, built from the descriptors: nothing is parsed at startup. |
| `src/parsers/` | Ports of the C# parsers: LGPL-3.0, see `THIRD_PARTY_NOTICES.md`. |

## Rules

- **Descriptors come from the generator.** To add or change a tablet, fix the definition upstream
  and update the `tablets/OpenTabletDriver` submodule, then run
  `cargo run --locked -p ntd-codegen` and commit the result. To change the shape of a descriptor,
  change `src/descriptor.rs` **and** `tools/ntd-codegen/src/emit.rs` in the same commit. CI fails
  when `generated/` is stale.
- **Do not reintroduce `include_dir`, a `build.rs` or JSON parsing of the catalogue** on the
  startup path. `serde_json` stays only for the `Deserialize` types of `config.rs`.
- **Parsers**: never "simplify" a mask or a timing without understanding it (`git blame` first). A
  parser change needs golden files in `qa/golden/`, from a real capture made with
  `tools/ntd-recorder`; never invent or edit one (rules 80, 103).
- A parser must not panic on any input (`indexing_slicing` is `deny`): a short or odd report
  returns `None`. The fuzz targets of P2.7 enforce it.
- `parsers/mod.rs` `create_parser` selects a parser from the class name of the definition; a new
  upstream class needs a Rust parser, or it falls back to the generic one on purpose.
- Licence: `MIT AND LGPL-3.0-only` because of the parsers. Do not move LGPL code into a crate
  that is plain MIT.

## Checks for this crate

```bash
cargo run --locked -q -p ntd-codegen -- --check
cargo nextest run --locked -p ntd-tablets
cargo test --locked -p next_tablet_driver --test golden
cargo tree -p ntd-tablets -e normal    # no hidapi, no I/O crate
```
