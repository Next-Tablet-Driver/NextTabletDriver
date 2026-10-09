# csharp_console

Minimal .NET console example: initializes the embedded engine via
`NtdClient`, sets the mode and active area once, polls tablet state in a
loop, then disposes. Runs standalone, no NextTabletDriver desktop app
required.

## Build & run

1. Build the native SDK from the repo root:

   ```bash
   cargo build --release --manifest-path sdk/Cargo.toml
   ```

   This produces `sdk/target/release/ntd_sdk.dll` (Windows) or
   `sdk/target/release/libntd_sdk.so` (Linux).

2. Copy the native library next to this project so .NET's P/Invoke
   resolution can find it:

   - Windows: copy `sdk/target/release/ntd_sdk.dll` into
     `sdk/examples/csharp_console/`.
   - Linux: copy `sdk/target/release/libntd_sdk.so` into
     `sdk/examples/csharp_console/`, or run with
     `LD_LIBRARY_PATH=../../target/release`.

3. Run it:

   ```bash
   dotnet run --project sdk/examples/csharp_console
   ```
