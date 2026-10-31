# NextTabletDriver documentation

Find the guide you need.

## Use NextTabletDriver

| Guide | What you will learn |
| --- | --- |
| [Getting started](getting-started.md) | Install the app, set up your area, save profiles, and learn every tab. |
| [Linux setup](linux.md) | Permissions, the `.deb` package, the AppImage, Arch and NixOS. |
| [Troubleshooting](troubleshooting.md) | Fix common problems, read the logs, report a bug, and see the known limitations. |
| [Button mapping](button-mapping.md) | Make the pen buttons and the tablet keys press mouse buttons, keys and shortcuts. |
| [Drawing with Windows Ink](drawing.md) | Report the pen as a real pen (pressure, tilt, hover) for drawing programs. |
| [Plugins](plugins.md) | Smooth the pen movement with filters, or write your own. |
| [Themes](themes.md) | Change the colors and the look of the app. |

## Build on NextTabletDriver

| Guide | What you will learn |
| --- | --- |
| [Plugin development](plugin-development.md) | Write, test, debug and share a plugin, in Rust or in C. |
| [Integrations](integrations.md) | Read live pen data with the WebSocket server, or embed the engine with the SDK (C and C#). |

## Understand and improve the project

| Guide | What you will learn |
| --- | --- |
| [Architecture](architecture.md) | How a pen packet becomes a cursor movement, and how the code is organized. |
| [Development](development.md) | Build, run and test the project. Code style. How the CI works. |
| [Adding a tablet](adding-a-tablet.md) | Find a tablet's USB ID, write a definition, and write a parser. |
| [Releasing](releasing.md) | Version numbers, the update chain, and the one exception that must not happen again. |

More documentation lives next to the code:

- [`.github/CONTRIBUTING.md`](../.github/CONTRIBUTING.md): how to send a change.
- [`.github/SECURITY.md`](../.github/SECURITY.md): how to report a security problem.
- [`.github/SUPPORT.md`](../.github/SUPPORT.md): where to get help.
- [`frontend/README.md`](../frontend/README.md): the interface code.
- [`qa/benches/README.md`](../qa/benches/README.md) and [`qa/fuzz/README.md`](../qa/fuzz/README.md): speed tests and fuzzing.
- [`sdk/README.md`](../sdk/README.md): the two developer kits (plugin API and embedding SDK) and their examples.

## How these docs are written

Help us keep them easy to read.

- **American English** (US spelling), in short sentences and simple words.
- **Keep the technical terms** that developers know: pipeline, thread, plugin, WebSocket, udev. Explain a term the first time it appears on a page.
- **Write to "you".** Use the active voice: "Press Ctrl+S", not "Ctrl+S should be pressed".
- **Say what is true today.** Do not describe a feature that does not work yet. If something is missing, say so in the [known limitations](troubleshooting.md#known-limitations).
- **Every command and every code sample must work as written.** Run it before you commit it.
- **Use the names you see on screen** for buttons and tabs, in bold: **Output**, **Save Settings**.
- **Change the docs in the same pull request as the code.** If a setting, a file path or a command changes, update the guide that mentions it.
