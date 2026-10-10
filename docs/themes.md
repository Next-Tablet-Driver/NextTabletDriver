# Themes

A theme sets the colors, corner shapes and fonts of the app. You can pick one that comes with the app, or write your own in a small JSON file.

## Pick a theme

Open **Settings → Themes** and choose a theme in the **Current Theme** list.

| Theme | Style |
| --- | --- |
| System | Follows your operating system: dark or light. |
| Dark | The default dark theme. |
| Light | A light theme. |
| Catppuccin Latte | Light, from the [Catppuccin](https://catppuccin.com) palette. |
| Catppuccin Frappe | Dark, from Catppuccin. |
| Catppuccin Macchiato | Dark, from Catppuccin. |
| Catppuccin Mocha | Dark, from Catppuccin. |

Your own themes appear in the same list, under **My themes**.

## Add a theme file

1. Get a theme file (a `.json` file), or [write one](#write-your-own-theme).
2. In **Settings → Themes**, choose **Import theme...** and select the file. The app copies it to the themes folder and selects it.

You can also put the file in the themes folder yourself. **Open Themes Folder** shows it (it is the `Themes` folder in your `Settings` folder, see [Where your files are](getting-started.md#where-your-files-are)). Then choose **Reload**.

Other buttons on the same screen:

| Button | What it does |
| --- | --- |
| **Reload** | Reads the themes folder again. |
| **Copy as template** | Copies a complete theme file, with the colors you see now, to the clipboard. |
| **Delete** | Deletes the selected theme file. Only shown for your own themes. |
| **Browse Online Themes** | Opens a page with themes made by others. At the moment this button opens a page that does not exist. Use the [theme repository](https://github.com/Next-Tablet-Driver/NextTabletDriver-Themes) directly. |

If a file has a mistake, it is not used. The screen lists it under **Themes that could not be loaded**, with the reason.

## Write your own theme

The easiest way:

1. Choose the theme that is closest to what you want.
2. Choose **Copy as template**.
3. Create a new file in the themes folder, for example `midnight-pink.json`, and paste.
4. Change the values you like, and delete the lines you do not want to change.
5. Save, then choose **Reload** in the app and select your theme.

A theme file only needs the tokens that you want to change. Everything else comes from the *base* theme.

### The file

```json
{
  "schema": 1,
  "metadata": {
    "name": "Midnight Pink",
    "author": "You",
    "version": "1.0",
    "description": "A dark theme with a pink accent."
  },
  "base": "dark",
  "tokens": {
    "bg-app": "#101018",
    "bg-panel": "#181826",
    "text-main": "#e6e6f0",
    "accent": "#ff4d8d",
    "accent-hover": "#ff7aa9",
    "text-on-accent": "#101018",
    "radius-md": "8px"
  }
}
```

| Key | Required | Meaning |
| --- | --- | --- |
| `schema` | Yes | The file format version. It must be `1`. |
| `metadata.name` | Yes | The name shown in the list. |
| `metadata.author`, `version`, `description` | No | Information for people. |
| `base` | No | `"dark"` or `"light"`. Tokens that you leave out come from this theme. The default is `"dark"`. |
| `tokens` | Yes | The values to change. Each key is a token name, written without the leading `--`. |

The name of the file gives the theme its id: lowercase letters, digits, `-` and `_`, up to 64 characters. If two files end up with the same id, only the first one is used.

### Token values

Every token has a type. The app checks each value, and refuses a theme that has a wrong one.

| Type | Examples | Rules |
| --- | --- | --- |
| Color | `#f90`, `#ff9900`, `#ff990080`, `rgb(255 153 0)`, `hsl(36 100% 50%)`, `transparent` | `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`, `hsla()` with plain numbers, or `transparent`. |
| RGB triplet | `255 255 255` | Three numbers from 0 to 255, separated by spaces. |
| Length | `6px`, `0.5rem` | A number with `px`, `rem` or `em`. Corner radii are limited to 32 px. |
| Number | `0.5` | A plain number in the allowed range. |
| Font | `Segoe UI, sans-serif` | Up to 8 font names, separated by commas. Only fonts installed on the PC work. The app cannot load font files. |

A theme file is *data only*. It cannot run code or load files from the internet.

### Limits

A theme file can be at most 64 KB. It can have at most 200 tokens, and each value is at most 200 characters. The folder shows at most 100 theme files.

### Warnings

- A token name that does not exist is ignored, with a warning.
- Hard-to-read colors give a warning, but the theme still works. The app checks these pairs: normal text on the window background and on panels (at least 4.5:1), secondary text on panels (at least 3:1), and text on accent buttons (at least 4.5:1).

### What you can change

You can change the look, not the layout. Spacing, font sizes and layers cannot be themed, because changing them could break the screen. This is the complete list of tokens (46). The same list is in your template when you choose **Copy as template**.

**Surfaces**

| Token | Type | Used for |
| --- | --- | --- |
| `bg-app` | color | Window background. |
| `bg-panel` | color | Cards, groups, menus and panels. |
| `bg-panel-hover` | color | Panels and menu items under the pointer. |
| `bg-input` | color | Background of inputs and dropdown triggers. |
| `bg-tab-active` | color | Background of the active tab. |

**Text**

| Token | Type | Used for |
| --- | --- | --- |
| `text-main` | color | Normal text. |
| `text-muted` | color | Secondary text, hints and icons. |
| `text-active` | color | Titles and hovered text. |
| `text-strong` | color | Names and headings one step above normal text. |
| `text-emphasis` | color | The strongest text (statistics, highlighted names). |
| `text-on-accent` | color | Text drawn on an accent-colored background. |
| `text-number` | color | Numeric values in inputs. |

**Accent and borders**

| Token | Type | Used for |
| --- | --- | --- |
| `accent` | color | Main theme color: selections, checkboxes, active widgets. |
| `accent-hover` | color | Accent while hovered. |
| `border` | color | Borders between cards and panels. |
| `border-hover` | color | Borders of interactive cards while hovered. |

**Status**

| Token | Type | Used for |
| --- | --- | --- |
| `info` | color | Informational status. |
| `warning` | color | Warning status. |
| `error` | color | Error status (log levels, validation). |
| `danger` | color | Destructive actions. |
| `danger-hover` | color | Destructive actions while hovered. |
| `success` | color | Success or running status. |
| `debug` | color | Debug-level log lines. |

**Overlays**

| Token | Type | Used for |
| --- | --- | --- |
| `overlay-rgb` | RGB triplet | Color of translucent highlights (white on dark themes, black on light themes). |
| `shade-rgb` | RGB triplet | Color of translucent shadows and darkening. |

**Inputs**

| Token | Type | Used for |
| --- | --- | --- |
| `input-bg` | color | Input field background. |
| `input-bg-hover` | color | Input field background while hovered. |
| `input-bg-focus` | color | Input field background while focused. |
| `input-border` | color | Input field border. |
| `input-border-hover` | color | Input field border while hovered. |

**Tablet and display preview**

| Token | Type | Used for |
| --- | --- | --- |
| `area-display` | color | Display area in the output preview. |
| `area-tablet` | color | Tablet area in the output preview. |
| `area-border-main` | color | Outline of the tablet area. |
| `area-border-pink` | color | Outline of the osu! playfield. |
| `area-playfield` | color | Fill of the osu! playfield rectangle. |
| `area-contrast` | color | Text and handles drawn over the preview areas. |

**Console and title bar**

| Token | Type | Used for |
| --- | --- | --- |
| `console-text` | color | Log message text. |
| `titlebar-close-bg` | color | Close button background while hovered. |
| `titlebar-close-fg` | color | Close button icon while hovered. |

**Shape and fonts**

| Token | Type | Used for |
| --- | --- | --- |
| `radius-sm` | length (up to 32px) | Smallest corner radius. |
| `radius-md` | length (up to 32px) | Default corner radius (buttons, inputs). |
| `radius-lg` | length (up to 32px) | Large corner radius (panels, menus). |
| `radius-xl` | length (up to 32px) | Extra large corner radius (cards). |
| `disabled-opacity` | number (0 to 1) | Opacity of disabled controls. |
| `font-sans` | font | Interface font. |
| `font-mono` | font | Monospace font (console, hashes). |

## For developers

The token list is defined in one place: [`frontend/src/lib/theme/tokens.ts`](../frontend/src/lib/theme/tokens.ts). The checks are in `validate.ts`, and the default values are in `frontend/src/styles/tokens.css`. A test fails if the list and the CSS file drift apart. If you add a token, update all three and this page.
