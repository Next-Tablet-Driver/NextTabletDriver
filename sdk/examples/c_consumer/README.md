# c_consumer

Minimal C example: initializes the embedded engine, sets the mode and active
area once, polls tablet state in a loop, then shuts down. Runs standalone,
no NextTabletDriver desktop app required.

## Build

First build the SDK itself from the repo root:

```bash
cargo build --release --manifest-path sdk/Cargo.toml
```

This produces `sdk/target/release/ntd_sdk.dll` (+ an import library) on
Windows, or `sdk/target/release/libntd_sdk.so` on Linux, and regenerates
`sdk/include/ntd_sdk.h`.

### Windows (MSVC, `cl.exe` from a "Developer Command Prompt")

```bat
cl main.c /I ..\..\include /link ..\..\target\release\ntd_sdk.dll.lib
copy ..\..\target\release\ntd_sdk.dll .
main.exe
```

### Windows (MinGW-w64, `gcc`)

```bash
gcc main.c -I ../../include -L ../../target/release -lntd_sdk -o main.exe
cp ../../target/release/ntd_sdk.dll .
./main.exe
```

### Linux (`gcc`)

```bash
gcc main.c -I ../../include -L ../../target/release -lntd_sdk -o main
LD_LIBRARY_PATH=../../target/release ./main
```
