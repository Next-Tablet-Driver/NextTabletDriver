# C plugin example

A NextTabletDriver plugin written in C. It mirrors the horizontal axis, and has one setting, a checkbox called **Flip horizontally**. It shows that a plugin does not need Rust: it only needs to export seven C functions.

The full contract (function signatures, structure layouts, rules) is in [Writing a plugin in another language](../../../docs/plugin-development.md#writing-a-plugin-in-another-language).

## Build

You need a C compiler for your system, and for the same CPU type as the app (x64 or ARM64). Run the command in this folder.

**Windows, Visual Studio.** Open a "Developer Command Prompt" so that `cl.exe` is available.

```bat
cl /O2 /LD plugin.c /Fe:c_flip_x.dll
```

This also creates `plugin.obj`, `c_flip_x.lib` and `c_flip_x.exp`. You do not need them. Delete them.

**Windows, MinGW-w64.**

```bash
gcc -O2 -shared plugin.c -o c_flip_x.dll
```

**Linux.**

```bash
cc -O2 -shared -fPIC plugin.c -o libc_flip_x.so
```

The MSVC command compiles with no warnings at warning level 4 (`/W4`) and the library loads in NextTabletDriver. The CI compiles this file on Linux with `-Wall -Wextra -Werror` on every pull request, and checks that the library exports the seven plugin functions. The MinGW command is the usual one for a shared library and is not part of the CI.

## Try it

Follow [Try it in the app](../../../docs/plugin-development.md#try-it-in-the-app): copy the file into the `plugins` folder, approve it on the **Filters** tab, choose **Reload Engine** in the tray menu, and enable the plugin. It works in absolute mode. With the checkbox on, moving the pen to the right of the active area moves the cursor to the left.

## How it works

`plugin.c` has the four parts that every plugin needs:

1. **The structures** `NtdPluginPacket`, `NtdPluginContext` and `NtdPluginStatus`. They must match the engine's layout exactly.
2. **The manifest**, a JSON string. It gives the plugin an `id`, a name, and a list of settings.
3. **The state** of one instance (here only the `flip` flag).
4. **The seven exported functions**: `ntd_plugin_handshake`, `ntd_plugin_manifest`, `ntd_plugin_create`, `ntd_plugin_process`, `ntd_plugin_set_property`, `ntd_plugin_reset` and `ntd_plugin_destroy`.

Notes:

- `ntd_plugin_create` sets `flip` to `true`, the same as the `default` in the manifest. The two must match, because the app only sends settings that the user changed.
- `ntd_plugin_set_property` reads the JSON text of the value. Here it only needs to compare it with `"true"` and `"false"`. A real plugin that reads numbers should parse them carefully and ignore anything it cannot read, including `null`.
- `ntd_plugin_process` never allocates memory. It runs for every packet.
