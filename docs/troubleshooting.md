# Troubleshooting

Find your problem below. If nothing helps, [report a bug](#report-a-bug).

## Start with the logs

The logs tell you what the driver is doing.

- Open the **Console** tab. It shows live messages. Use the buttons to filter by **Info**, **Warn**, **Error** or **Debug**, and the search box to find text. It keeps the last 2000 lines.
- The same messages are written to files in your `Settings` folder (see [Where your files are](getting-started.md#where-your-files-are)): `session.log` for this run and `session.prev.log` for the previous run. Use `session.prev.log` if the app crashed.

## My tablet is not detected

The footer says **No Tablet Detected**.

1. Plug the tablet straight into the PC. Try another USB port and another cable. Avoid hubs.
2. Close or uninstall your tablet maker's driver. Two programs cannot use the tablet at the same time.
3. **Linux:** install the udev rules and check your permissions. See [Linux setup](linux.md).
4. Look at the Console. If the tablet is not in the definition list, you see a warning like this:

   ```text
   Found unrecognized potential tablet device: [28bd:0914] 'XP-Pen' - 'Deco 01 V2'.
   ```

   The numbers in brackets are your tablet's VID:PID. The model is not supported yet. See [Adding a tablet](adding-a-tablet.md), and include these numbers if you ask for support.
5. On some tablets, the driver needs to see a packet from the pen before it can confirm the model. Move the pen over the surface and look again.

## The tablet is detected, but the cursor does not move

- On the **Output** tab, check the display area. It must cover a part of your screen, with a width and height above zero. If you are not sure, use **File → Reset to default**.
- Make sure you are in the mode you want (**Absolute** or **Relative**, in the footer).
- Only one program can own the tablet. If a game or tool that uses the NextTabletDriver [SDK](integrations.md#sdk) is running, it may be the owner and the app only mirrors it. Close that program.
- Choose **Reload Engine** in the tray menu.

## The cursor moves twice (Linux)

The real tablet and the virtual one are both moving the cursor. The udev rules should prevent this. See [Linux setup](linux.md#the-cursor-moves-twice-or-the-real-tablet-also-moves-it).

## My drawing app gets no pressure (Windows)

On Windows, NextTabletDriver sends position and a left click only. Pressure and tilt are not passed to other programs. This is a current limitation of the Windows input method. On Linux, pressure and tilt work.

## The pen buttons or the eraser do nothing

They are not forwarded to the system yet, on any platform. The settings for them are saved but have no effect. See [Known limitations](#known-limitations).

## My settings are gone after a restart

Your changes work right away, but they are only kept after you save. Press **Ctrl+S** on the Output tab, or use **File → Save Settings**.

## The window is blank, or the app does not start (Windows)

The interface needs the Microsoft Edge WebView2 Runtime. It is part of Windows 11 and of recent Windows 10 updates. If the window stays blank, install the [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) from Microsoft.

## A plugin does not load

- A banner on the **Filters** tab says that a plugin is waiting for approval. Choose **Review and trust**. See [Plugins](plugins.md#how-plugins-are-trusted).
- The plugin is in the list, but nothing changes. The driver reads its plugins when the engine starts, so choose **Reload Engine** in the tray menu after you add or approve a plugin. Also check that the plugin is enabled and that you use **absolute mode**: filters have no effect in relative mode.
- The file must end in `.dll` on Windows or `.so` on Linux, and sit in the `plugins` folder.
- Open the Console and look for messages from `Plugins`. They tell you if the file is damaged or built for a different plugin interface version. The [plugin development guide](plugin-development.md#debugging-and-common-errors) explains each message.

## The WebSocket server does not start

Another program may already use the port (8080 by default). Change the port on the **Settings** tab. If the port is busy, the Console shows `Failed to bind to port`. See [Integrations](integrations.md#websocket).

## Start from scratch

Close the app and delete your `Settings` folder (see [Where your files are](getting-started.md#where-your-files-are)). The next launch starts with the defaults.

## Known limitations

These are things that do not work yet, or not fully. They are listed so that you do not waste time. Each one is a known gap, not something wrong with your setup.

| Area | What to expect | What you can do |
| --- | --- | --- |
| Pressure and tilt on Windows | Not sent to other programs. | Use position and click only (this is enough for osu!). On Linux they work. |
| Pen side buttons and eraser | Not forwarded on any platform. The **Pen Settings** options *Eraser threshold*, *Disable tilt*, *Tip binding*, *Eraser binding* and *Pen button actions* are saved but not used. | Wait for a later version. |
| Pressure curve on Windows | It only changes the pressure that programs receive, so it has no visible effect on Windows. | None needed. |
| Relative mode | Sensitivity (10 px per mm) and reset time (100 ms) are fixed. There is no setting for them in the app. | Use absolute mode, or import settings from OpenTabletDriver (it brings in its sensitivity). |
| Language | The language choice is saved, but the interface is in English only. | None. |
| Usage statistics | The checkbox **Enable Anonymous Usage Statistics** is a placeholder. The app does not send data. | None. |
| Tablet Debugger and Input Lag Analysis | These two windows (menu **Tablet**) are experimental and may not open correctly. | Use the **Console** tab. |
| Plugins | The installers do not include any plugin. | Build them from source: [Plugins](plugins.md#build-the-plugins-from-this-repository). |
| WebSocket tilt option | The **Tilt** checkbox has no effect. Tilt is not in the data. | Use pressure and position. |
| Online themes button | **Browse Online Themes** opens a page that does not exist. | Use the theme repository: <https://github.com/Next-Tablet-Driver/NextTabletDriver-Themes>. |
| Run at startup on Linux AppImage | May not work, because the AppImage runs from a temporary folder. | Use the `.deb`, or add the AppImage to your startup applications yourself. |
| Windows installer | It is not code-signed. SmartScreen may warn you. | Choose **More info → Run anyway**. |
| Tablet models | A model works only if the project has a parser for it. | See [Adding a tablet](adding-a-tablet.md). |

## Report a bug

A good report gets fixed faster. Please:

1. Check the [open issues](https://github.com/Next-Tablet-Driver/NextTabletDriver/issues) first.
2. Use the latest version.
3. Open a [new issue](https://github.com/Next-Tablet-Driver/NextTabletDriver/issues/new/choose). You can start from **Help → Report Bug / Suggest Feature** in the app.
4. Tell us:
    - your operating system and version,
    - the app version,
    - your tablet model and its VID:PID,
    - what you did, what you expected, and what happened,
    - the relevant lines from the Console, or `session.log` (or `session.prev.log` after a crash).

Do not post private information. Logs can contain your user name in file paths. Remove it if you want.
