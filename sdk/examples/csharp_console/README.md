# C# example

A small .NET console program that uses the NextTabletDriver SDK through the `NtdClient` class. It starts the engine, sets the mode and the active area once, reads the pen state in a loop (200 times, every 50 ms), and then stops the engine.

It runs on its own. You do not need the NextTabletDriver desktop app. For an explanation of the SDK, read [Integrations](../../../docs/integrations.md#sdk).

On Linux, the [udev rules](../../../docs/linux.md) must be installed, so the program may open your tablet.

## Build and run

1. Build the native SDK library. Run this from the root of the repository:

   ```bash
   cargo build --release --manifest-path sdk/Cargo.toml
   ```

   This creates `sdk/target/release/ntd_sdk.dll` (Windows) or `sdk/target/release/libntd_sdk.so` (Linux).

2. Put the library where .NET can find it, next to this project:

   - **Windows:** copy `sdk/target/release/ntd_sdk.dll` into `sdk/examples/csharp_console/`.
   - **Linux:** copy `sdk/target/release/libntd_sdk.so` into `sdk/examples/csharp_console/`. Or leave it where it is and run the program with `LD_LIBRARY_PATH=../../target/release`.

3. Run it, from the root of the repository:

   ```bash
   dotnet run --project sdk/examples/csharp_console
   ```

## What you should see

The program prints the interface version, and then one line per reading: the pen position and pressure when a pen is near the tablet, a message when the tablet is open but no pen is near it, or `(no tablet detected)`.
