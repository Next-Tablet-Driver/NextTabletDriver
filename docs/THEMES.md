# Themes

NextTabletDriver's interface is themable. You can pick one of the built-in themes, or import
a theme file made by you or someone else. A theme only changes how the app **looks** (colors,
corner rounding, fonts); it cannot change the layout or run code.

---

## Using a theme

Open **Settings > Themes**.

| Button | What it does |
|---|---|
| **Current Theme** | Switches theme immediately. The choice is saved with your profile. |
| **Import theme...** | Pick a `.json` theme file. It is copied to the themes folder and selected. |
| **Reload** | Re-reads the themes folder (after you edit a file or add one by hand). |
| **Open Themes Folder** | Opens `Settings/Themes/` in your file explorer. |
| **Copy as template** | Copies the theme currently on screen as a complete theme file: the easiest way to start your own. |
| **Delete** | Removes the selected theme file (only for themes you imported). |

If a file in the folder is not a valid theme, it is listed under **Themes that could not be
loaded** with the exact reason. If the theme you selected disappears, the app falls back to Dark
and tells you why: it never ends up unreadable.

### Built-in themes

| Theme | Description |
|---|---|
| `System` | Follows your operating system's light/dark setting, live. |
| `Dark` | The default. |
| `Light` | Light palette. |
| `CatppuccinLatte` | [Catppuccin](https://catppuccin.com/palette) Latte (light). |
| `CatppuccinFrappe` | Catppuccin Frappe (dark). |
| `CatppuccinMacchiato` | Catppuccin Macchiato (dark). |
| `CatppuccinMocha` | Catppuccin Mocha (dark). |

---

## Creating a theme

1. In **Settings > Themes**, select the built-in theme closest to what you want and click
   **Copy as template**.
2. Paste into a new file, for example `my-theme.json`, in the themes folder
   (**Open Themes Folder**).
3. Change `metadata.name`, then edit or delete tokens. **A token you remove falls back to
   the `base` theme**, so a theme can be as small as one color.
4. Click **Reload**, then pick your theme in the list. Edit, reload, repeat.

Two ready-made examples are in [`docs/themes/examples/`](themes/examples).

### File format

```json
{
  "$schema": "https://raw.githubusercontent.com/Next-Tablet-Driver/NextTabletDriver/master/docs/themes/theme.schema.json",
  "schema": 1,
  "metadata": {
    "name": "Neon Night",
    "author": "You",
    "version": "1.0",
    "description": "Dark purple with a hot pink accent."
  },
  "base": "dark",
  "tokens": {
    "bg-app": "#0d0b1a",
    "bg-panel": "#16122b",
    "accent": "#c4147d",
    "accent-hover": "#e0239a",
    "radius-md": "8px"
  }
}
```

| Field | Required | Description |
|---|---|---|
| `schema` | yes | Format version. Currently `1`. |
| `metadata.name` | yes | Name shown in the theme list (up to 200 characters). |
| `metadata.author`, `version`, `description` | no | Shown for information. |
| `base` | recommended | `dark` or `light`. The built-in theme every token you do not set falls back to; also decides `color-scheme` (native scrollbars, form controls). Defaults to `dark` with a warning. |
| `tokens` | yes | The overrides. Keys are the token names below. |
| `$schema` | no | Lets editors such as VS Code autocomplete token names and check values. |

### Rules

A theme file is treated as untrusted data:

- Only the tokens listed below are accepted. An unknown token is **ignored with a warning**
  (so themes written for a newer version still load).
- Each value must match the type of its token. Colors are `#rgb`, `#rrggbb`, `#rrggbbaa`,
  `rgb()`, `rgba()`, `hsl()`, `hsla()` or `transparent`. Anything that looks like CSS code
  (`url(...)`, `var(...)`, `;`, braces, comments) is rejected.
- At most 64 KB per file, 200 tokens, 200 characters per value, 100 theme files.
- Fonts must be installed on the user's system; web fonts cannot be loaded.

Not everything is themable on purpose: spacing, font sizes, animation timing and layering are fixed
so that a theme can never break the layout.

### Design tips

- **Contrast.** The app warns you under the theme list when text on the window, panels or accent
  buttons has a contrast ratio under 4.5:1 (3:1 for secondary text). It never blocks a theme.
- **Accent.** `accent` colors checkboxes, selections and the active tab. `text-on-accent` is the
  text drawn on it: set both together.
- **Overlays.** Hover and divider effects are translucent layers of `overlay-rgb`. Use white
  (`255 255 255`) on dark themes and black (`0 0 0`) on light ones, otherwise highlights are
  invisible.
- **Statuses.** `info`, `warning`, `error`, `danger`, `success` and `debug` also drive the tinted
  backgrounds of log filters and badges, so one color is enough.

---

## Token reference

Names are the CSS variable names without the leading `--`.

### Surfaces

| Token | Type | Description |
|---|---|---|
| `bg-app` | color | Window background. |
| `bg-panel` | color | Cards, groups, menus and panels. |
| `bg-panel-hover` | color | Panels and menu items under the pointer. |
| `bg-input` | color | Background of inputs and dropdown triggers. |
| `bg-tab-active` | color | Background of the active tab. |

### Text

| Token | Type | Description |
|---|---|---|
| `text-main` | color | Normal text. |
| `text-muted` | color | Secondary text, hints and icons. |
| `text-active` | color | Titles and hovered text. |
| `text-strong` | color | Names and headings one step above normal text. |
| `text-emphasis` | color | The strongest text (statistics, highlighted names). |
| `text-on-accent` | color | Text drawn on an accent-colored background. |
| `text-number` | color | Numeric values in inputs. |

### Accent

| Token | Type | Description |
|---|---|---|
| `accent` | color | Main theme color: selections, checkboxes, active widgets. |
| `accent-hover` | color | Accent while hovered. |

### Borders

| Token | Type | Description |
|---|---|---|
| `border` | color | Borders between cards and panels. |
| `border-hover` | color | Borders of interactive cards while hovered. |

### Status colors

| Token | Type | Description |
|---|---|---|
| `info` | color | Informational status. |
| `warning` | color | Warning status. |
| `error` | color | Error status (log levels, validation). |
| `danger` | color | Destructive actions. |
| `danger-hover` | color | Destructive actions while hovered. |
| `success` | color | Success / running status. |
| `debug` | color | Debug-level log lines. |

### Overlays

| Token | Type | Description |
|---|---|---|
| `overlay-rgb` | rgb-triplet | Channels of the color used for translucent highlights (white on dark themes, black on light themes). |
| `shade-rgb` | rgb-triplet | Channels of the color used for translucent shadows and darkening. |

### Inputs

| Token | Type | Description |
|---|---|---|
| `input-bg` | color | Input field background. |
| `input-bg-hover` | color | Input field background while hovered. |
| `input-bg-focus` | color | Input field background while focused. |
| `input-border` | color | Input field border. |
| `input-border-hover` | color | Input field border while hovered. |

### Tablet and display preview

| Token | Type | Description |
|---|---|---|
| `area-display` | color | Display area in the output preview. |
| `area-tablet` | color | Tablet area in the output preview. |
| `area-border-main` | color | Outline of the tablet area. |
| `area-border-pink` | color | Outline of the osu! playfield. |
| `area-playfield` | color | Fill of the osu! playfield rectangle. |
| `area-contrast` | color | Text and handles drawn over the preview areas. |

### Console

| Token | Type | Description |
|---|---|---|
| `console-text` | color | Log message text. |

### Title bar

| Token | Type | Description |
|---|---|---|
| `titlebar-close-bg` | color | Close button background while hovered. |
| `titlebar-close-fg` | color | Close button icon while hovered. |

### Shape

| Token | Type | Description |
|---|---|---|
| `radius-sm` | length (max 32px) | Smallest corner radius. |
| `radius-md` | length (max 32px) | Default corner radius (buttons, inputs). |
| `radius-lg` | length (max 32px) | Large corner radius (panels, menus). |
| `radius-xl` | length (max 32px) | Extra large corner radius (cards). |
| `disabled-opacity` | number (0-1) | Opacity of disabled controls (0 to 1). |

### Typography

| Token | Type | Description |
|---|---|---|
| `font-sans` | font-family | Interface font stack. Only installed fonts can be used. |
| `font-mono` | font-family | Monospace font stack (console, hashes). |

---

## Converting a theme from the old (egui) format

Themes made for the previous native interface (`colors` / `spacing` keys, stored as
`Themes/<Name>/theme.json`) are not read any more: they have to be converted once. A worked
example is [`examples/neon.json`](themes/examples/neon.json), converted from the original Neon theme.

| Old key | New token(s) |
|---|---|
| `dark_mode` | `base`: `"dark"` or `"light"` |
| `window_bg` | `bg-app` |
| `panel_bg` | `bg-panel` |
| `text_color` | `text-main` (and `console-text`) |
| `strong_text_color` | `text-active`, `text-emphasis` |
| `accent_color` | `accent`, plus a lighter `accent-hover` |
| `border_color` | `border`, `input-border` |
| `widget_bg` | `input-bg`, `bg-input` |
| `widget_hover` | `input-bg-hover`, `input-bg-focus`, `bg-panel-hover` |
| `widget_active` | none: the active tab and selections now follow `accent` |
| `success_color`, `warning_color`, `error_color`, `info_color` | `success`, `warning`, `error` (also `danger`), `info` |
| `playfield_color` + `playfield_opacity` | `area-playfield` as one `rgba()` color, `area-border-pink` for the outline |
| `spacing.corner_radius` | `radius-sm`, `radius-md`, `radius-lg`, `radius-xl` |
| `spacing.item_spacing_*`, `button_padding_*`, `border_width` | dropped: spacing is no longer themable |

Also: put the file directly in the themes folder (`Themes/neon.json`, no sub-folder), add
`"schema": 1`, move `metadata.update_url` out (it is not used), and fill the tokens the old format
had no equivalent for (`text-muted`, `danger-hover`, `debug`, `overlay-rgb`...). The quickest way to
see them all is **Copy as template** from the built-in theme that is closest to yours.

---

## Sharing a theme

Share the `.json` file. Anyone can import it with **Import theme...**; nothing else is needed. Please
keep `metadata.author` filled in so people know who to thank.

---

## For contributors: adding a built-in theme

Built-in themes are plain CSS in [`frontend/src/styles/themes.css`](../frontend/src/styles/themes.css):
one `:root.theme-<id>` block that overrides the semantic tokens. To add one:

1. Add the block (copy an existing one: every core token must be defined, a test checks it).
2. Register it in `BUILTIN_THEMES` in `frontend/src/lib/theme/apply.ts`.
3. Add the matching variant to `ThemePreference` in `src/core/config/models.rs` if it is not there.
4. Document it in the table above and open a pull request with a screenshot.

The token contract lives in `frontend/src/lib/theme/tokens.ts`; this page, the JSON Schema
(`docs/themes/theme.schema.json`) and the validator are all tested against it.
