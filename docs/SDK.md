# NextTabletDriver SDK

A native SDK (`ntd_sdk.dll` / `libntd_sdk.so`) for embedding NextTabletDriver's
tablet engine directly inside a third-party application: a game, a Blender
plugin, a Unity project. Unlike a thin client, the SDK **embeds and runs its
own instance of the engine inside your process**: detection, HID parsing, and
the filter/mapping pipeline all run in-process. Your application does **not**
need the NextTabletDriver desktop app installed or running.

Because the embedded engine never goes through `engine::injector` (no
`SendInput` on Windows, no `uinput` on Linux), you receive tablet pressure and
tilt **raw and lossless**, sidestepping the pressure/tilt loss the desktop
app suffers when it has to inject synthetic mouse/stylus events into the OS.

Supported platforms: Windows and Linux.

## What v1 exposes

- Live tablet state: normalized position (`u`/`v`), mapped screen
  coordinates, pressure, tilt, buttons, pen-down/eraser flags, connection
  status, device identity (name/VID/PID).
- Read/write access to two config fields: driver mode (absolute/relative) and
  the active mapping area.

Not in v1 (deliberately out of scope): event callbacks (polling only) and
profile persistence.

## Building

From the repo root:

```bash
cargo build --release --manifest-path sdk/Cargo.toml
```

This produces:

- `sdk/target/release/ntd_sdk.dll` (+ import library) on Windows, or
  `sdk/target/release/libntd_sdk.so` on Linux: the redistributable library.
- `sdk/include/ntd_sdk.h`: regenerated on every build by `cbindgen`.
- `sdk/bindings/csharp/NextTabletDriver.Sdk/NativeMethods.g.cs`: regenerated
  on every build by `csbindgen`, from the same `sdk/src/ffi.rs` source as the
  C header, so the two can never silently drift apart.

The root NextTabletDriver crate builds the desktop app behind a `gui` Cargo
feature (enabled by default); `sdk/` depends on it with
`default-features = false`, so none of the GUI dependencies (`eframe`, `egui`,
`gtk`, ...) are pulled into the SDK build.

## C / C++

Include the generated header and link against the library:

```c
#include "ntd_sdk.h"
```

### Exported functions

| Function | Description |
| --- | --- |
| `uint32_t ntd_sdk_abi_version(void)` | This build's FFI ABI version. Compare it against the version you compiled/generated your bindings for. |
| `int32_t ntd_init(void)` | Starts the embedded engine. Becomes the HID owner if no other process currently is one, otherwise starts in reader mode. Idempotent. |
| `void ntd_shutdown(void)` | Stops the embedded engine and joins its background thread. Safe to call even if `ntd_init` was never called. |
| `int32_t ntd_poll_state(NtdState *out_state)` | Copies the current tablet + config snapshot into `*out_state`. |
| `int32_t ntd_set_mode(uint8_t mode)` | Sets the driver mode (`0` = absolute, `1` = relative). |
| `int32_t ntd_set_active_area(float x, float y, float w, float h, float rotation)` | Sets the active mapping area, in millimeters (clamped to the device's physical surface). |

### `NtdState`

Field-for-field: `is_connected`, `status`, `u`/`v`, `screen_x`/`screen_y`,
`pressure`, `tilt_x`/`tilt_y`, `buttons`, `is_down`, `eraser`,
`device_name`/`device_name_len` (UTF-8, fixed 64-byte buffer), `vid`/`pid`,
`mode`, `active_area_x`/`_y`/`_w`/`_h`/`_rotation`, `config_version`. See
`sdk/include/ntd_sdk.h` for exact types and doc comments.

### Error codes

| Code | Name | Meaning |
| --- | --- | --- |
| `0` | `NTD_OK` | Success. |
| `-1` | `NTD_ERR_HID_INIT_FAILED` | `ntd_init` only: the HID backend itself failed to initialize (no supported backend, missing permissions, ...). |
| `-2` | `NTD_ERR_NOT_INITIALIZED` | `ntd_init` was never called, or `ntd_shutdown` already ran. |
| `-3` | `NTD_ERR_NULL_POINTER` | A required output pointer was null. |
| `-4` | `NTD_ERR_INVALID_ARGUMENT` | An argument was outside its valid range (e.g. an unknown mode byte). |
| `-5` | `NTD_ERR_COMMAND_FAILED` | The current HID owner process couldn't be reached to apply a forwarded command. |
| `-6` | `NTD_ERR_PANIC` | The call panicked; caught at the FFI boundary and never propagated to the host. |

A working example is in [`sdk/examples/c_consumer`](../sdk/examples/c_consumer).

### Diagnostics

`ntd_init` installs a minimal stderr logger the first time it's called (skipped
if the host process already installed its own `log` backend), so you can see
what's happening inside the embedded engine: HID init failures, HID-owner
vs. reader transitions, device connect/disconnect, read errors, instead of
only ever seeing `is_connected == false` with no explanation. Control the
level with the `NTD_SDK_LOG` environment variable
(`off`/`error`/`warn`/`info`/`debug`/`trace`; defaults to `info`):

```bash
NTD_SDK_LOG=debug dotnet run --project sdk/examples/csharp_console
```

If you only ever see `(no tablet connected)` with no other output even at
`debug`, that means the engine loop is working correctly and simply hasn't
parsed a valid packet from a supported device yet. Check the physical
connection and that your tablet model has a driver under
[`src/drivers/parsers`](../src/drivers/parsers).

## C# / .NET / Unity

C# is a first-class target. Unity is the most common engine a third-party
tablet integration gets built for, so the SDK ships more than a raw P/Invoke
surface:

- `NativeMethods.g.cs`: generated by `csbindgen` from `ffi.rs`, the six
  `[DllImport("ntd_sdk")]` declarations and the `NtdState` struct in
  `[StructLayout(LayoutKind.Sequential)]`. Regenerated on every SDK build;
  don't edit it by hand.
- `NtdClient.cs`: a hand-written idiomatic wrapper (`sdk/bindings/csharp/NextTabletDriver.Sdk/NtdClient.cs`)
  that does not change on regeneration:
  - `NtdClient : IDisposable`, with `ntd_init` in the constructor, `ntd_shutdown`
    in `Dispose()`.
  - `PollState()` returns a managed `TabletState` struct, with `DeviceName`
    already decoded to a C# `string` rather than leaving you to handle the
    raw fixed byte buffer.
  - `SetMode(DriverMode)`, `SetActiveArea(x, y, w, h, rotation)`.
  - Native `int32_t` error codes are translated into a thrown `NtdException`
    rather than leaking numeric codes into idiomatic C# call sites.

```csharp
using var client = new NtdClient();
client.SetMode(DriverMode.Absolute);
client.SetActiveArea(0f, 0f, 152f, 95f, 0f);

var state = client.PollState();
Console.WriteLine($"u={state.U} v={state.V} pressure={state.Pressure}");
```

`sdk/bindings/csharp/NextTabletDriver.Sdk/NextTabletDriver.Sdk.csproj` targets
`netstandard2.1` (compatible with both Unity and modern .NET). It's meant to
be consumed as source: copy the `NextTabletDriver.Sdk/` folder into your
project, or reference the `.csproj` directly, since Unity doesn't handle
NuGet-distributed native plugins well. A `.nupkg` is a possible future
improvement, not built for v1.

A console example is in
[`sdk/examples/csharp_console`](../sdk/examples/csharp_console).

### Unity

1. Build the native library (see **Building** above).
2. Copy `ntd_sdk.dll` (Windows) or `libntd_sdk.so` (Linux) into
   `Assets/Plugins/x86_64/` (Windows and Linux both use this path by
   convention; Linux may alternatively use `Assets/Plugins/Linux/x86_64/`).
3. Copy `NativeMethods.g.cs` and `NtdClient.cs` into your Unity project's
   `Assets/Scripts/` (or any folder Unity compiles).
4. `[DllImport]` in the generated bindings uses the bare name `"ntd_sdk"`;
   no extension or `lib` prefix needed. Unity resolves that to
   `ntd_sdk.dll` on Windows and `libntd_sdk.so` on Linux automatically.

## HID ownership arbitration

Multiple processes can end up wanting the same physical tablet at once: the
desktop app, one SDK-embedding game, two SDK-embedding games at the same
time. Opening the same HID device from more than one process at a time can
fail or behave unpredictably depending on the tablet's firmware, so exactly
**one** process at a time is the HID **owner**; every other process is a
**reader**.

- The owner opens the real HID device, runs the actual detection/parsing
  pipeline, and publishes its state into a small named shared-memory segment
  (a seqlock, so readers never observe a torn/inconsistent snapshot).
- Readers don't touch the HID device at all. They mirror the owner's
  published state into their own local state, so `ntd_poll_state` behaves
  identically regardless of which role the calling process has.
- A reader that calls `ntd_set_mode`/`ntd_set_active_area` doesn't write
  locally; it forwards the request to the current owner over a small
  command channel (named pipe on Windows, Unix socket on Linux), and the
  owner applies it through the exact same validation path a local write
  would use.
- Ownership is decided automatically, first-come-first-served, via a
  machine-wide named lock. If the owner process exits, its lock is released
  automatically (by the OS), and a reader retrying in the background is
  promoted to owner, transparently, with no interruption visible to
  `ntd_poll_state` callers.

This requires no configuration from either the SDK consumer or the end user.
It uniformly handles: desktop app alone, desktop app + one game, desktop app
+ several games, several games with no desktop app running, and the current
owner crashing or being closed while others are still running.

## Global state

Each loaded copy of `ntd_sdk` (one per process, or one per plugin host if the
library gets loaded into isolated contexts) has a single global embedded
engine instance. `ntd_init()` is idempotent: call it once at startup and
`ntd_shutdown()` once at teardown. Don't construct more than one `NtdClient`
in the same process; disposing one would shut down the engine the others are
still using.
