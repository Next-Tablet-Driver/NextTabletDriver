{
  "metadata": {
    "name": "Neon Genesis",
    "author": "Designer123",
    "version": "1.1",

    // (Optional) Direct link to the raw .json file on GitHub (or any CDN).
    // This field is reserved for a future automatic theme-update system.
    // Leave it out if you don't publish your theme online.
    "update_url": "https://raw.githubusercontent.com/Designer123/my-themes/main/neon-genesis/theme.json"
  },

  "colors": {
    // Set to true for dark themes, false for light themes.
    // This tells egui to start from the correct base palette for
    // internal micro-details (shadows, disabled state tints, etc.).
    "dark_mode": true,

    // ── Core palette ──────────────────────────────────────────────
    "panel_bg":          "#12121c",   // Background of cards, groups, side-panels
    "window_bg":         "#0a0a0f",   // Global application background
    "text_color":        "#a0a0b5",   // Normal label text
    "strong_text_color": "#ffffff",   // Bold text, headings, hovered labels
    "accent_color":      "#ff007f",   // Checkboxes, selections, focused widgets
    "border_color":      "#2c2c3d",   // Borders between cards and panels

    // ── Widget states ─────────────────────────────────────────────
    "widget_bg":     "#1a1a24",   // Button / input background (idle)
    "widget_hover":  "#262635",   // Button / input background (hovered)
    "widget_active": "#ff007f",   // Button / input background (pressed)

    // ── Semantic status colors (all optional) ─────────────────────
    // These are used throughout the UI for:
    //   success  → "RUNNING" badge, NEW items in Release tab, Parser latency, pen-contact dot
    //   warning  → "FIX" items in Release tab, UI-Sync latency, Report-Rate card, console WARN logs
    //   error    → "STOPPED" badge, DEL items in Release tab, console ERROR logs
    //   info     → "IMP" items in Release tab, HID-Read latency, Pressure card, console INFO logs
    //   playfield→ Color of the osu! playfield area preview in the Display/Tablet panel
    //
    // If omitted, sensible defaults are used based on `dark_mode`.
    "success_color": "#a6e3a1",
    "warning_color": "#f9e2af",
    "error_color":   "#f38ba8",
    "info_color":    "#89b4fa",
    "playfield_color": "#ff69b4"
  },

  // (Optional block) – omit entirely to keep default egui spacing.
  "spacing": {
    // Rounding applied to all widgets, buttons, panels, and the window itself.
    "corner_radius": 12.0,

    // Horizontal / vertical gap between consecutive UI elements.
    "item_spacing_x": 12.0,
    "item_spacing_y": 12.0,

    // Internal padding inside buttons.
    "button_padding_x": 16.0,
    "button_padding_y":  8.0,

    // Stroke width for borders (panels use half this value).
    "border_width": 2.0
  }
}
