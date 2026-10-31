# C example

A small C program that uses the NextTabletDriver SDK. It starts the engine, sets the mode and the active area once, reads the pen state in a loop (200 times, every 5 ms), and then stops the engine.

It runs on its own. You do not need the NextTabletDriver desktop app. For an explanation of the SDK and its functions, read [Integrations](../../../../docs/integrations.md#sdk).

On Linux, the [udev rules](../../../../docs/linux.md) must be installed, so the program may open your tablet.

## Build

First build the SDK library. Run this from the root of the repository:

```bash
cargo build --release --manifest-path apps/sdk/Cargo.toml
```

This creates the library in `apps/sdk/target/release/`: `ntd_sdk.dll` and an import library on Windows, or `libntd_sdk.so` on Linux. It also regenerates the header `apps/sdk/include/ntd_sdk.h`.

Then compile the example. Run these commands from this folder, `apps/sdk/examples/c_consumer`.

### Windows with MSVC

Use a "Developer Command Prompt" so that `cl.exe` is available.

```bat
cl main.c /I ..\..\include /link ..\..\target\release\ntd_sdk.dll.lib
copy ..\..\target\release\ntd_sdk.dll .
main.exe
```

### Windows with MinGW-w64

```bash
gcc main.c -I ../../include -L ../../target/release -lntd_sdk -o main.exe
cp ../../target/release/ntd_sdk.dll .
./main.exe
```

### Linux with gcc

```bash
gcc main.c -I ../../include -L ../../target/release -lntd_sdk -o main
LD_LIBRARY_PATH=../../target/release ./main
```

## What you should see

The program prints the interface version, and then one line per reading:

- the pen position and pressure, when a pen is near the tablet,
- `(tablet found, pen out of range ...)` when the tablet is open but no pen is near it,
- `(no tablet detected)` when no supported tablet was found.
