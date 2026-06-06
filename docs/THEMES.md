# Guide de Création de Thèmes pour NextTabletDriver

NextTabletDriver supporte des thèmes 100% personnalisables via des fichiers JSON.
Vous avez un contrôle absolu sur la palette de couleurs : rien n'est hardcodé. Vous contrôlez les couleurs sémantiques (statuts, console), les fonds de tous les champs de texte et sélecteurs, ainsi que la structure de l'interface (arrondi, espacements, bordures).

---

## Démarrage rapide

1. Copiez `example-theme.json` (à la racine du projet) dans votre dossier `themes/`.
2. Ouvrez NextTabletDriver.
3. Allez dans **Settings → Application Theme → Import Theme (.json)**.
4. Sélectionnez votre fichier. Le thème s'applique **immédiatement**.

> Si votre fichier JSON contient une erreur de syntaxe, l'application revient automatiquement au thème sombre par défaut.

---

## Structure complète d'un `theme.json`

```json
{
  "metadata": {
    "name": "Mon Thème",
    "author": "Votre Nom",
    "version": "1.0",
    "update_url": "https://raw.githubusercontent.com/.../theme.json"
  },
  "colors": {
    "dark_mode": true,

    "panel_bg":          "#12121c",
    "window_bg":         "#0a0a0f",
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

## Référence des champs

### `metadata`

| Champ | Type | Obligatoire | Description |
|---|---|---|---|
| `name` | string | ✅ | Nom du thème affiché dans les paramètres. |
| `author` | string | ✅ | Votre nom / pseudo. |
| `version` | string | ✅ | Version du thème (ex : `"1.0"`). Utile pour gérer vos mises à jour. |
| `update_url` | string | ❌ | URL directe vers le fichier `.json` brut (ex: GitHub Raw). **Réservé pour un futur système de mise à jour automatique des thèmes** — documenter ce champ maintenant permet de l'activer sans breaking change plus tard. |

---

### `colors`

Toutes les couleurs utilisent le format **hexadécimal** :
- `"#RRGGBB"` → couleur opaque (ex: `"#ff0000"` pour rouge)
- `"#RRGGBBAA"` → couleur avec canal alpha (ex: `"#ff000080"` pour rouge à 50%)

#### Palette principale (obligatoire)

| Champ | Description |
|---|---|
| `dark_mode` | `true` ou `false`. Indique si le thème est sombre ou clair. Egui utilise ce flag pour ajuster des micro-détails internes (ombres, états désactivés, etc.). |
| `panel_bg` | Fond des cartes, groupes, et panneaux de paramètres. |
| `window_bg` | Fond global de la fenêtre de l'application. |
| `text_color` | Texte normal (labels, descriptions). |
| `strong_text_color` | Texte en gras, titres, et texte survolé. |
| `accent_color` | Couleur principale du thème : checkboxes, sélections, widgets actifs, curseurs. |
| `border_color` | Bordures entre cartes et panneaux. |
| `widget_bg` | Fond de tous les boutons, champs de saisie (inputs), et listes déroulantes (combo boxes). |
| `widget_hover` | Fond de ces mêmes éléments à l'état survolé. |
| `widget_active` | Fond de ces mêmes éléments à l'état cliqué/actif. |

#### Couleurs sémantiques (optionnel)

Ces couleurs contrôlent les indicateurs de statut dans toute l'interface. Si omises, des valeurs par défaut adaptées à `dark_mode` sont utilisées.

| Champ | Où c'est utilisé |
|---|---|
| `success_color` | Badge "RUNNING", items "NEW" dans la Release tab, latence Parser, point de contact du stylet. |
| `warning_color` | Items "FIX" dans la Release tab, latence UI Sync, carte Report Rate dans le Debugger. |
| `error_color` | Badge "STOPPED", items "DEL" dans la Release tab. |
| `info_color` | Items "IMP" dans la Release tab, logs INFO de la console, latence HID Read, carte Pressure dans le Debugger. |
| `playfield_color` | Couleur du rectangle osu! Playfield dans la prévisualisation de la tablette. Par défaut : rose osu! (`#ff69b4`). Utilisez `#RRGGBBAA` pour ajuster la transparence du fond (ex: `#ff69b480`). |

---

### `spacing` *(bloc entièrement optionnel)*

Omettre ce bloc conserve les espacements par défaut d'egui.

| Champ | Type | Description |
|---|---|---|
| `corner_radius` | float | Arrondi (en pixels) appliqué à tous les widgets, boutons, panneaux et à la fenêtre. |
| `item_spacing_x` | float | Espace horizontal entre deux éléments consécutifs. |
| `item_spacing_y` | float | Espace vertical entre deux éléments consécutifs. |
| `button_padding_x` | float | Padding horizontal à l'intérieur d'un bouton. |
| `button_padding_y` | float | Padding vertical à l'intérieur d'un bouton. |
| `border_width` | float | Épaisseur des bordures. Les panneaux utilisent automatiquement la moitié de cette valeur. |

---

## Conseils de design

- **Contrastes** : assurez-vous que `text_color` est lisible sur `panel_bg` et `window_bg`.
- **Cohérence** : `widget_active` et `accent_color` peuvent être identiques pour un thème simple.
- **Transparences** : utilisez le format `#RRGGBBAA` pour des effets de superposition subtils sur les widgets.
- **Thème clair** : pensez à mettre `dark_mode: false` et à choisir des couleurs `panel_bg`/`window_bg` claires.

---

## Partager votre thème

1. Publiez votre `theme.json` sur GitHub (ou tout hébergeur).
2. Renseignez le lien **GitHub Raw** dans `update_url` (exemple : `https://raw.githubusercontent.com/VotreUser/repo/main/theme.json`).
3. Partagez l'URL directe du `.json` pour que les utilisateurs puissent l'importer en un clic via **Import Theme**.

> Le système de mise à jour automatique via `update_url` est prévu pour une prochaine version.
