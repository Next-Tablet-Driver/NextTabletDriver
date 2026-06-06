# Theme Creation Guide for NextTabletDriver

NextTabletDriver supports 100% customizable themes via JSON files.
You have absolute control over the color palette: nothing is hardcoded. You control semantic colors (statuses, console), backgrounds for all text fields and dropdowns, as well as the interface structure (corner rounding, spacing, borders).

---

## Quick Start

1. Download the [example theme](https://raw.githubusercontent.com/Next-Tablet-Driver/NextTabletDriver-Themes/refs/heads/main/00%20EXAMPLE/theme.json) and place it into your `themes/` folder.
2. Open NextTabletDriver.
3. Go to **Settings > Application Theme > Import Theme (.json)**.
4. Select your file. The theme applies **immediately**.

> If your JSON file contains a syntax error, the application will automatically revert to the default dark theme.

---

## Full Structure of a `theme.json`

```json
{
  "metadata": {
    "name": "My Theme",
    "author": "Your Name",
    "version": "1.0",
    "update_url": "https://raw.githubusercontent.com/Next-Tablet-Driver/NextTabletDriver-Themes/refs/heads/main/00%20EXAMPLE/theme.json"
  },
  "colors": {
    "dark_mode": true,

    "panel_bg":         "#12121c",
    "window_bg":        "#0a0a0f",
    "text_color":        "#a0a0b5",
    "strong_text_color": "#ffffff",
    "accent_color":      "#ff007f",
    "border_color":      "#2c2c3d",
    "widget_bg":         "#1a1a24",
    "widget_hover":      "#262635",
    "widget_active":     "#ff007f",

    "success_color": "#a6e3a1",
    "warning_color": "#f9e2af",
    "error_color":   "#f38ba8",
    "info_color":    "#89b4fa",
    "playfield_color": "#ff69b4"
  },
  "spacing": {
    "corner_radius":    8.0,
    "item_spacing_x":  10.0,
    "item_spacing_y":  10.0,
    "button_padding_x": 12.0,
    "button_padding_y":  6.0,
    "border_width":      1.0
  }
}
```

---

## Field Reference

### `metadata`

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | ✅ | Name of the theme displayed in the settings. |
| `author` | string | ✅ | Your name / alias. |
| `version` | string | ✅ | Theme version (e.g., `"1.0"`). Useful for managing your updates. |
| `update_url` | string | ❌ | Direct URL to the raw `.json` file (e.g., GitHub Raw). **Reserved for a future automatic theme update system** — documenting this field now allows activating it without a breaking change later. |

---

### `colors`

All colors use **hexadecimal** format:
- `"#RRGGBB"` → opaque color (e.g., `"#ff0000"` for red)
- `"#RRGGBBAA"` → color with alpha channel (e.g., `"#ff000080"` for 50% red)

#### Main Palette (Required)

| Field | Description |
|---|---|
| `dark_mode` | `true` or `false`. Indicates if the theme is dark or light. Egui uses this flag to adjust internal micro-details (shadows, disabled states, etc.). |
| `panel_bg` | Background of cards, groups, and settings panels. |
| `window_bg` | Global background of the application window. |
| `text_color` | Normal text (labels, descriptions). |
| `strong_text_color` | Bold text, titles, and hovered text. |
| `accent_color` | Main theme color: checkboxes, selections, active widgets, sliders. |
| `border_color` | Borders between cards and panels. |
| `widget_bg` | Background of all buttons, text inputs, and combo boxes. |
| `widget_hover` | Background of these same elements in hovered state. |
| `widget_active` | Background of these same elements in clicked/active state. |

#### Semantic Colors (Optional)

These colors control the status indicators throughout the interface. If omitted, default values suitable for `dark_mode` are used.

| Field | Where it's used |
|---|---|
| `success_color` | "RUNNING" badge, "NEW" items in the Release tab, Parser latency, stylus touch point. |
| `warning_color` | "FIX" items in the Release tab, UI Sync latency, Report Rate card in the Debugger. |
| `error_color` | "STOPPED" badge, "DEL" items in the Release tab. |
| `info_color` | "IMP" items in the Release tab, console INFO logs, HID Read latency, Pressure card in the Debugger. |
| `playfield_color` | Color of the osu! Playfield rectangle in the tablet preview. Default: osu! pink (`#ff69b4`). Use `#RRGGBBAA` to adjust background transparency (e.g., `#ff69b480`). |

---

### `spacing` *(completely optional block)*

Omitting this block keeps the default egui spacing.

| Field | Type | Description |
|---|---|---|
| `corner_radius` | float | Corner rounding (in pixels) applied to all widgets, buttons, panels, and the window. |
| `item_spacing_x` | float | Horizontal space between two consecutive elements. |
| `item_spacing_y` | float | Vertical space between two consecutive elements. |
| `button_padding_x` | float | Horizontal padding inside a button. |
| `button_padding_y` | float | Vertical padding inside a button. |
| `border_width` | float | Border thickness. Panels automatically use half of this value. |

---

## Design Tips

- **Contrasts**: Ensure `text_color` is readable on `panel_bg` and `window_bg`.
- **Consistency**: `widget_active` and `accent_color` can be identical for a simple theme.
- **Transparencies**: Use `#RRGGBBAA` format for subtle overlay effects on widgets.
- **Light Theme**: Remember to set `dark_mode: false` and choose light `panel_bg`/`window_bg` colors.

---

## Sharing Your Theme

1. Publish your `theme.json` on GitHub (or any host).
2. Enter the **GitHub Raw** link in `update_url` (example: `https://raw.githubusercontent.com/YourUser/repo/main/theme.json`).
3. Share the direct URL to the `.json` so users can import it with one click via **Import Theme**.

> The automatic update system via `update_url` is planned for a future release.