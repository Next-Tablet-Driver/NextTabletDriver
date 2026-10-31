# Third-party notices

## OpenTabletDriver tablet configurations (LGPL-3.0)

NextTabletDriver embeds, at compile time, the tablet configuration files of
[OpenTabletDriver](https://github.com/OpenTabletDriver/OpenTabletDriver)
(`tablets/OpenTabletDriver/OpenTabletDriver.Configurations/Configurations`, loaded in
`src/drivers/config_loader.rs`).

- Copyright (C) OpenTabletDriver contributors.
- Licence: GNU Lesser General Public License v3.0, full text in
  [`licenses/OpenTabletDriver-LGPL-3.0.txt`](licenses/OpenTabletDriver-LGPL-3.0.txt).
- Source of the covered files: the `tablets/OpenTabletDriver` git submodule, pinned to the commit
  recorded in this repository. The files are unmodified.
- The configuration files stay under the LGPL-3.0. The rest of NextTabletDriver stays under the
  MIT licence (see `LICENSE`).

## Report parsers ported from OpenTabletDriver (LGPL-3.0)

The report parsers in `src/drivers/parsers/` are Rust ports of the C# parsers of
OpenTabletDriver (`OpenTabletDriver.Configurations/Parsers`). They are derivative works and are
therefore licensed under the **LGPL-3.0**, not the MIT licence. Copyright (C) OpenTabletDriver
contributors and NextTabletDriver contributors.
