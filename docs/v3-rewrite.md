# NextTabletDriver v3 — Plan de refonte du backend

> **Statut** : document de cadrage. Il décide **pourquoi** on refait le backend, **comment** on le fait,
> et surtout **ce qu'il ne faut pas faire**.
>
> **Public** : contributeurs humains **et** assistants IA (LLMs). Si tu es un LLM qui travaille sur ce
> dépôt : la section [« Ce qu'il ne faut PAS faire »](#9-ce-quil-ne-faut-pas-faire) est **normative**.
> Une règle qui commence par « NE PAS » est une interdiction, pas une suggestion. En cas de doute,
> arrête-toi et demande, plutôt que de « faire au mieux ».

---

## Sommaire

0. [Résumé des décisions](#0-résumé-des-décisions)
1. [Diagnostic de la v2](#1-diagnostic-de-la-v2)
2. [Nouveau dépôt ou même dépôt ?](#2-nouveau-dépôt-ou-même-dépôt-)
3. [Réécriture d'un coup ou migration progressive ?](#3-réécriture-dun-coup-ou-migration-progressive-)
4. [Architecture cible : le workspace](#4-architecture-cible--le-workspace)
5. [Le contrat backend ↔ frontend](#5-le-contrat-backend--frontend)
6. [Le frontend : on le garde, mais sa couche de données change](#6-le-frontend--on-le-garde-mais-sa-couche-de-données-change)
7. [Plan de migration par phases](#7-plan-de-migration-par-phases)
8. [Règles par crate](#8-règles-par-crate)
9. [Ce qu'il ne faut PAS faire](#9-ce-quil-ne-faut-pas-faire)
10. [Checklist de PR](#10-checklist-de-pr)
11. [Problèmes trouvés en dehors de l'architecture](#11-problèmes-trouvés-en-dehors-de-larchitecture)

---

## 0. Résumé des décisions

| Question | Décision | Raison en une ligne |
|---|---|---|
| Refaire l'architecture du backend ? | **Oui** | Les modules ont des dépendances circulaires qu'un seul crate ne peut pas empêcher. |
| Nouveau dépôt « NextTabletDriver v3 » ? | **Non. Même dépôt, version 3.0.0** | L'auto-update des utilisateurs v2 pointe en dur vers ce dépôt. Un nouveau dépôt les abandonne sur la v2. |
| Réécrire d'un coup (big bang) ? | **Non. Migration progressive** | Environ 30 parsers de tablettes et un pipeline sensible à la latence : la parité ne se réécrit pas d'un coup. |
| Garder le frontend ? | **Oui** | Svelte 5 + TS + tests + design system + E2E : la base est saine. |
| Changer quelque chose côté frontend ? | **Oui, la couche de données** | Types écrits à la main, `Config` fourre-tout, chaînes magiques, `invoke` hors couche. |
| Combien de crates ? | **Une douzaine, pas plus** | Assez pour imposer le sens des dépendances, pas assez pour devenir de la bureaucratie. |

---

## 1. Diagnostic de la v2

### 1.1 Ce qui va bien (à ne pas casser)

- Un **pipeline de traitement déjà assez pur** (`src/engine/pipeline/transformer.rs`, `projector.rs`,
  `src/core/math/*`). On le testerait facilement s'il était isolé.
- Un **ABI de plugins** séparé (`sdk/plugin_api`), une **vérification de confiance** des plugins
  (`src/engine/plugins/trust.rs`, minisign) et des plugins réels (`plugins/*`).
- Les **descripteurs de tablettes en données** (`tablets/`) plutôt qu'en code.
- La **qualité** : clippy, `deny.toml`, des seuils de couverture (`coverage-thresholds.json`),
  du fuzzing (`qa/fuzz`), des E2E Playwright et un smoke test natif.
- Un frontend structuré par **features** (`frontend/src/lib/features/*`).

### 1.2 Ce qui ne va pas

Le backend (≈ 23 500 lignes) vit dans **un seul crate** `next_tablet_driver`. À l'intérieur d'un
crate, `crate::n_importe_quoi` est toujours accessible, donc **rien n'empêche les mauvaises
dépendances**. Voici celles qui existent aujourd'hui :

| Violation | Où | Pourquoi c'est grave |
|---|---|---|
| `drivers` → `engine` | `src/drivers/parsers/wacom/intuos_v1.rs:3`, `pl.rs:3` importent `engine::state::{LockRecoveryExt, WriteRecoverExt}` | Un parser (bytes → rapport) doit être pur. Là, il dépend du runtime et de ses verrous. |
| `filters` ↔ `engine::plugins` (cycle) | `src/filters/mod.rs:8` ↔ `src/engine/plugins/manager.rs:12` | Un cycle : impossible de tester ou de réutiliser l'un sans l'autre. |
| `engine` → `application` | `src/engine/tablet_manager/owner.rs:38,61,252` appellent `application::telemetry::capture_event` | La couche basse appelle la couche haute : la télémétrie est soudée au moteur. |
| `engine` → `settings` (globales) | `src/engine/plugins/manager.rs:44-80` appellent `settings::get_plugins_dir()` / `get_settings_dir()` | Des chemins globaux : on ne peut pas tester avec un dossier temporaire ni embarquer le moteur ailleurs. |
| `engine` → `settings` (logs) | `src/engine/tablet_manager/polling.rs:390` | La boucle de polling connaît le module de configuration. |
| Feature `desktop` | `Cargo.toml` `[features]` | Elle sert uniquement à *retirer* l'I/O pour le SDK. C'est le symptôme d'un découpage manquant. |
| Trois `Cargo.lock` | racine, `src-tauri/`, `sdk/` | Les versions divergent ; `src-tauri/Cargo.toml` dit même « copied: this crate has its own lockfile ». |
| Fichiers « dieu » | `core/config/models.rs` (934 l.), `tablet_manager/polling.rs` (715 l.) | Trop de responsabilités par fichier. |
| `MappingConfig` fourre-tout | `core/config/models.rs` | Un même objet contient la zone de mapping, les bindings, le websocket, le thème, la langue, le démarrage auto… |
| Bindings et modes en `String` | `tip_binding: String`, `pen_button_bindings: Vec<String>` | Les erreurs se voient à l'exécution et pas à la compilation ; le frontend doit deviner le format. |

**Conclusion** : il ne manque pas de code. Il manque des **frontières**. Les sous-crates sont la façon
la moins chère d'en avoir qui sont **vérifiées par le compilateur**.

---

## 2. Nouveau dépôt ou même dépôt ?

### Décision : **même dépôt**, la v3 s'appelle simplement `3.0.0`.

### Pourquoi pas un nouveau dépôt

1. **L'auto-update est câblé sur ce dépôt.**
   - `src-tauri/tauri.conf.json` → `https://github.com/Next-Tablet-Driver/NextTabletDriver/releases/latest/download/latest.json`
   - `src/application/services/autoupdate/github.rs` → `api.github.com/repos/Next-Tablet-Driver/NextTabletDriver/releases`

   Si la v3 vit dans un autre dépôt, **toutes les installations v2 restent sur la v2 pour toujours**,
   sans le savoir. C'est la raison la plus forte, et elle suffit à trancher.
2. **Historique Git** : `git blame` sur un parser Wacom explique *pourquoi* tel octet est masqué.
   Ce savoir est irremplaçable. Un nouveau dépôt le coupe.
3. **Issues, PR, étoiles, référencement, liens externes** (osu!, forums, wikis) : tout pointe ici.
4. **Écosystème de plugins** : les auteurs de plugins suivent ce dépôt et ce SDK.
5. **« v3 » est une version, pas un produit.** SemVer est fait pour ça : `3.0.0` = changements
   incompatibles. Le nom dans le dépôt n'apporte rien.

### Ce qu'on fait à la place

- Une branche longue `v3` **seulement si nécessaire**. Mieux : on travaille sur `main` par petites
  PR (voir §3), et la v2 reste maintenable via une branche `release/2.x` pour les correctifs.
- Si un jour on veut vraiment renommer le projet : **renommer le dépôt** sur GitHub (les redirections
  sont conservées) **et** publier d'abord une dernière v2.x qui pointe l'updater vers le nouvel
  endpoint. Jamais l'inverse.

---

## 3. Réécriture d'un coup ou migration progressive ?

### Décision : **migration progressive** (« strangler fig »), pas de big bang.

### Pourquoi

- La nouvelle architecture **correspond presque un pour un aux modules existants** (§4). On
  **déplace** du code et on **coupe des dépendances**, on ne réinvente pas d'algorithmes.
- Les parsers encodent des années de rétro-ingénierie. Les réécrire, c'est réintroduire des bugs
  déjà corrigés.
- Le produit est **sensible à la latence** (osu!). Un big bang mélange régressions de latence et
  changements d'architecture : impossible de savoir ce qui a cassé.
- **Effet second système** : une réécriture complète gonfle toujours en périmètre (« tant qu'on y
  est… ») et ne sort jamais.
- Les tests existants (unitaires, fuzz, E2E, smoke natif) **servent de filet** pendant tout le
  déplacement. Avec un big bang, on les perd aussi.

### Règle d'or

> À chaque PR, le binaire compile, les tests passent, et l'application fonctionne pour un utilisateur.
> Il n'existe jamais d'état « ça marchera quand tout sera fini ».

---

## 4. Architecture cible : le workspace

### 4.1 Disposition

```
Cargo.toml                 # [workspace] + [workspace.dependencies] + [workspace.lints]
Cargo.lock                 # UN SEUL lockfile
crates/
  ntd-core/                # types et maths purs
  ntd-tablets/             # descripteurs + parsers (bytes → rapport), purs
  ntd-hid/                 # I/O HID (hidapi)
  ntd-pipeline/            # transformation, projection, chaîne de filtres, trait Filter
  ntd-plugin-api/          # ABI C stable (ex sdk/plugin_api)
  ntd-plugin-host/         # chargement, vérification de confiance, gestion des plugins
  ntd-output/              # injection d'entrées OS (SendInput / uinput)
  ntd-interop/             # shm, lock, protocole de commandes inter-process
  ntd-settings/            # profils, préférences, thèmes, import OTD
  ntd-engine/              # runtime : threads, état, boutons, orchestration
  ntd-services/            # websocket, autoupdate, télémétrie, supervisor
  ntd-ipc/                 # DTO du contrat frontend + génération des types TS
  ntd-test-support/        # fakes (HID, output, horloge), fixtures — dev-only
apps/
  desktop/                 # ex src-tauri : câblage Tauri uniquement
  sdk/                     # cdylib/staticlib pour intégrateurs
plugins/                   # plugins officiels, membres du workspace
frontend/                  # inchangé dans sa structure
tablets/                   # données des tablettes (embarquées par ntd-tablets)
qa/fuzz/                   # cibles fuzz sur ntd-tablets et ntd-pipeline
xtask/                     # tâches de dev (génération des types TS, vérifs)
```

### 4.2 Responsabilités et dépendances

| Crate | Repris de | Dépend de | I/O autorisée |
|---|---|---|---|
| `ntd-core` | `core/input`, `core/math`, types de `core/config/models` réellement partagés, erreurs de base | — | **Aucune** |
| `ntd-tablets` | `drivers/parsers`, `drivers/config_loader`, `drivers/models`, `drivers/config`, matching VID/PID de `drivers/detection` | core | **Aucune** (les JSON sont embarqués) |
| `ntd-hid` | `drivers/hid_caps`, énumération de `detection`, `tablet_manager/reader` | core, tablets | HID |
| `ntd-pipeline` | `engine/pipeline/*`, `filters` (chaîne + trait) | core | **Aucune** |
| `ntd-plugin-api` | `sdk/plugin_api` | — | **Aucune** |
| `ntd-plugin-host` | `engine/plugins/*` | pipeline, plugin-api | fichiers, `libloading` |
| `ntd-output` | `engine/injector/*` | core | OS (injection) |
| `ntd-interop` | `engine/interop/*` | core | shm, verrous OS |
| `ntd-settings` | `settings/*` | core | fichiers (chemins **injectés**) |
| `ntd-engine` | `engine/tablet_manager/*`, `engine/state` | core, tablets, hid, pipeline, plugin-host, output, interop, settings | threads |
| `ntd-services` | `application/services/*`, `application/telemetry/*` | engine, settings | réseau |
| `ntd-ipc` | DTO implicites de `src-tauri/src/commands.rs` + `frontend/.../tauri/commands.ts` | core, settings (types) | **Aucune** |
| `apps/desktop` | `src-tauri` | engine, services, ipc | Tauri |
| `apps/sdk` | `sdk` | core, tablets, hid, pipeline | via ses dépendances |

### 4.3 Graphe (les flèches vont **toujours vers le bas**)

```
                 apps/desktop                 apps/sdk
                /     |      \                /   |   \
        services     ipc      \          tablets hid pipeline
            |          \       \
          engine ───────┴───────┘
   ┌────┬───┼────────┬─────────┬─────────┬──────────┐
  hid tablets pipeline plugin-host output  interop  settings
   │    │      │         │   │     │        │         │
   └────┴──────┴─────────┘  plugin-api      └─────────┴──► core
                 (tout dépend de core, core ne dépend de rien)
```

### 4.4 Les trois inversions qui cassent les cycles

1. **Filtres ↔ plugins** : `ntd-pipeline` définit `trait Filter`. `ntd-plugin-host` fournit un
   `PluginFilter: Filter`. Le pipeline ne sait pas que les plugins existent.
2. **Moteur → télémétrie** : `ntd-engine` publie des `EngineEvent` (ex. `DeviceLost`,
   `ParserError`) sur un canal. `ntd-services` s'abonne et décide d'envoyer ou non la télémétrie.
3. **Moteur → chemins globaux** : on construit une struct `Paths { settings, plugins, logs, … }`
   **une fois** dans `apps/desktop`, puis on la passe en paramètre. Aucun crate de bibliothèque
   n'appelle `directories` lui-même.

### 4.5 Interface publique du moteur

```rust
// ntd-engine — esquisse, pas une API figée
pub struct Engine { /* privé */ }

impl Engine {
    pub fn start(cfg: EngineConfig, deps: EngineDeps) -> Result<(EngineHandle, Receiver<EngineEvent>), EngineError>;
}

#[derive(Clone)]
pub struct EngineHandle { /* Sender<Command> */ }

impl EngineHandle {
    pub fn apply_mapping(&self, m: Mapping) -> Result<(), EngineError>;
    pub fn set_bindings(&self, b: Bindings) -> Result<(), EngineError>;
    pub fn set_filter_chain(&self, c: FilterChainConfig) -> Result<(), EngineError>;
    pub fn shutdown(self);
}

pub struct EngineDeps {
    pub hid: Box<dyn HidBackend>,      // réel ou fake
    pub output: Box<dyn OutputSink>,   // réel ou fake
    pub clock: Box<dyn Clock>,         // réel ou fake
    pub paths: Paths,
}
```

Le moteur se pilote **uniquement** par cette poignée. Il n'y a pas de `SharedState` global que
chaque commande Tauri verrouille et modifie à sa façon.

### 4.6 Configuration du workspace

```toml
[workspace]
resolver = "3"
members = ["crates/*", "apps/*", "plugins/*", "xtask"]

[workspace.package]
edition = "2024"
rust-version = "1.99.0"
license = "MIT"
repository = "https://github.com/Next-Tablet-Driver/NextTabletDriver"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "2"
log = "0.4"
crossbeam-channel = "0.5"
hidapi = "2.6.3"
# … une seule version de chaque dépendance pour tout le workspace

[workspace.lints.rust]
unsafe_code = "deny"           # surchargé à "allow" UNIQUEMENT dans hid, output, interop, plugin-host, plugin-api

[workspace.lints.clippy]
# reprendre la politique actuelle de clippy.toml / Cargo.toml
```

Chaque crate : `[lints] workspace = true` et `serde.workspace = true`, etc.

---

## 5. Le contrat backend ↔ frontend

On garde le frontend, donc **le contrat est la chose la plus importante à protéger**.

### 5.1 Principe

- `ntd-ipc` contient **tous** les types échangés avec le frontend : arguments et retours des
  commandes, payloads des événements, erreurs.
- Les types TypeScript sont **générés** depuis Rust (recommandé : `tauri-specta`, qui génère
  aussi les fonctions typées pour les commandes ; sinon `ts-rs`) dans
  `frontend/src/lib/infrastructure/tauri/bindings.ts`.
- La CI régénère les bindings et **échoue s'il y a une différence** (`cargo xtask gen-ts && git diff --exit-code`).

### 5.2 Ordre

1. **D'abord**, on fige le contrat v2 tel quel dans `ntd-ipc` (les 26 commandes de
   `src-tauri/src/commands.rs`, les événements `tablet-event`, `log-event`, `device-changed`,
   `profile-loaded`, `navigate-to-updates`). On génère les types. Le frontend ne change pas de
   comportement.
2. **Ensuite**, on fait évoluer le contrat vers sa forme v3 (§6.3), commande par commande, chaque
   changement étant **compilé des deux côtés** grâce aux types générés.

### 5.3 Erreurs

Les commandes renvoient `Result<T, IpcError>`, où `IpcError` est un enum sérialisé
(`{ kind: "DeviceNotFound" } | { kind: "InvalidMapping", field: "…" } | …`). Le frontend
`switch` sur `kind`. On ne fait jamais `.to_string()` d'une erreur vers le frontend pour qu'il
analyse le texte.

---

## 6. Le frontend : on le garde, mais sa couche de données change

### 6.1 Verdict : garder le frontend n'est **pas** une erreur

- Stack moderne et adaptée : **Svelte 5** (runes), **TypeScript**, **Vite**.
- Organisation par features (`mapping`, `pen`, `filters`, `settings`, `console`, `release`, `credits`).
- Design system avec tokens, thèmes validés (`theme/validate.ts`, `contrast.ts`) et tests.
- Tests unitaires (Vitest + Testing Library), E2E Playwright, smoke natif, mock Tauri (`src/dev/tauri-mock.ts`).
- ≈ 12 000 lignes **qui fonctionnent**. Les réécrire ne corrigerait aucun des problèmes listés
  ci-dessous : ils viennent tous du **contrat** et de la **couche de données**, pas de Svelte.

> Réécrire le frontend *et* le backend en même temps, c'est ne plus avoir aucun point fixe pour
> détecter les régressions. On garde le frontend **précisément** pour qu'il serve de référence.

### 6.2 Ce qui ne va pas côté frontend

| Problème | Où | Conséquence |
|---|---|---|
| Types écrits à la main, dupliqués depuis Rust | `infrastructure/tauri/commands.ts` (`Config`, `Area`, `PluginPropertyKind`…) | Ils dérivent en silence de Rust. Une faute de frappe = `undefined` à l'exécution. |
| `Config` fourre-tout | même fichier ; `get_config` / `set_config(new_config: MappingConfig)` côté Rust | Chaque petit changement renvoie **tout** l'objet, avec des conflits possibles si deux écrans modifient en même temps. |
| Chaînes magiques | `mode: string`, `tip_binding: string`, noms d'événements `"tablet-event"` | Pas de vérification à la compilation. |
| `invoke` direct dans les composants | `components/MenuBar.svelte:93,113,194,278` | Ça contourne la couche `infrastructure`, ce qui casse le mock et la cohérence. |
| Détection « dirty » par `JSON.stringify` | `features/mapping/mapping-state.svelte.ts` (le commentaire mentionne l'ordre de sérialisation) | Fragile : dépend de l'ordre des clés. |
| `setTimeout(…, 50)` pour `isLoaded` | `mapping-state.svelte.ts` | Contournement d'une course, pas une correction. |
| `listen` direct dans un composant | `features/mapping/TabletVisualizer.svelte` | Contourne `infrastructure/events.ts`. |
| Flux haute fréquence via `emit` JSON | `src-tauri/src/lib.rs:242` (`tablet-event`) | Coûteux en CPU ; le bon outil est `tauri::ipc::Channel`. |
| Double dépendance d'icônes | `package.json` : `@lucide/svelte` **et** `lucide-svelte` | Poids inutile, deux API. |

### 6.3 Ce qui doit changer (et rien de plus)

1. **Types générés** (§5) : suppression de toutes les interfaces DTO écrites à la main dans `commands.ts`.
2. **Une seule porte vers Tauri** : `infrastructure/tauri/` est le **seul** endroit qui importe
   `@tauri-apps/api`. Règle ESLint `no-restricted-imports` pour l'imposer partout ailleurs.
3. **Découper `Config` par domaine**, côté contrat comme côté stores :
   - `Mapping` (zones, rotation, mode, snapping, ratio)
   - `PenSettings` (seuils, pression, courbe, tilt)
   - `Bindings` (types : `enum Binding { Key{…}, Mouse{…}, Disabled, … }`)
   - `FilterChain` (plugins activés et leurs propriétés)
   - `AppPreferences` (thème, langue, démarrage auto, tray, timer haute résolution)
   - `WebSocketSettings`

   Une commande par domaine (`get_mapping` / `update_mapping`…). Un store Svelte par domaine.
4. **Le backend fait autorité.** Le frontend envoie une intention (`update_mapping`), le backend
   valide, applique et renvoie l'état réel (ou un événement `mapping-changed`). Le frontend
   affiche **ce que le backend renvoie**. Pas de « sauvegarde » qui écrase l'état à l'aveugle.
5. **Dirty et historique** : on compare de façon structurelle (égalité champ par champ) et on
   supprime `JSON.stringify`. L'undo/redo peut rester côté frontend, mais il porte sur des objets
   de domaine typés.
6. **Flux temps réel** : `tablet-event` passe par un `Channel` Tauri, débité à la fréquence d'affichage
   (≈ 60 Hz) **côté backend**. Le frontend ne reçoit jamais 1000 Hz.
7. **Supprimer les `setTimeout` de synchronisation**, en remplaçant par de vrais états (`loading | ready | error`).
8. **Garder** : composants UI, design system, thèmes, pages, tests, E2E, mock Tauri (mis à jour
   automatiquement grâce aux types générés).

---

## 7. Plan de migration par phases

Chaque phase = plusieurs petites PR. **Critère de sortie** obligatoire avant la phase suivante.

### Phase 0 — Filets de sécurité (avant de toucher l'architecture)

- Corriger la clé publique de l'updater (§11).
- Ajouter des **tests de golden files** pour les parsers : pour chaque tablette, des rapports HID
  bruts → rapport parsé attendu (snapshot). C'est le filet le plus important.
- Ajouter un **benchmark de latence** du pipeline (criterion existe déjà) et noter la référence.
- Figer le contrat IPC actuel (§5.2 étape 1) avec génération TS.
- **Sortie** : CI verte, golden files pour toutes les tablettes supportées, latence de référence notée.

### Phase 1 — Workspace sans changement de code

- Créer le `[workspace]`, fusionner les trois `Cargo.lock`, introduire `[workspace.dependencies]`
  et `[workspace.lints]`.
- Déplacer `src-tauri` → `apps/desktop`, `sdk` → `apps/sdk`, `sdk/plugin_api` → `crates/ntd-plugin-api`.
- **Sortie** : un seul lockfile, `cargo build --workspace` et `cargo test --workspace` verts,
  aucun changement de comportement.

### Phase 2 — Extraire les feuilles pures

Dans l'ordre : `ntd-core` → `ntd-tablets` → `ntd-pipeline`.

- Pour `ntd-tablets` : remplacer l'usage de `engine::state::LockRecoveryExt` dans les parsers
  Wacom par un état local au parser (un parser ne devrait pas avoir besoin d'un verrou) ou, à défaut,
  déplacer l'utilitaire dans `ntd-core`.
- Pour `ntd-pipeline` : introduire `trait Filter`, couper le cycle avec les plugins.
- Brancher `qa/fuzz` sur ces crates directement.
- **Sortie** : ces crates n'ont **aucune** dépendance I/O (vérifié dans `cargo tree`), golden files
  identiques, latence identique.

### Phase 3 — Extraire les adaptateurs I/O

`ntd-hid`, `ntd-output`, `ntd-interop`, `ntd-plugin-host`, `ntd-settings`, chacun derrière un trait
avec un fake dans `ntd-test-support`.

- `ntd-settings` reçoit `Paths` en paramètre ; plus aucune fonction `get_*_dir()` globale.
- **Sortie** : la feature `desktop` n'existe plus ; `apps/sdk` ne dépend plus de `ntd-engine`.

### Phase 4 — Le moteur

- `ntd-engine` avec `EngineHandle` + `EngineEvent` (§4.5).
- Découper `polling.rs` (715 l.) et `owner.rs` : lecture, parsing, pipeline, sortie, boutons.
- Télémétrie sortie du moteur (événements).
- **Sortie** : test d'intégration « fake HID → engine → fake output » avec horloge simulée ;
  latence ≤ référence.

### Phase 5 — Services et application

- `ntd-services` (websocket, autoupdate, télémétrie, supervisor) s'abonne aux `EngineEvent`.
- `apps/desktop` devient du câblage : environ 300 lignes au total, aucune logique métier.
- **Sortie** : `apps/desktop/src` ne contient plus de logique ; chaque commande fait ≤ 10 lignes.

### Phase 6 — Contrat v3 et frontend

- Découpage de `Config` par domaine, bindings typés, erreurs typées, `Channel` pour le temps réel (§6.3).
- Migration automatique des profils v2 → v3 (§8, `ntd-settings`).
- **Sortie** : E2E verts, profils v2 réels importés sans perte, `3.0.0-beta.1` publiée sur le canal
  de pré-release.

### Phase 7 — Sortie

- Bêta publique, puis `3.0.0`.
- Une dernière `2.x` maintenue pour les correctifs critiques pendant une durée annoncée.

---

## 8. Règles par crate

### `ntd-core`
- Zéro I/O, zéro thread, zéro `unsafe`, zéro dépendance lourde.
- Types de valeur (`Copy` quand c'est possible) ; unités explicites dans les noms ou des newtypes
  (`Millimeters`, `TabletUnits`, `Pixels`).

### `ntd-tablets`
- Un parser = une fonction `fn parse(&mut self, report: &[u8]) -> Result<Option<TabletReport>, ParseError>`.
- **Jamais de panic** sur une entrée : tout rapport malformé renvoie une erreur (c'est fuzzé).
- Pas de `log::` dans le chemin chaud : on renvoie une erreur typée et l'appelant décide.
- Chaque nouvelle tablette arrive avec ses golden files.

### `ntd-hid`
- Seul crate qui parle à `hidapi`.
- Gère la reconnexion, mais **ne parse pas** : il délivre des octets bruts.

### `ntd-pipeline`
- Déterministe : même entrée + même horloge = même sortie.
- **Aucune allocation** dans le chemin chaud (pré-allouer à la configuration).
- Le temps est injecté (`Clock`), jamais lu avec `Instant::now()` directement.

### `ntd-plugin-api`
- **ABI C stable, versionnée.** Toute modification incompatible = nouvelle version de l'ABI et le
  host garde le support de l'ancienne pendant au moins une version majeure.
- Uniquement des types `#[repr(C)]`. Pas de `String`, `Vec`, `Box` ni de trait Rust qui traversent la frontière.

### `ntd-plugin-host`
- La vérification de confiance (minisign) se fait **avant** `dlopen`, jamais après.
- Un plugin qui panique ou renvoie une erreur est désactivé, le pipeline continue sans lui.

### `ntd-output`
- Un trait `OutputSink` ; une implémentation par OS ; un fake qui enregistre les événements.

### `ntd-settings`
- Chaque fichier persistant a un champ `version`. Les migrations sont des fonctions pures
  `v2 → v3` testées avec de **vrais** fichiers v2 en fixtures.
- Écriture atomique (fichier temporaire + rename).
- Un fichier corrompu → sauvegarde du fichier corrompu, chargement des valeurs par défaut, message
  à l'utilisateur. On ne perd jamais silencieusement un profil.

### `ntd-engine`
- Seul propriétaire des threads du moteur.
- API = `EngineHandle` (commandes) + `Receiver<EngineEvent>`. Pas de `Arc<Mutex<…>>` exposé.

### `ntd-services`
- Toute fonctionnalité réseau (télémétrie, update, websocket) est **désactivable**, et la télémétrie
  est **opt-in**.

### `ntd-ipc`
- Uniquement des types sérialisables et la génération TS. Pas de logique.

### `apps/desktop`
- Câblage : construire `Paths`, démarrer `Engine` et `services`, enregistrer les commandes, le tray,
  l'updater. Chaque commande = traduire DTO → appel `EngineHandle` → DTO.

---

## 9. Ce qu'il ne faut PAS faire

> Cette section est **normative**, pour les humains comme pour les LLMs.
> Si une tâche semble exiger d'enfreindre une de ces règles : **arrête-toi et demande**.

### 9.1 Projet, dépôt, stratégie

1. **NE PAS** créer un nouveau dépôt pour la v3. L'updater des utilisateurs v2 pointe ici (§2).
2. **NE PAS** changer l'URL de l'updater, le nom du dépôt ou le propriétaire GitHub sans avoir d'abord
   publié une version v2.x qui pointe vers le nouvel emplacement.
3. **NE PAS** faire une réécriture big bang sur une branche qui diverge pendant des mois.
4. **NE PAS** mélanger dans une même PR un déplacement de code et un changement de comportement.
   Une PR qui déplace du code ne change **rien** d'autre (diff lisible avec `git diff -M`).
5. **NE PAS** profiter de la refonte pour ajouter des fonctionnalités. Les fonctionnalités
   attendent la fin de la phase concernée.
6. **NE PAS** réécrire le frontend.
7. **NE PAS** supprimer l'historique Git, squasher l'historique existant ou repartir d'un `git init`.
8. **NE PAS** casser `main` : chaque PR laisse l'application fonctionnelle.

### 9.2 Architecture et découpage

9. **NE PAS** créer de dépendance d'un crate bas vers un crate haut (voir §4.3). Si tu en as besoin,
   c'est qu'il manque un trait ou un événement : inverse la dépendance.
10. **NE PAS** créer de cycle entre crates, même via une feature optionnelle.
11. **NE PAS** créer un crate fourre-tout `ntd-common`, `ntd-utils`, `ntd-shared` ou `ntd-misc`. Chaque
    fonction a un propriétaire ; s'il n'y en a pas, c'est `ntd-core` et elle doit être pure.
12. **NE PAS** découper plus fin que nécessaire : **pas** un crate par fabricant de tablette, **pas**
    un crate par filtre, **pas** un crate par OS. Un module suffit.
13. **NE PAS** réintroduire de feature Cargo pour « retirer » des parties d'un crate (comme `desktop`).
    Si un consommateur n'en veut pas une partie, cette partie est un autre crate.
14. **NE PAS** utiliser les features Cargo pour choisir l'OS. Utiliser `cfg(target_os = …)`.
15. **NE PAS** ajouter d'état global (`static`, `lazy_static`, `OnceLock` métier, `get_settings_dir()`)
    dans un crate de bibliothèque. Les dépendances se passent en paramètre.
16. **NE PAS** exposer `Arc<Mutex<État>>` hors de `ntd-engine`. On communique par commandes et
    événements.
17. **NE PAS** appeler la télémétrie, le réseau ou le système de fichiers depuis `ntd-core`,
    `ntd-tablets` ou `ntd-pipeline`.
18. **NE PAS** mettre de logique métier dans `apps/desktop` ni dans les commandes Tauri.
19. **NE PAS** faire dépendre `apps/sdk` de `ntd-engine`, `ntd-services` ou de Tauri.
20. **NE PAS** dupliquer un type dans deux crates « pour éviter une dépendance ». Il appartient au
    crate le plus bas qui en a besoin.
21. **NE PAS** garder plusieurs `Cargo.lock`. Un seul workspace, un seul lockfile.
22. **NE PAS** déclarer une version de dépendance dans un crate membre : `foo.workspace = true`.

### 9.3 Rust : code et erreurs

23. **NE PAS** utiliser `unwrap()`, `expect()` ou `panic!` sur une donnée venant de l'extérieur
    (HID, fichier, réseau, plugin, frontend). Seuls les invariants internes prouvés peuvent `expect`
    avec un message qui explique l'invariant.
24. **NE PAS** utiliser `anyhow` (ni `Box<dyn Error>`) dans les crates de bibliothèque. On utilise des
    enums `thiserror`. `anyhow` est toléré seulement dans `xtask`.
25. **NE PAS** convertir une erreur en `String` pour la transporter. On garde le type.
26. **NE PAS** avaler les erreurs avec `let _ = …` ou `.ok()` sans commentaire qui justifie pourquoi
    l'échec est sans conséquence (le code v2 fait `let _ = app.emit(…)` partout).
27. **NE PAS** utiliser `unsafe` hors de `ntd-hid`, `ntd-output`, `ntd-interop`, `ntd-plugin-host`,
    `ntd-plugin-api`. Chaque bloc `unsafe` a un commentaire `// SAFETY:`.
28. **NE PAS** désactiver un lint clippy globalement pour faire passer la CI. `#[allow]` local avec
    justification, ou on corrige.
29. **NE PAS** écrire des fichiers de plus d'environ 400 lignes. Au-delà, découper par responsabilité.
30. **NE PAS** utiliser `String` pour un ensemble fini de valeurs (mode, binding, type de courbe,
    niveau de log). Utiliser un `enum` sérialisé.
31. **NE PAS** laisser des nombres sans unité : distinguer unités tablette, millimètres, pixels,
    secondes avec des newtypes ou des noms explicites.
32. **NE PAS** faire `pub` par défaut. `pub(crate)` d'abord ; l'API publique d'un crate est
    volontaire et documentée.

### 9.4 Latence, concurrence et chemin chaud

33. **NE PAS** allouer (`Vec::new`, `String`, `format!`, `Box`, `clone()` de collections) dans la
    boucle de lecture → parsing → pipeline → sortie.
34. **NE PAS** logger dans le chemin chaud (même en `debug!` : le formatage coûte). Compteurs ou
    échantillonnage seulement.
35. **NE PAS** prendre un verrou bloquant partagé avec l'UI dans le chemin chaud. La configuration
    arrive par un canal ou un swap atomique (`arc-swap`), lu sans blocage.
36. **NE PAS** utiliser `async`/Tokio dans le moteur. Le moteur est fait de threads dédiés avec des
    canaux ; l'async est réservé à `ntd-services` si besoin.
37. **NE PAS** appeler `Instant::now()` ou `sleep` directement dans le pipeline. On injecte un `Clock`.
38. **NE PAS** envoyer chaque rapport HID au frontend. On décime côté backend (≈ 60 Hz).
39. **NE PAS** fusionner une PR qui dégrade le benchmark de latence de référence sans décision explicite.

### 9.5 Tablettes et parsers

40. **NE PAS** réécrire un parser « plus proprement » sans golden files qui prouvent la parité octet par octet.
41. **NE PAS** supprimer un masque de bits, un décalage ou un cas particulier « qui a l'air inutile »
    dans un parser. Regarder `git blame` d'abord : c'est presque toujours un correctif matériel.
42. **NE PAS** ajouter une tablette sans descripteur dans `tablets/` **et** golden files.
43. **NE PAS** faire paniquer un parser sur un rapport trop court ou inconnu : on renvoie une erreur ou `None`.
44. **NE PAS** coder en dur un VID/PID dans le code Rust : il va dans les descripteurs.
45. **NE PAS** copier des descripteurs ou du code d'autres pilotes (ex. OpenTabletDriver) sans
    vérifier leur licence et l'attribution requise (`tablets/OpenTabletDriver` existe : vérifier
    la compatibilité de sa licence avec la licence MIT du projet).

### 9.6 Plugins, ABI et sécurité

46. **NE PAS** charger (`dlopen`/`LoadLibrary`) une bibliothèque avant d'avoir vérifié sa signature.
47. **NE PAS** modifier `ntd-plugin-api` de façon incompatible sans incrémenter la version d'ABI et
    garder la compatibilité avec la précédente.
48. **NE PAS** faire traverser la frontière ABI à des types Rust (`String`, `Vec`, `Box`, `Option<&T>` non
    `repr(C)`, traits, panics). Les panics doivent être attrapées (`catch_unwind`) côté plugin et côté host.
49. **NE PAS** affaiblir, contourner ou rendre optionnelle la vérification de confiance « pour le dev »
    dans le code de production. Le mode dev est explicite et visible dans l'interface.
50. **NE PAS** committer de clé privée, de token, ni de clé de développement à la place d'une vraie clé
    publique (voir §11).
51. **NE PAS** exposer le websocket hors de `127.0.0.1` par défaut.
52. **NE PAS** activer la télémétrie par défaut ni y inclure des données personnelles (chemins
    utilisateur, noms de fichiers de profils).

### 9.7 Settings et persistance

53. **NE PAS** changer le format d'un fichier persistant sans champ `version` et migration testée.
54. **NE PAS** casser les profils v2 des utilisateurs : la v3 les importe automatiquement, et garde
    une sauvegarde de l'original.
55. **NE PAS** écrire un fichier de configuration en place. Fichier temporaire, puis rename atomique.
56. **NE PAS** remplacer silencieusement un fichier illisible par les valeurs par défaut sans garder
    une copie et prévenir l'utilisateur.
57. **NE PAS** remettre un objet `Config` unique qui contient tout. Un type par domaine (§6.3).

### 9.8 Contrat IPC et frontend

58. **NE PAS** écrire à la main en TypeScript un type qui existe en Rust. Il est généré.
59. **NE PAS** modifier `bindings.ts` à la main. Il est régénéré et vérifié en CI.
60. **NE PAS** importer `@tauri-apps/api` ailleurs que dans `frontend/src/lib/infrastructure/tauri/`.
61. **NE PAS** utiliser de chaînes littérales pour les noms de commandes ou d'événements dans les composants.
62. **NE PAS** renvoyer tout l'état à chaque modification (`set_config(tout)`). Une commande par
    domaine, avec validation backend.
63. **NE PAS** faire confiance au frontend pour valider. Le backend valide tout et renvoie l'état réel.
64. **NE PAS** renvoyer des erreurs sous forme de texte que le frontend devrait analyser. Erreurs typées.
65. **NE PAS** utiliser `setTimeout` pour « attendre que ça soit prêt ». Utiliser un état explicite.
66. **NE PAS** comparer des objets avec `JSON.stringify`.
67. **NE PAS** utiliser `emit` d'événements JSON pour les flux haute fréquence. Utiliser `Channel`.
68. **NE PAS** changer de framework frontend, de design system ou de structure de dossiers pendant la refonte.
69. **NE PAS** garder deux bibliothèques pour la même chose (`@lucide/svelte` et `lucide-svelte`).

### 9.9 Tests et CI

70. **NE PAS** supprimer, ignorer (`#[ignore]`, `.skip`) ou affaiblir un test pour faire passer une PR.
71. **NE PAS** baisser les seuils de `coverage-thresholds.json`.
72. **NE PAS** écrire de tests qui nécessitent une vraie tablette ou une vraie injection OS pour
    tester la logique. Utiliser les fakes de `ntd-test-support`.
73. **NE PAS** mettre à jour des snapshots / golden files en masse sans relire chaque différence et
    expliquer pourquoi dans la PR.
74. **NE PAS** tester les détails d'implémentation (« la fonction privée X est appelée »). Tester le
    comportement observable.
75. **NE PAS** fusionner avec une CI rouge « parce que c'est flaky ». Un test flaky est un bug à corriger.

### 9.10 Release et mise à jour

76. **NE PAS** publier une version dont la signature n'a pas été vérifiée avec la clé publique
    réellement embarquée dans l'application.
77. **NE PAS** publier la v3 en `latest` avant une période de bêta sur un canal de pré-release.
78. **NE PAS** retirer le support d'auto-update vers la v3 depuis la dernière v2.x.

### 9.11 Spécifique aux LLMs / assistants de code

79. **NE PAS** inventer d'API, de crate, de commande Tauri ou de champ de configuration. Vérifier dans
    le code (`grep`) avant de les utiliser ou de les citer.
80. **NE PAS** « simplifier » du code de parser, de masque HID ou de timing sans le comprendre.
81. **NE PAS** générer du code de remplacement pour un module entier quand la tâche demande un
    déplacement. Déplacer = `git mv` + corriger les chemins, rien d'autre.
82. **NE PAS** ajouter de dépendance externe sans la déclarer dans `[workspace.dependencies]`, vérifier
    qu'elle passe `cargo deny`, et justifier pourquoi la std ou une dépendance existante ne suffit pas.
83. **NE PAS** contourner une règle de ce document en disant qu'elle « ne s'applique pas vraiment ici ».
    Demander.
84. **NE PAS** déclarer une tâche terminée sans avoir lancé `cargo fmt --check`, `cargo clippy --workspace
    --all-targets`, `cargo test --workspace`, et côté frontend `npm run check`, `npm run lint`, `npm test`.
85. **NE PAS** produire de gros commits « refactor everything ». Une PR = une intention.
86. **NE PAS** laisser de `TODO` sans issue liée, ni de code commenté « au cas où ».
87. **NE PAS** inclure d'identifiant de modèle IA dans les commits, le code ou la documentation.

---

## 10. Checklist de PR

- [ ] La PR a **une seule intention** (déplacer **ou** changer un comportement, pas les deux).
- [ ] Aucune dépendance de crate ne va vers le haut (§4.3) ; `cargo tree -p <crate>` est conforme.
- [ ] Aucun nouvel état global, aucun `unwrap` sur une donnée externe, aucun `unsafe` hors des crates autorisés.
- [ ] Golden files des parsers identiques (ou différences justifiées une par une).
- [ ] Benchmark de latence ≤ référence si le chemin chaud est touché.
- [ ] `bindings.ts` régénéré et sans diff si le contrat est touché.
- [ ] Migration et fixtures si un format persistant est touché.
- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`,
      `cargo deny check` verts.
- [ ] `npm run check`, `npm run lint`, `npm test`, `npm run e2e` verts.
- [ ] Documentation (`docs/`) à jour si le comportement visible change.

---

## 11. Problèmes trouvés en dehors de l'architecture

Ces points sont à traiter **dès la phase 0**, indépendamment de la refonte :

1. **Clé publique de l'updater factice.** `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`
   vaut `dW51c2VkX3B1YmtleV9mb3JfZGV2ZWxvcG1lbnRfcHVycG9zZXM=`, qui se décode en
   `unused_pubkey_for_development_purposes`. La release est signée en CI
   (`.github/workflows/release.yml`, `TAURI_SIGNING_PRIVATE_KEY`), mais l'application ne peut pas
   vérifier cette signature avec cette clé. Selon le comportement de l'updater, l'auto-update échoue
   ou n'est pas protégé. **À corriger avant toute autre chose**, puisque la stratégie de migration
   v2 → v3 repose entièrement sur l'auto-update.
2. **Licence des descripteurs `tablets/OpenTabletDriver`** : vérifier la licence d'origine et sa
   compatibilité avec la licence MIT du projet, et ajouter l'attribution requise.
3. **Double dépendance d'icônes** dans `frontend/package.json` (`@lucide/svelte` et `lucide-svelte`).
