# Drawing with Windows Ink

By default the driver moves the cursor like a mouse. This is the fastest path and the best one for osu! and other games. Drawing programs want more: pressure, tilt and hover. Windows Ink gives them that.

## Turn it on

1. Open the **Drawing** tab.
2. In **Pen Mode**, choose **Windows Ink**.

You can also use the tray icon: right-click it and check **Windows Ink (Drawing)**. The change is instant. You do not need to restart anything.

To go back to the mouse path for a game, choose **Mouse** in the same place.

## What changes

| | Off (default) | On |
| --- | --- | --- |
| How the pen is sent | Mouse events (`SendInput`) | A virtual Windows Ink pen |
| Pressure and tilt | Not sent | Sent |
| Hover | Cursor only | Real pen hover |
| Tip touch | Left mouse button | Pen contact |
| Eraser end of the pen | Not sent | Sent as a real eraser |

The pressure curve, the **Disable Pressure** and **Disable Tilt** switches and the tip threshold apply in both modes.

## The Drawing tab

- **Pen Mode:** choose **Mouse** or **Windows Ink**.
- **Test Pad:** draw on it to check what a program receives. With Windows Ink on, the type reads `pen`, the line gets thicker with pressure, you can read the tilt, and the eraser end of the pen erases. With it off, the type reads `mouse` and the pressure stays flat.

## The pressure curve

The curve and the other pen settings are on the **Pen Settings** tab, because they apply to the pen in both modes.

- **Pressure Curve:** pick Linear, Exponential or Custom. With Custom, drag the points, click the graph to add one (up to 32), and double-click a point to remove it. Use the arrow keys on a selected point for small moves.
- **Live dot:** while the pen is near the tablet, a dot on the graph shows your pressure and what the curve reports for it. Dashed lines lead to the two axes, and the figures in the corners show the same numbers (**Raw**, **Reported**) and whether the pen is touching or hovering. The dot is hollow while the pen hovers and solid once the tip touches. Press and watch the dot to see how the curve feels before you draw.
- **Disable Pressure:** the pen reports full pressure while it touches the tablet. It still has to touch to draw.

## Good to know

- It needs **Windows 10 version 1809** or later. If Windows refuses to create the pen, the driver logs a warning and keeps using the mouse.
- It only works in **Absolute** mode. In Relative mode the pen keeps acting as a mouse.
- The setting is saved in the profile, like the other settings.
- Pen buttons and tablet keys still send the mouse buttons and keys you bound to them.
- The eraser end is sent only if your tablet reports it. Not every driver does yet.
- Programs that read tablets through WinTab only (some older versions of Photoshop) do not see this pen. In Photoshop, choose **Windows Ink** in the tablet settings.
