# Getting started

This guide takes you from download to a working tablet. It also explains every part of the app.

## Before you start

- **Windows 10 or 11**, or **Linux** (X11 or Wayland). Linux needs one extra step: [Linux setup](linux.md).
- **Your tablet maker's driver can get in the way.** Two programs cannot use the same tablet at once. If the pen does not work, close or uninstall the maker's driver and try again.
- NextTabletDriver reads tablet definitions from [OpenTabletDriver](https://github.com/OpenTabletDriver/OpenTabletDriver). Check [Supported tablets](../README.md#supported-tablets) if you are not sure that yours is covered.

## Install

### Windows

1. Download `NextTabletDriver_<version>_x64-setup.exe` from the [releases page](https://github.com/Next-Tablet-Driver/NextTabletDriver/releases). On an ARM PC, download the `arm64` file instead.
2. Run it and follow the steps. If you have an older version that was installed with Inno Setup, the installer removes it first.
3. Start NextTabletDriver from the Start menu.

The installer is not code-signed yet, so Windows SmartScreen may say "Windows protected your PC". Choose **More info**, then **Run anyway**.

### Linux

Follow [Linux setup](linux.md). It covers the `.deb` package, the AppImage and NixOS, and the permissions you need.

## Your first launch

Plug in your tablet, then start the app. The footer at the bottom shows the name of your tablet. If it says **No Tablet Detected**, see [Troubleshooting](troubleshooting.md#my-tablet-is-not-detected).

The app has seven tabs:

| Tab | What it is for |
| --- | --- |
| **Output** | Choose which part of the tablet maps to which part of the screen. |
| **Filters** | Turn plugins on or off and change their settings. |
| **Pen Settings** | Tip threshold, pressure curve and other pen options. |
| **Console** | Live log messages from the driver. |
| **Settings** | Startup options, theme, language and the WebSocket server. |
| **Release** | Release notes of recent versions. |
| **Credits** | The people who built the project. |

The menu bar has **File** (profiles), **Tablet** (the Tablet Debugger and Input Lag Analysis windows, which are still experimental) and **Help** (links and update check).

## Set up your area

The **Output** tab has two panels.

- **Display** is your screen. The *display area* (also called the target area) is the part of your desktop that the pen can reach. Its size and position are in pixels.
- **Tablet** is your tablet. The *active area* is the part of the tablet surface that you use. Its size and position are in millimeters. X and Y are the position of the center of the area, measured from the top-left corner of the tablet.

You can drag the areas with the mouse or type exact numbers.

| Option | What it does |
| --- | --- |
| **Force Aspect Ratio** | Keeps the active area the same shape as the display area, so a circle drawn on the tablet is a circle on screen. |
| **Show Osu!Playfield** | Draws the osu! playfield inside the active area, so you can see how much of the tablet it uses. |
| **Rotation** | Turns the active area, in degrees. |
| **Enable Edge Snapping** | Snaps the display area to the edges of your monitors while you drag it. |

Keyboard shortcuts on this tab: **Ctrl+Z** undo, **Ctrl+Y** redo, **Ctrl+Shift+Z** go back to where you started, **Ctrl+S** save.

### Absolute and relative mode

Use the mode selector in the footer.

- **Absolute mode**: each point on the tablet is one point on the screen. Use it for osu! and for drawing.
- **Relative mode**: the pen moves the cursor like a mouse. You can lift the pen and move it back without moving the cursor. The sensitivity is 10 pixels per millimeter, and movement resets after 100 ms of no input. These two values cannot be changed in the app yet.

You can also switch the mode from the tray menu.

## Save your settings

Your changes work **right away**, but they are only kept for the next launch when you save them.

- **Ctrl+S** on the Output tab, or **File → Save Settings**, saves your settings. They come back the next time you start the app.
- **File → Save Settings As...** (or **Export .json**) writes your settings to a `.json` file that you choose.
- **File → Load Settings...** (or **Import .json**) loads such a file.
- **File → Reset to default** goes back to the default settings.

### Profiles

A profile is a saved set of settings, for example one for osu! and one for drawing. Every `.json` file in the `profiles` folder (see [Where your files are](#where-your-files-are)) shows up under **File → Presets** and in the tray menu under **Profiles**. To make a new profile, use **Save Settings As...** and save the file in that folder.

### Import from OpenTabletDriver

**File → Import OTD Settings** reads an OpenTabletDriver settings file. It uses the first profile in the file and imports the mode, the tablet and display areas, and the relative mode sensitivity and rotation. It does not import filters or bindings.

## Pen settings

On the **Pen Settings** tab, these options have an effect today:

- **Tip threshold**: how hard you press before the pen counts as touching, as a percentage of the maximum pressure.
- **Disable pressure**: the pen reports full pressure whenever it touches the tablet. The pen still has to touch to count as down.
- **Pressure curve**: reshape the pressure. Linear, exponential, or your own curve. With **Custom**, drag the points on the graph, click the graph to add one, and double-click a point to remove it. The curve changes the pressure that other programs receive, so it matters when Windows Ink is on, or on Linux.

- The **Drawing** tab turns on Windows Ink, so drawing programs get pressure and tilt. Leave it off for osu!. See [Drawing](drawing.md).
- **Button Actions** and **Tablet Keys**: decide what each pen button and each key on the tablet does. See [Button mapping](button-mapping.md).

The other options on this tab (eraser threshold, disable tilt, tip and eraser bindings) are saved but not used yet. See [Known limitations](troubleshooting.md#known-limitations).

### Pressure and tilt

| | Windows | Linux |
| --- | --- | --- |
| Cursor position | Yes | Yes |
| Click when the tip touches | Yes (left mouse button) | Yes |
| Pressure and tilt reach other apps | Only with Windows Ink | Yes |

By default on Windows, NextTabletDriver acts like a mouse. It still reads pressure and tilt, and it shows them in the app, but other programs do not receive them. This is the best choice for osu!, which only needs position and a click. For drawing, turn on Windows Ink on the **Drawing** tab. See [Drawing](drawing.md).

## Filters (plugins)

Filters change the pen position before it reaches the screen. The most common use is smoothing. The installer does not include any filters yet. See [Plugins](plugins.md) to build and install them.

Two things to know. Filters only work in absolute mode. And when you add a filter, choose **Reload Engine** in the tray menu so that the driver starts to use it.

## Settings

| Setting | What it does |
| --- | --- |
| **Run at startup** | Starts NextTabletDriver in the background when you sign in. It starts hidden, in the tray. |
| **System Tray when Minimize** | Minimizing the window hides the app in the tray instead of the taskbar. Closing the window always quits the app. To bring the window back, double-click the tray icon. |
| **Force High Resolution Timer** | Windows only. Asks Windows for a 0.5 ms timer to reduce delay. On by default. |
| **Theme** | Change the colors. See [Themes](themes.md). |
| **Language** | The choice is saved, but the interface is in English only for now. |
| **WebSocket server** | Sends live pen data to other programs on your PC. Off by default. See [Integrations](integrations.md). |

The checkbox **Enable Anonymous Usage Statistics** is a placeholder. It is not connected to anything, and the app does not send any data.

## The tray icon

The tray icon is always there while the app runs. Double-click it to show the window. Right-click it for the menu: show the dashboard, see the tablet name, switch profile or mode, open the plugins and themes folders, check for updates, reload the engine, and quit.

## Updates

When a new version is available, the title bar shows it. You can also use **Help → Check for Update** or the tray menu. The app asks before it downloads and installs the update, then restarts.

## Uninstall

- **Windows**: open **Settings → Apps**, find NextTabletDriver, and choose **Uninstall**. The uninstaller asks if you want to delete your profiles and settings too.
- **Linux**: remove the package with your package manager, or delete the AppImage. To remove your settings, delete the settings folder below.

## Where your files are

All your data is in one `Settings` folder.

| System | Folder |
| --- | --- |
| Windows | `%APPDATA%\NextTabletDriver\NextTabletReader\config\Settings` |
| Linux | `~/.config/nexttabletreader/Settings` |

| Item | What it is |
| --- | --- |
| `last_session.json` | The settings that load at startup. This is what **Save Settings** writes. |
| `profiles/` | Your profiles (presets). |
| `plugins/` | Plugin libraries (`.dll` on Windows, `.so` on Linux). |
| `Themes/` | Your own theme files. |
| `trusted_plugins.json` | The plugins you approved. |
| `app_preferences.json` | App-level preferences. |
| `session.log`, `session.prev.log` | The log of this run and of the previous run. |

To start fresh, close the app and delete this folder.

## Next steps

- Something is wrong? [Troubleshooting](troubleshooting.md).
- Want smoother lines? [Plugins](plugins.md).
- Want to change the look? [Themes](themes.md).
