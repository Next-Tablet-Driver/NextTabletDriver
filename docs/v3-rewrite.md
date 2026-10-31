# NextTabletDriver v3 — Plan de refonte du backend

> **Statut** : document de cadrage. Il décide **pourquoi** on refait le backend, **comment** on le fait,
> et surtout **ce qu'il ne faut pas faire**.
>
> **Public** : contributeurs humains **et** assistants IA (LLMs). Si tu es un LLM qui travaille sur ce
> dépôt : la section [« Ce qu'il ne faut PAS faire »](#9-ce-quil-ne-faut-pas-faire) est **normative**.
> Une règle qui commence par « NE PAS » est une interdiction, pas une suggestion. En cas de doute,
> arrête-toi et demande, plutôt que de « faire au mieux ».
>
> **Avancement et prochaine tâche** : voir la [roadmap (§17)](#17-roadmap). Elle est la source de
> vérité sur ce qui est fait et sur ce qu'il faut faire ensuite.

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
12. [Qualité de code](#12-qualité-de-code)
13. [CI/CD](#13-cicd)
14. [Contributions, IA et AGENTS.md](#14-contributions-ia-et-agentsmd)
15. [Outils internes](#15-outils-internes)
16. [Plateformes cibles](#16-plateformes-cibles)
17. [Roadmap](#17-roadmap)

---

## 0. Résumé des décisions

| Question | Décision | Raison en une ligne |
|---|---|---|
| Refaire l'architecture du backend ? | **Oui** | Les modules ont des dépendances circulaires qu'un seul crate ne peut pas empêcher. |
| Nouveau dépôt « NextTabletDriver v3 » ? | **Non. Même dépôt, version 3.0.0** | L'auto-update des utilisateurs v2 pointe en dur vers ce dépôt. Un nouveau dépôt les abandonne sur la v2. |
| Réécrire d'un coup (big bang) ? | **Non. Migration progressive** | Environ 30 parsers de tablettes et un pipeline sensible à la latence : la parité ne se réécrit pas d'un coup. |
| Garder le frontend ? | **Oui** | Svelte 5 + TS + tests + design system + E2E : la base est saine. |
| Changer quelque chose côté frontend ? | **Oui, la couche de données** | Types écrits à la main, `Config` fourre-tout, chaînes magiques, `invoke` hors couche. |
| Quelles plateformes ? | **Windows, Linux, macOS ; x64, x86 32 bits, ARM** | Pas tout en 3.0 : la v3 livre d'abord Windows et Linux, mais l'architecture prévoit la place de chaque plateforme (§16). |
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
  ntd-platform/            # intégration OS hors HID/injection : démarrage auto, instance
                           # unique, priorité des threads, timer haute résolution (§16.4)
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
qa/fuzz/                   # cibles fuzz sur ntd-tablets, ntd-pipeline et ntd-interop
tools/
  ntd-codegen/             # tablets/*.json → Rust généré dans ntd-tablets (§15.1)
  ntd-tester/              # replay HID → moteur réel → sortie factice (§15.2)
  ntd-recorder/            # capture HID d'une tablette pour les issues (§15.3)
  ntd-stress/              # stress du websocket et du protocole interop (§15.5)
xtask/                     # tâches de dev (types TS, lint-style, plan CI)
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
| `ntd-platform` | démarrage auto, instance unique, priorité de thread et timer (aujourd'hui éparpillés dans `startup.rs`, `application/`, `engine/`) | core | OS |
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
members = ["crates/*", "apps/*", "plugins/*", "tools/*", "xtask"]

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

# Les lints complets sont dans §12.3.
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
- Écrire `ntd-recorder` (§15.3) en premier : c'est lui qui produit les captures dont les golden
  files ont besoin.
- Ajouter un **benchmark de latence** du pipeline (criterion existe déjà) et noter la référence.
- Figer le contrat IPC actuel (§5.2 étape 1) avec génération TS.
- **Sortie** : CI verte, golden files pour toutes les tablettes supportées, latence de référence notée.

### Phase 1 — Workspace sans changement de code

- Créer le `[workspace]`, fusionner les trois `Cargo.lock`, introduire `[workspace.dependencies]`
  et `[workspace.lints]`.
- Déplacer `src-tauri` → `apps/desktop`, `sdk` → `apps/sdk`, `sdk/plugin_api` → `crates/ntd-plugin-api`.
- Mettre en place la politique de qualité (§12) et la CI (§13). Les nouveaux lints `deny` vont
  faire apparaître du code existant non conforme : le corriger dans des PR dédiées, **sans**
  changer de comportement, crate par crate.
- Écrire `AGENTS.md` et la politique de contribution (§14).
- **Sortie** : un seul lockfile, `cargo build --workspace` et `cargo test --workspace` verts,
  aucun changement de comportement.

### Phase 2 — Extraire les feuilles pures

Dans l'ordre : `ntd-core` → `ntd-tablets` → `ntd-pipeline`.

- Pour `ntd-tablets` : remplacer l'usage de `engine::state::LockRecoveryExt` dans les parsers
  Wacom par un état local au parser (un parser ne devrait pas avoir besoin d'un verrou) ou, à défaut,
  déplacer l'utilitaire dans `ntd-core`.
- Pour `ntd-pipeline` : introduire `trait Filter`, couper le cycle avec les plugins.
- Brancher `qa/fuzz` sur ces crates directement, et ajouter les cibles des parsers (§15.4).
- Remplacer `include_dir` + parsing JSON au démarrage par `ntd-codegen` (§15.1).
- **Sortie** : ces crates n'ont **aucune** dépendance I/O (vérifié dans `cargo tree`), golden files
  identiques, latence identique.

### Phase 3 — Extraire les adaptateurs I/O

`ntd-hid`, `ntd-output`, `ntd-interop`, `ntd-plugin-host`, `ntd-settings`, chacun derrière un trait
avec un fake dans `ntd-test-support`.

- `ntd-settings` reçoit `Paths` en paramètre ; plus aucune fonction `get_*_dir()` globale.
- Créer `ntd-platform` et donner à chaque adaptateur OS (`ntd-hid`, `ntd-output`, `ntd-interop`,
  `ntd-platform`) la structure `windows` / `linux` / `macos` de §16.3, avec le module `macos`
  qui renvoie `Unsupported`.
- **Sortie** : la feature `desktop` n'existe plus ; `apps/sdk` ne dépend plus de `ntd-engine` ;
  le workspace compile (`cargo check`) sur toutes les cibles de §16.2, y compris celles qui ne
  sont pas encore implémentées.

### Phase 4 — Le moteur

- `ntd-engine` avec `EngineHandle` + `EngineEvent` (§4.5).
- Découper `polling.rs` (715 l.) et `owner.rs` : lecture, parsing, pipeline, sortie, boutons.
- Télémétrie sortie du moteur (événements).
- **Sortie** : `ntd-tester` (§15.2) passe sur toutes les captures ; latence ≤ référence.

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
    --all-targets --locked`, `cargo test --workspace`, `cargo xtask lint-style`, `cargo machete`,
    `typos`, et côté frontend `npm run check`, `npm run lint`, `npm test`.
85. **NE PAS** produire de gros commits « refactor everything ». Une PR = une intention.
86. **NE PAS** laisser de `TODO` sans issue liée, ni de code commenté « au cas où ».
87. **NE PAS** inclure d'identifiant de modèle IA dans les commits, le code ou la documentation.
88. **NE PAS** ouvrir de pull request en tant qu'agent, même si on te le demande. Tu prépares la
    branche et la description ; un humain ouvre la PR (§14.3).

### 9.12 Qualité de code et commentaires

89. **NE PAS** dépasser 100 caractères par ligne dans un fichier `.rs`, commentaires et chaînes
    compris (§12.1).
90. **NE PAS** écrire de commentaire qui paraphrase le code (`// incrémente i`, `// retourne le
    résultat`). Un commentaire explique un **pourquoi** : un choix, une contrainte matérielle,
    un invariant, un piège (§12.2).
91. **NE PAS** décorer les commentaires : pas de bannières (`// =====`, `// -----`, `/*****/`), pas
    d'emojis, pas d'ASCII art, pas de titres de section en majuscules, pas de signature ni de date.
92. **NE PAS** laisser de code commenté. L'historique Git le garde.
93. **NE PAS** écrire `#[allow(...)]` sans `reason = "..."`. La raison est le commentaire.
94. **NE PAS** utiliser `println!`, `eprintln!`, `print!` ni `dbg!`. Tout passe par `log`.
95. **NE PAS** utiliser `unreachable!()` : renvoyer une erreur, ou restructurer le code pour que
    le cas n'existe pas dans les types.
96. **NE PAS** écrire de bloc `unsafe` sans commentaire `// SAFETY:` qui prouve l'invariant.
97. **NE PAS** déclarer un élément `pub` sans doc `///`, ni une fonction publique qui renvoie
    `Result` sans section `# Errors`, ni une fonction qui peut paniquer sans `# Panics`.
98. **NE PAS** déclarer de dépendance inutilisée (cargo-machete échoue).
99. **NE PAS** rétrograder un lint de `deny` à `warn` ou `allow` au niveau du workspace sans
    décision des mainteneurs, consignée dans la PR et dans la `reason`.
100. **NE PAS** figer une version précise de Rust dans `rust-toolchain.toml` ou dans un workflow.
     La toolchain est `stable` (§12.6). Quand une nouvelle version casse la CI, on corrige le
     code, on ne revient pas à une version figée.
101. **NE PAS** modifier à la main un fichier dans `crates/ntd-tablets/src/generated/`. On modifie
     le JSON dans `tablets/` ou le générateur, puis on régénère.
102. **NE PAS** committer des fichiers régénérés sans rapport avec la PR. Un passage de codegen
     peut toucher d'autres fichiers : on garde ceux qui sont nécessaires et on annule le reste.
103. **NE PAS** inventer ou retoucher une capture `ntd-recorder`. Une capture vient d'une vraie
     tablette ; si elle est fausse, on en refait une.
104. **NE PAS** mettre des données personnelles dans une capture (nom de machine, chemins,
     numéro de série complet) : `ntd-recorder` les retire, et on vérifie avant de committer.
105. **NE PAS** supprimer une entrée du corpus fuzz qui a fait planter le code : elle devient un
     test de régression.

### 9.13 Plateformes et architectures

106. **NE PAS** supposer 64 bits. Pas de `usize`/`isize` dans les formats persistants, l'ABI des
     plugins, le protocole interop ou le contrat IPC : utiliser `u32`/`u64` explicites. Pas de
     `as usize` sur un `u64` : `usize::try_from`.
107. **NE PAS** écrire `cfg(not(windows))` pour dire « Linux ». Toujours `cfg(target_os = "...")`
     explicite, et un OS non géré tombe dans une branche qui renvoie `Unsupported`, jamais dans
     une branche silencieuse ou vide.
108. **NE PAS** mettre de code spécifique à un OS hors de `ntd-hid`, `ntd-output`, `ntd-interop`,
     `ntd-platform` et `apps/desktop`.
109. **NE PAS** lire des octets HID ou réseau avec `from_ne_bytes`, `transmute` ou un cast de
     pointeur. Toujours `from_le_bytes` / `from_be_bytes` explicite.
110. **NE PAS** utiliser d'intrinsèques SIMD ou d'instructions propres à une architecture sans
     version portable équivalente, testée sur toutes les cibles.
111. **NE PAS** charger un plugin compilé pour une autre cible : le manifeste déclare son target
     triple et le host refuse un plugin qui ne correspond pas.
112. **NE PAS** annoncer (README, site, notes de version) une plateforme de niveau 3 comme
     supportée.
113. **NE PAS** laisser une plateforme de niveau 3 cesser de compiler : `check-targets` est une
     check bloquante.
114. **NE PAS** supposer Windows dans le frontend (barre de titre, raccourcis `Ctrl`, emplacement
     des boutons de fenêtre) : passer par `infrastructure/os.ts`, qui expose la plateforme.
115. **NE PAS** promettre le 32 bits sur macOS : il n'existe plus (macOS ne lance plus
     d'applications 32 bits depuis 10.15).

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
- [ ] Aucune ligne de plus de 100 caractères, aucun commentaire décoratif ou paraphrasant (§12).
- [ ] `cargo machete` et `typos` verts (§13).
- [ ] La description dit **quoi**, **pourquoi**, **comment c'est testé**, et si une IA a écrit
      une part significative du code (§14).

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

---

## 12. Qualité de code

Ces règles s'appliquent à tout le code Rust du workspace. Elles sont **vérifiées par la CI** :
une règle qu'aucun outil ne vérifie finit toujours par ne plus être respectée.

### 12.1 Largeur de ligne : 100 caractères, strict

- `rustfmt.toml` : `max_width = 100` (déjà le cas en v2).
- **Limite de rustfmt** : sur la toolchain stable, rustfmt ne coupe ni les commentaires ni les
  chaînes longues (`wrap_comments`, `comment_width` et `error_on_line_overflow` sont instables).
  Clippy n'a pas de lint de longueur de ligne. Une vérification dédiée
  (`cargo xtask lint-style`, §12.5) fait donc échouer la CI dès qu'une ligne de `.rs` dépasse
  100 caractères, commentaires et chaînes compris.
- Une chaîne trop longue se découpe avec `concat!` ou une continuation `\` en fin de ligne. On ne
  la contourne pas avec un `#[rustfmt::skip]`.
- Éditeur : `.vscode/settings.json` garde `"editor.rulers": [80, 100]` (déjà le cas). 80 est un
  repère visuel, 100 est la limite. `.editorconfig` ajoute `max_line_length = 100` pour `[*.rs]`.

`rustfmt.toml` cible (options stables uniquement) :

```toml
edition = "2024"
max_width = 100
newline_style = "Unix"
use_field_init_shorthand = true
use_try_shorthand = true
```

### 12.2 Commentaires

**Principe** : le code dit **quoi**, le commentaire dit **pourquoi**. Si un commentaire répète ce
que le code dit, on le supprime. Si le code n'est pas clair sans commentaire, on renomme ou on
découpe d'abord, et on commente seulement ce qui reste.

Un commentaire est justifié quand il explique :
- un **choix** entre plusieurs options (« canal borné plutôt que non borné : on préfère perdre un
  rapport que d'accumuler de la latence ») ;
- une **contrainte matérielle ou externe** (« la Intuos Pro envoie la pression sur 13 bits ; les
  3 bits hauts sont du bruit ») ;
- un **invariant** que les types n'expriment pas ;
- un **piège** (« ne pas déplacer cet appel avant `flush` : Windows perd l'événement ») ;
- la preuve d'un bloc `unsafe` (`// SAFETY:`, obligatoire).

Interdits :
- la paraphrase (`// boucle sur les rapports`, `// crée le parser`) ;
- les décorations : bannières `// ======`, `// ------`, `/*******/`, titres en majuscules,
  emojis, ASCII art, signatures, dates ;
- le code commenté ;
- les `TODO` sans issue : la seule forme acceptée est `// TODO(#123): ...`.

```rust
// Mauvais
// ===== PARSING =====
// On récupère la pression
let pressure = u16::from_le_bytes([r[6], r[7]]) & 0x1FFF; // masque 🎯

// Bon
// Les 3 bits hauts portent l'état du bouton latéral sur ce modèle,
// pas la pression : voir les golden files intuos_pro_*.
let pressure = u16::from_le_bytes([r[6], r[7]]) & 0x1FFF;
```

**Documentation publique obligatoire** (`missing_docs = "deny"`) :
- chaque élément `pub` a un commentaire `///` court et factuel : ce que c'est, pas comment c'est
  implémenté ;
- chaque fonction publique qui renvoie `Result` a une section `# Errors`
  (`missing_errors_doc = "deny"`) ;
- chaque fonction publique qui peut paniquer a une section `# Panics`
  (`missing_panics_doc = "deny"`), ce qui devrait être rare vu §9.3 ;
- chaque crate a un `//!` en tête de `lib.rs` qui dit sa responsabilité et ce qu'il ne fait pas.

`unreachable_pub = "deny"` complète cette règle : un élément qui n'est pas réellement exporté
s'écrit `pub(crate)` et n'a donc pas besoin de doc publique.

### 12.3 Cargo.toml strict

Les lints sont définis **une seule fois** dans `[workspace.lints]` du `Cargo.toml` racine ; chaque
crate a `[lints] workspace = true` et rien d'autre.

```toml
[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "deny"
unreachable_pub = "deny"
dead_code = "deny"

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"

[workspace.lints.clippy]
# Groupes : tout est bloquant.
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
cargo = { level = "deny", priority = -1 }

# Gestion d'erreurs : aucune donnée externe ne doit pouvoir faire paniquer le pilote.
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
exit = "deny"
lossy_float_literal = "deny"

# Sorties : tout passe par `log`.
print_stdout = "deny"
print_stderr = "deny"
dbg_macro = "deny"

# unsafe et FFI.
undocumented_unsafe_blocks = "deny"
multiple_unsafe_ops_per_block = "deny"
uninit_assumed_init = "deny"
uninit_vec = "deny"
cast_ptr_alignment = "deny"
ptr_as_ptr = "deny"
fn_to_numeric_cast_any = "deny"

# Documentation.
missing_errors_doc = "deny"
missing_panics_doc = "deny"

# Tout #[allow] doit porter sa justification : #[allow(lint, reason = "...")].
allow_attributes_without_reason = "deny"

# Exceptions, reprises de la v2.
# Les rapports HID mélangent u8/u16/i16/f32 : les conversions sont voulues et bornées.
cast_precision_loss = "allow"
cast_possible_truncation = "allow"
cast_sign_loss = "allow"
# x/y, tilt_x/tilt_y : les noms proches sont le vocabulaire du domaine.
similar_names = "allow"
# Les parsers sont de longs `match` plats sur les octets ; les découper nuit à la lecture.
too_many_lines = "allow"
assert_is_empty = "allow"
# Les doublons de versions sont gérés par cargo-deny (bans), avec une liste d'exceptions revue.
multiple_crate_versions = "allow"
```

Points d'attention :
- **`unsafe_code`** : `deny` (et pas `forbid`) pour que les crates FFI autorisés (`ntd-hid`,
  `ntd-output`, `ntd-interop`, `ntd-plugin-host`, `ntd-plugin-api`) puissent mettre
  `#![allow(unsafe_code, reason = "...")]` à la racine du module concerné. `cargo xtask
  lint-style` échoue si ce `allow` apparaît dans un autre crate.
- **`clippy::cargo`** impose des métadonnées complètes : chaque crate hérite de
  `[workspace.package]` (`license.workspace = true`, `repository.workspace = true`…) et déclare
  sa propre `description`. Les versions wildcard (`"*"`) sont interdites.
- **`nursery` en `deny`** : ces lints évoluent entre versions de Rust. Avec la toolchain
  `stable` (§12.6), une nouvelle version de Rust peut donc faire échouer la CI sans qu'aucun code
  n'ait changé. C'est voulu : on corrige ces nouveaux lints dans une PR dédiée, et on ne
  rétrograde jamais le lint pour faire repasser la CI (règle 99).
- **Tests** : `clippy.toml` garde `allow-unwrap-in-tests`, `allow-expect-in-tests`,
  `allow-panic-in-tests` et `allow-indexing-slicing-in-tests`. Un test qui panique, c'est un
  test qui échoue, et c'est le comportement voulu.
- Toutes les commandes cargo de la CI utilisent `--locked`.

### 12.4 Gestion d'erreurs

- Un enum d'erreur `thiserror` par crate (`TabletError`, `PipelineError`, `EngineError`…). Les
  erreurs d'un crate bas s'emballent dans celui du dessus via `#[from]`, sans perdre la source.
- `?` partout. Une conversion `Option` → `Result` s'écrit `.ok_or(Erreur::Variante)`, jamais
  `.unwrap()`.
- Les seuls `expect` tolérés hors tests sont des invariants internes, avec
  `#[allow(clippy::expect_used, reason = "invariant : ...")]`. La raison est relue en review.

### 12.5 `cargo xtask lint-style`

Une tâche dans `xtask/` vérifie ce que rustfmt et clippy ne savent pas vérifier. Elle tourne en CI
et en local :
- lignes de plus de 100 caractères dans les `.rs` (hors fichiers générés) ;
- commentaires décoratifs : bannières (`//` suivi de 4 `=`, `-`, `*` ou `#`, ou plus), emojis ;
- `TODO`/`FIXME` sans `(#numéro)` ;
- `#![allow(unsafe_code` hors des crates autorisés ;
- `#[rustfmt::skip]` sans commentaire de justification juste au-dessus.

### 12.6 Toolchain : `stable`

La v3 suit la **dernière version stable de Rust**, plutôt qu'une version figée comme en v2
(`1.99.0`).

`rust-toolchain.toml` :

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-analyzer", "rust-src"]
```

Conséquences et règles :
- **Pas de version précise** dans `rust-toolchain.toml`, ni dans les workflows CI. Les jobs
  installent la toolchain depuis ce fichier, sans la redéfinir.
- **`rust-version`** (la MSRV, version minimale supportée) reste déclarée une seule fois, dans
  `[workspace.package]`. Elle documente la plus vieille version qui compile le projet. Ce
  n'est pas la version utilisée en CI. On la remonte quand on adopte une nouveauté du langage.
- **Une nouvelle version stable peut casser la CI** (nouveaux lints `pedantic`/`nursery`,
  avertissements de `rustc`). On corrige dans une PR dédiée, intitulée
  `chore: fix lints for Rust 1.xx`, qui ne contient rien d'autre.
- **Anticiper** : un job planifié chaque semaine lance `clippy` et les tests sur la toolchain
  `beta`. Il n'est pas bloquant, mais il ouvre une issue s'il échoue, ce qui laisse environ six
  semaines pour corriger avant que la version arrive en stable.
- **Traçabilité des releases** : les builds ne sont plus reproductibles à la version de `rustc`
  près. Le job de release écrit donc la sortie de `rustc -Vv` dans les notes de version et dans
  les artefacts.
- `Cargo.lock` reste commité et toutes les commandes utilisent `--locked` : seules la toolchain
  et les lints suivent `stable`, les dépendances ne bougent pas sans PR.

---

## 13. CI/CD

### 13.1 Principes

- **Une seule check requise** sur `main` : `ci-success`, qui dépend de tous les jobs. On peut ainsi
  ajouter des jobs sans toucher aux règles de protection de branche.
- **CI par crate touché** : un job `plan` calcule les crates modifiés par la PR et leurs
  dépendants (`cargo metadata` + `git diff` sur la base). Lints, tests et builds ne tournent que
  sur ceux-là. On fait une **CI complète** sur `main`, sur les tags, et dès qu'un fichier global
  change (`Cargo.toml` racine, `Cargo.lock`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`,
  `deny.toml`, `.github/**`).
- `RUSTFLAGS="-D warnings"`, `--locked` partout, `concurrency` avec `cancel-in-progress` sur les PR.
- Actions **épinglées par SHA**, `permissions: contents: read` par défaut. Les droits d'écriture
  ne sont donnés qu'aux jobs de release.
- Les secrets de signature ne sont accessibles que dans un **environnement GitHub `release`**
  protégé (reviewers requis), jamais dans les jobs de PR.

### 13.2 `ci.yml` : tests et qualité (PR, push sur `main`)

| Job | Contenu |
|---|---|
| `plan` | `cargo xtask ci-plan` : crates touchés, ou exécution complète (§15.6) |
| `fmt` | `cargo fmt --all --check` |
| `style` | `cargo xtask lint-style` (§12.5) |
| `typos` | `crate-ci/typos` sur le code, les commentaires et `docs/` (config `typos.toml`, exclusions : `tablets/`, fichiers générés) |
| `machete` | `cargo machete` : aucune dépendance inutilisée |
| `clippy` | `cargo clippy --locked --all-targets --all-features` sur les crates touchés |
| `clippy-release` | idem avec `--release` (code derrière `cfg(not(debug_assertions))`) |
| `doc` | `cargo doc --no-deps` avec `RUSTDOCFLAGS=-D warnings` |
| `test` | `cargo nextest run` + doctests, matrice Windows x64 et Linux x64 |
| `golden` | golden files des parsers (§7, phase 0) |
| `bindings` | `cargo xtask gen-ts && git diff --exit-code` |
| `codegen` | `cargo run --locked -p ntd-codegen && git diff --exit-code` (§15.1) |
| `tester` | `cargo run --locked -p ntd-tester` sur toutes les captures (§15.2) |
| `fuzz` | chaque cible fuzz pendant 60 s sur les PR qui touchent les crates concernés ; plus longtemps dans le workflow nightly |
| `coverage` | `cargo llvm-cov` + seuils de `coverage-thresholds.json` (exécution complète) |
| `bench` | benchmark de latence comparé à la référence, sur `main` et sur les PR étiquetées `perf` |
| `frontend` | `npm run check`, `lint`, `test`, `build` |
| `e2e` | Playwright |
| `build` | build des applications sur les cibles de niveau 1 et 2 (§16.2) |
| `check-targets` | `cargo clippy` sur les cibles de niveau 3 (§16.2) : elles doivent compiler même si elles ne sont pas encore implémentées |
| `ci-success` | agrège tous les jobs ci-dessus |

Hors `ci.yml`, un workflow `toolchain-beta.yml` planifié chaque semaine lance clippy et les tests
sur la toolchain `beta`. Il n'est pas bloquant et ouvre une issue s'il échoue (§12.6).

### 13.3 `security.yml` : sécurité (PR, `main`, et chaque jour)

- `cargo deny check` : advisories, licences, bans (doublons), sources. Une seule exécution grâce au
  lockfile unique, au lieu d'un audit par lockfile comme en v2.
- `npm audit --omit=dev` sur `frontend/`.
- CodeQL (existe déjà).
- `actions/dependency-review-action` sur les PR.
- Secret scanning et push protection activés dans les réglages du dépôt.

### 13.4 `prerelease.yml` : pré-release (tag `v3.0.0-beta.N` / `-rc.N`)

1. Vérifier que le tag pointe sur un commit de `main` dont la CI est verte, et que la version des
   `Cargo.toml` et de `tauri.conf.json` correspond au tag.
2. Construire la matrice Windows x64/ARM64 et Linux x64/ARM64.
3. Signer, puis **vérifier la signature avec la clé publique réellement embarquée** dans
   `tauri.conf.json`. Le job échoue si la clé est absente ou factice (§11).
4. Publier une **GitHub pre-release** avec les sommes SHA-256.
5. Publier le manifeste du **canal bêta**. Attention : l'endpoint actuel
   `releases/latest/download/latest.json` ignore les pre-releases, donc le canal bêta a besoin
   de son propre endpoint stable. Les utilisateurs s'y abonnent avec une option « recevoir les
   bêtas » dans les paramètres.

### 13.5 `nightly.yml` : build de nuit

- Chaque nuit, **seulement s'il y a eu des commits** sur `main` depuis le dernier nightly.
- Build de la matrice complète, signature, publication sous un tag `nightly` qu'on **écrase**
  (pre-release GitHub, marquée comme non supportée).
- Fuzzing long (par exemple 30 min par cible) et `ntd-stress` sur le build produit.
- Canal d'update `nightly` séparé, opt-in, avec son propre endpoint (même raison que le canal bêta).
- Ne tourne que sur le dépôt principal (`if: github.repository == 'Next-Tablet-Driver/NextTabletDriver'`),
  jamais sur les forks.

### 13.6 `release.yml` : release (tag `vX.Y.Z`)

Mêmes étapes que la pré-release, plus :
- publication en `latest` et mise à jour de `latest.json` (canal stable) ;
- notes de version générées à partir des titres de PR (déjà normalisés par `pr-title.yml`) ;
- publication winget (`winget-release.yml` existe).

---

## 14. Contributions, IA et AGENTS.md

### 14.1 `AGENTS.md`

- **`AGENTS.md` à la racine**, court : ce qu'est le projet, la disposition du workspace (§4.1),
  les commandes à lancer avant de rendre une tâche (§9, règle 84), et un renvoi vers ce
  document, avec la mention que §9 est normatif. Ajouter `CLAUDE.md` en lien symbolique vers
  `AGENTS.md` pour que tous les outils lisent le même fichier.
- **Un `AGENTS.md` par zone sensible**, avec les détails propres à la zone :
  - `crates/ntd-tablets/AGENTS.md` : ajouter une tablette, golden files, ne jamais supprimer un
    masque sans `git blame` ;
  - `crates/ntd-plugin-api/AGENTS.md` : stabilité de l'ABI, `repr(C)`, versionnage ;
  - `crates/ntd-engine/AGENTS.md` : règles du chemin chaud (§9.4) ;
  - `crates/ntd-ipc/AGENTS.md` : régénérer les bindings, ne jamais éditer `bindings.ts`.
- Si un mainteneur dit autre chose que `AGENTS.md`, on suit le mainteneur, **sauf** pour la
  règle 14.3.

### 14.2 Politique d'utilisation de l'IA

- L'auteur d'une PR est responsable de **chaque ligne**, quel que soit l'outil qui l'a écrite.
  Avant d'ouvrir la PR, il a lu tout le diff, lancé le code, et sait répondre aux questions de
  review sans demander à un modèle.
- La description dit si une IA a écrit une part significative du code ou de la description.
  Supprimer cette mention d'une description rédigée par un agent est un motif de fermeture.
- Les mainteneurs peuvent fermer sans review une PR qu'ils estiment générée et non vérifiée.
- On ne rouvre pas une PR fermée et on ne la recrée pas à l'identique : on en discute d'abord.

### 14.3 Les agents n'ouvrent pas de PR

Un agent IA prépare la branche, les commits et le texte de la description. Il n'ouvre **jamais**
la pull request lui-même, même si on le lui demande : c'est un humain qui l'ouvre, après avoir
relu. Un agent ne pousse jamais sur `main`.

### 14.4 Critères de fermeture sans review

Une PR peut être fermée sans review complète si :
- la description est vide, répète seulement le titre, ou ne dit pas comment le changement a été
  testé ;
- elle mélange des changements sans rapport (reformatage de code non touché, lockfile modifié
  sans raison, fichiers générés sans le changement qui les justifie) ;
- elle modifie un parser sans golden files, ou change un comportement visible sans capture ;
- elle duplique une PR ouverte sans dire en quoi elle diffère ;
- l'auteur ne sait pas expliquer son changement ;
- elle cache ou retire la mention d'usage de l'IA (§14.2) ;
- la CI échoue et l'auteur ne la corrige pas, ou ne répond plus à la review.

### 14.5 Modèle de description de PR

`.github/pull_request_template.md` :

```markdown
## Quoi

## Pourquoi

## Impact (utilisateurs, latence, formats persistants, ABI, contrat IPC)

## Comment c'est testé

## Limites connues

## Usage de l'IA
<!-- Aucun / partiel (préciser quoi) / majoritaire -->
```

---

## 15. Outils internes

Ces outils s'inspirent de ceux de PumpkinMC (`tools/pumpkin-codegen`, `tools/pumpkin-tester`,
`tools/pumpkin-fuzzer`, l'Extractor et `.github/scripts/ci_packages.py`), adaptés à un pilote de
tablette. Ce sont des binaires dans `tools/`, membres du workspace. Ils suivent les lints du
workspace : un outil CLI qui écrit sur la sortie standard le déclare avec
`#[expect(clippy::print_stdout, reason = "...")]` sur sa fonction `main`, pas par un `allow`
global.

### 15.1 `ntd-codegen` : les tablettes deviennent du code

**Problème v2** : `src/drivers/config_loader.rs` embarque `tablets/` avec `include_dir` et
parse le JSON **au démarrage**. Un JSON invalide n'est donc découvert qu'à l'exécution, chez
l'utilisateur.

**Ce que fait l'outil** :
- lit `tablets/**/*.json`, les valide (champs obligatoires, VID/PID uniques, parser référencé
  existant, dimensions cohérentes) ;
- génère des tables statiques Rust dans `crates/ntd-tablets/src/generated/`, avec un en-tête
  « fichier généré par ntd-codegen, ne pas modifier » ;
- les fichiers générés sont **commités**, comme chez Pumpkin : les diffs se relisent en review et
  la compilation ne dépend pas d'un `build.rs`.

**Commandes** : `cargo run --locked -p ntd-codegen` régénère tout. La CI régénère et échoue s'il y
a une différence.

**Conséquences** : plus de `serde_json` ni d'`include_dir` dans le chemin de démarrage, plus
d'erreur de descripteur à l'exécution, et le matching VID/PID devient une recherche dans une
table statique.

Un `tools/ntd-codegen/AGENTS.md` décrit comment ajouter un champ de descripteur.

### 15.2 `ntd-tester` : le moteur réel, sans tablette

**Équivalent** de `pumpkin-tester` (vrai serveur + bots + scénarios).

**Ce que fait l'outil** :
- démarre le **vrai** `ntd-engine` avec un `HidBackend` qui rejoue une capture (§15.3), un
  `OutputSink` qui enregistre ce qui serait injecté, une horloge simulée et des `Paths` dans un
  dossier temporaire ;
- pour chaque scénario, compare la sortie (positions, pression, clics, touches) à un fichier
  attendu, avec une tolérance explicite pour les flottants ;
- couvre ce que les tests unitaires ne voient pas : profils (mapping, rotation, mode relatif),
  chaîne de filtres et plugins officiels, bindings des boutons, débranchement et rebranchement
  au milieu d'une capture.

**Format d'un scénario** : `qa/scenarios/<nom>/` contient la capture, le profil et la sortie
attendue. Ajouter un scénario ne demande aucun code.

**Commande** : `cargo run --locked -p ntd-tester` (ou `-- <nom>` pour un seul scénario). Il
renvoie un code de sortie non nul au premier écart et affiche un diff lisible.

### 15.3 `ntd-recorder` : capturer une tablette

**Équivalent** de l'Extractor de Pumpkin, qui produit les données de référence depuis le jeu.
Ici, les données de référence viennent de la tablette de l'utilisateur.

**Ce que fait l'outil** :
- liste les périphériques HID, l'utilisateur choisit sa tablette ;
- enregistre le descripteur de rapport HID, les infos du périphérique (VID, PID, chaînes produit,
  interfaces) et une session de rapports bruts horodatés pendant que l'utilisateur suit des
  consignes à l'écran (« survole sans toucher », « appuie doucement puis fort », « clique sur le
  bouton 1 », « incline le stylet »…) ;
- écrit un seul fichier (`.ntdcap`, JSON ou binaire versionné), **sans données personnelles**
  (numéro de série tronqué, pas de nom de machine ni de chemin) ;
- est distribué en binaire autonome et documenté dans `docs/adding-a-tablet.md`. Le modèle
  d'issue `tablet_support.yml` demande ce fichier.

**Usages** :
1. Ajout d'une tablette : la capture sert à écrire le descripteur, puis devient golden file et
   scénario `ntd-tester`.
2. Rapport de bug : on rejoue la capture du rapporteur au lieu de deviner.
3. Corpus de départ pour le fuzzing (§15.4).

### 15.4 Fuzzing des parsers et du protocole

**Constat v2** : `qa/fuzz/fuzz_targets/` ne couvre que les maths (`rotate_point`,
`normalized_to_screen`, `physical_to_normalized`, `apply_relative_delta`). **Aucun parser HID
n'est fuzzé**, alors que c'est la surface qui reçoit des octets extérieurs.

**À ajouter** (cargo-fuzz) :
- une cible par famille de parser (`wacom`, `huion`, `xp_pen`, `uclogic`, `veikk`…) qui envoie des
  octets arbitraires au parser et vérifie qu'il renvoie `Ok`/`Err`/`None` **sans paniquer**, et
  que les valeurs produites restent dans les bornes du descripteur ;
- une cible pour le décodage du protocole de commandes `ntd-interop` et une pour les messages
  websocket entrants ;
- une cible pour le chargement des profils et la migration v2 → v3 (`ntd-settings`).

**Corpus** : il démarre avec les captures `ntd-recorder` ; chaque crash trouvé est ajouté au
corpus **et** devient un test unitaire.

### 15.5 `ntd-stress` : stress du websocket et de l'IPC

**Équivalent** de `pumpkin-fuzzer`, qui bombarde un serveur lancé avec des paquets aléatoires.

**Ce que fait l'outil** :
- se connecte à un NTD lancé (websocket local et protocole de commandes interop) ;
- modes : messages aléatoires, messages valides mais hors bornes, trames corrompues ou
  tronquées, connexions et déconnexions en rafale, clients lents qui ne lisent pas ;
- options : durée, concurrence, mode ;
- vérifie à la fin que le processus est vivant, que la mémoire est revenue à la normale et que
  la latence du pipeline n'a pas bougé.

Il tourne dans le workflow nightly (§13.5), pas sur chaque PR : c'est un test long.

### 15.6 `cargo xtask ci-plan` : CI par crate touché

**Inspiré de** `.github/scripts/ci_packages.py` de Pumpkin, mais **écrit en Rust dans `xtask`**
plutôt qu'en Python : une seule langue pour les outils, et il réutilise `cargo metadata`.

**Ce que fait la commande** :
- calcule les fichiers modifiés entre la base et la tête de la PR ;
- les associe aux crates, puis ajoute **tous les crates qui en dépendent** (graphe inverse de
  `cargo metadata`) ;
- bascule en exécution complète si un fichier global change (liste en §13.1), sur `main`, sur
  les tags et sur le nightly ;
- écrit dans `$GITHUB_OUTPUT` la liste des crates et la matrice de test (Linux et Windows x64
  sur les PR, matrice complète avec ARM64 en exécution complète).

Les jobs `clippy`, `test` et `build` lisent cette sortie et passent `-p <crate>` pour chaque
crate. Le script a ses propres tests unitaires (graphe de dépendances factice).

---

## 16. Plateformes cibles

### 16.1 Objectif

À terme, NextTabletDriver doit fonctionner de façon **stable** sur **Windows, Linux et macOS**,
sur toutes les architectures courantes de chacun : x86_64 (Intel/AMD 64 bits), x86 32 bits, ARM
64 bits et, sur Linux, ARM 32 bits ; sur macOS, Intel et Apple Silicon.

**Ce n'est pas l'objectif de la 3.0.** La v2 ne gère que Windows et Linux (`cfg(windows)` et
`cfg(target_os = "linux")` dans `engine/injector`, `engine/interop`…), et le support d'une
nouvelle plateforme est un vrai travail (permissions, injection, packaging). La refonte doit
en revanche **préparer** ce travail : chaque nouvelle plateforme doit être une implémentation à
ajouter dans des emplacements déjà prévus, pas une refonte de plus.

### 16.2 Matrice et niveaux de support

| Plateforme | Target Rust | Niveau en 3.0 | Ensuite |
|---|---|---|---|
| Windows x64 | `x86_64-pc-windows-msvc` | **1** | — |
| Linux x64 | `x86_64-unknown-linux-gnu` | **1** | — |
| Windows ARM64 | `aarch64-pc-windows-msvc` | **2** | 1 |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | **2** | 1 |
| macOS Apple Silicon | `aarch64-apple-darwin` | 3 | 2, puis 1 |
| macOS Intel | `x86_64-apple-darwin` | 3 | 2 (binaire universel avec Apple Silicon) |
| Windows x86 32 bits | `i686-pc-windows-msvc` | 3 | 2 |
| Linux x86 32 bits | `i686-unknown-linux-gnu` | 3 | 2 |
| Linux ARM 32 bits | `armv7-unknown-linux-gnueabihf` | 3 | 2 |

Hors périmètre : macOS 32 bits (il n'existe plus) et Windows ARM 32 bits (abandonné par
Microsoft).

Définition des niveaux :
- **Niveau 1** : fonctionnalités complètes, tests et `ntd-tester` en CI sur un runner natif,
  publiée dans chaque release, bugs bloquants pour une release.
- **Niveau 2** : fonctionnalités complètes, build et tests en CI (runner natif, ou émulation
  QEMU / `cross` pour l'ARM 32 bits et l'x86 32 bits sous Linux), publiée en release avec la
  mention « expérimental ».
- **Niveau 3** : **préparée, pas implémentée**. Le workspace compile pour cette cible (`cargo
  clippy --target …` bloquant en CI), les modules OS existent et renvoient
  `PlatformError::Unsupported`. Rien n'est publié.

Une plateforme passe au niveau supérieur par une décision explicite, quand elle remplit les
critères ; jamais « parce que ça compile ».

### 16.3 Ce que la refonte doit préparer dès maintenant

**Structure des adaptateurs OS.** Chaque crate qui touche l'OS (`ntd-hid`, `ntd-output`,
`ntd-interop`, `ntd-platform`) a la même forme :

```
src/
  lib.rs          # trait public + choix de l'implémentation selon target_os
  windows.rs
  linux.rs
  macos.rs        # en 3.0 : renvoie PlatformError::Unsupported
  unsupported.rs  # tout autre OS : erreur explicite, compile quand même
```

```rust
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::Backend;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::Backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::Backend;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub use unsupported::Backend;
```

**Capacités plutôt que suppositions.** Les plateformes n'offrent pas toutes la même chose
(injection de la pression, de l'inclinaison, souris absolue). Les traits exposent des capacités :

```rust
pub struct OutputCapabilities {
    pub absolute_pointer: bool,
    pub pressure: bool,
    pub tilt: bool,
    pub relative_pointer: bool,
}

pub trait OutputSink {
    fn capabilities(&self) -> OutputCapabilities;
    // ...
}
```

Le moteur et le frontend lisent ces capacités : le frontend masque ou désactive une option non
disponible au lieu de l'afficher sans effet.

**Indépendance vis-à-vis de l'architecture** :
- types à largeur fixe dans tout ce qui est sérialisé ou traverse une frontière (règle 106) ;
- octets lus avec un ordre explicite (règle 109) ;
- tous les `Atomic*` utilisés doivent exister sur toutes les cibles de §16.2 : vérifier
  `cfg(target_has_atomic = "64")` avant d'utiliser `AtomicU64` dans un crate partagé ;
- la CI de niveau 3 attrape ces problèmes tôt, avant même qu'une plateforme soit implémentée.

**Plugins et SDK** :
- l'ABI de `ntd-plugin-api` n'utilise que des types à largeur fixe et `repr(C)` : la même ABI
  vaut pour 32 et 64 bits ;
- le manifeste d'un plugin déclare son target triple ; le host refuse un plugin d'une autre
  cible (règle 111). Le format de distribution d'un plugin prévoit **plusieurs binaires** (un par
  cible) dès la 3.0, même si on n'en publie que deux ;
- `apps/sdk` et ses bindings (C, C#) sont construits par cible, avec le même nommage d'artefacts
  que l'application.

**Descripteurs de tablettes** : le schéma de `ntd-codegen` (§15.1) prévoit un champ optionnel par
OS pour les cas où l'interface ou la collection HID à ouvrir diffère selon le système.

**Mise à jour et packaging** :
- le manifeste d'update (`latest.json` et les canaux bêta/nightly) utilise les clés de
  plateforme de l'updater Tauri (`windows-x86_64`, `linux-aarch64`, `darwin-aarch64`…) ; le
  script qui le génère prend la liste des cibles en entrée au lieu de la coder en dur ;
- les noms d'artefacts suivent un seul format : `NextTabletDriver_<version>_<os>_<arch>.<ext>`.

**Frontend** : `infrastructure/os.ts` expose la plateforme (OS, architecture) et les capacités ;
les composants qui en dépendent (`TitleBar.svelte`, raccourcis clavier, menus) ne testent
jamais l'OS eux-mêmes (règle 114).

### 16.4 `ntd-platform`

Petit crate pour l'intégration système qui n'est ni HID ni injection, et qui est aujourd'hui
éparpillée :

| Fonction | Windows | Linux | macOS (plus tard) |
|---|---|---|---|
| Démarrage automatique | registre `Run` | `.desktop` dans `~/.config/autostart` | LaunchAgent |
| Instance unique | mutex nommé | socket ou verrou de fichier | socket ou verrou de fichier |
| Priorité du thread de polling | `SetThreadPriority` | `sched_setscheduler` | QoS de thread |
| Timer haute résolution (`force_high_resolution_timer`) | `timeBeginPeriod` | sans objet | sans objet |
| Dossiers (config, plugins, logs) | via `directories`, passés en `Paths` (§4.4) | idem | idem |

### 16.5 Points de vigilance par plateforme (pour plus tard)

Ces points ne sont **pas** à traiter en 3.0, mais l'architecture ne doit pas les rendre
difficiles.

**macOS** :
- **Permissions** : la lecture HID demande l'autorisation « Surveillance de l'entrée » et
  l'injection demande « Accessibilité ». L'application doit détecter leur absence et guider
  l'utilisateur, ce qui suppose des erreurs typées `PermissionDenied` dès maintenant dans
  `ntd-hid` et `ntd-output`.
- **Saisie exclusive** : pour que le pilote système ne déplace pas aussi le curseur, il faut
  ouvrir la tablette en mode exclusif (IOKit). `HidBackend` doit donc accepter une option
  « exclusif » dès la 3.0, même si seule une plateforme l'utilise.
- **Signature et notarisation** obligatoires (compte Apple Developer, Hardened Runtime). Le
  Hardened Runtime bloque le chargement de bibliothèques signées par d'autres que nous :
  charger des plugins tiers demande l'entitlement `com.apple.security.cs.disable-library-validation`.
  À décider explicitement au moment du portage, et la vérification minisign (§8) reste
  obligatoire dans tous les cas.
- **Mémoire partagée** : les noms POSIX `shm_open` sont limités à 31 caractères sur macOS ;
  `ntd-interop` ne doit pas supposer des noms plus longs.
- **Interface** : boutons de fenêtre à gauche, barre de menus globale, raccourcis `Cmd`.
- **Binaire universel** (Intel + Apple Silicon) plutôt que deux téléchargements.

**Linux** :
- règles udev pour l'accès à `hidraw` et `uinput` sans root, à fournir dans les paquets ;
- X11 et Wayland : l'injection par `uinput` fonctionne sur les deux, mais la détection des écrans
  diffère et doit passer par un trait ;
- formats de paquets par architecture (deb, rpm, AppImage), WebKitGTK disponible sur chaque cible.

**32 bits (Windows x86, Linux x86, ARM 32 bits)** :
- vérifier que Tauri, WebView2 (Windows x86) et WebKitGTK sont disponibles pour la cible ;
- mémoire plus limitée : pas de tampons dimensionnés « large » par défaut dans le pipeline.

**Windows ARM64** : vérifier que chaque plugin officiel et le SDK sont bien publiés pour ARM64,
et pas seulement l'application.

---

## 17. Roadmap

### 17.1 État actuel

| | |
|---|---|
| **Phase en cours** | Phase 0 — Filets de sécurité |
| **Dernière tâche terminée** | aucune |
| **Prochaine tâche** | P0.1 |
| **Dernière mise à jour** | 2026-10-31 |

Ce tableau est mis à jour par **chaque** PR qui coche une tâche.

### 17.2 Comment utiliser la roadmap

**Légende** :
- `[ ]` à faire
- `[~]` en cours : ajouter la branche ou la PR et l'auteur, par exemple `[~] (branche refactor/p2-2-tablets, @pseudo)`
- `[x]` terminée : ajouter le lien de la PR qui l'a terminée
- `[-]` abandonnée : ajouter la raison et le lien de la décision

**Règles** (pour les humains et les LLMs) :
1. **Ce fichier, sur `main`, fait foi.** Ni une issue, ni une conversation, ni la mémoire d'un
   agent ne remplace la case cochée ici.
2. **Quelle tâche prendre** : la **première** tâche `[ ]` de la phase en cours dont toutes les
   dépendances (« Dépend de ») sont `[x]`. Deux tâches d'une même phase sans dépendance entre elles
   peuvent avancer en parallèle, par des personnes différentes.
3. **Pas de saut de phase.** Une phase commence quand la **porte** de la phase précédente est
   cochée. Une porte se coche seulement quand tous ses critères sont vérifiés, commande à l'appui.
4. **Avant de commencer** : vérifier qu'aucune PR ou branche ouverte ne traite déjà la tâche, puis
   passer la tâche à `[~]` (petite PR de roadmap, ou premier commit de la branche).
5. **Une tâche = une PR**, sauf mention « (plusieurs PR) ». La PR qui termine la tâche la coche
   **dans le même commit** et met à jour §17.1.
6. **« Terminé quand » est un contrat.** On ne coche pas si un seul critère n'est pas vérifié.
   Une tâche à moitié faite reste `[~]`.
7. **Une tâche mal découpée** (trop grosse, dépendance oubliée) se corrige d'abord dans une PR qui
   modifie seulement la roadmap, puis on reprend.
8. **Pour un LLM** : ne travailler que sur **une** tâche à la fois, celle que la règle 2 désigne
   ou celle qu'un mainteneur a nommée ; relire la section de référence (§…) et les règles
   « NE PAS » (§9) avant de commencer ; ne jamais cocher une case sans avoir lancé les commandes
   de la règle 84 ; ne pas ouvrir de PR (règle 88).
9. **Chaque PR** passe aussi la checklist de PR (§10).

---

### Phase 0 — Filets de sécurité

Objectif : pouvoir détecter toute régression **avant** de toucher l'architecture (§7, phase 0).

- [ ] **P0.1** Corriger la clé publique de l'updater (§11.1)
  - Dépend de : —
  - Terminé quand : `tauri.conf.json` contient la vraie clé publique associée à
    `TAURI_SIGNING_PRIVATE_KEY` ; une mise à jour de test signée est acceptée par l'application
    et une mise à jour signée avec une autre clé est refusée.
- [ ] **P0.2** Vérifier la licence de `tablets/OpenTabletDriver` (§11.2)
  - Dépend de : —
  - Terminé quand : la licence d'origine est identifiée, l'attribution requise est ajoutée (ou les
    fichiers incompatibles sont remplacés), et la décision est écrite dans la PR.
- [ ] **P0.3** Supprimer la double dépendance d'icônes (§11.3)
  - Dépend de : —
  - Terminé quand : une seule bibliothèque Lucide dans `frontend/package.json`, `npm run check`,
    `npm test` et `npm run e2e` verts.
- [ ] **P0.4** Écrire `ntd-recorder` v1 (§15.3)
  - Dépend de : —
  - Note : il vit dans `tools/ntd-recorder` avec son propre `Cargo.lock` **temporairement** ; il
    rejoint le workspace en P1.2. C'est la seule exception à la règle 21.
  - Terminé quand : l'outil liste les périphériques, capture descripteur + rapports guidés, écrit
    un `.ntdcap` versionné sans données personnelles ; `docs/adding-a-tablet.md` et
    `.github/ISSUE_TEMPLATE/tablet_support.yml` demandent ce fichier.
- [ ] **P0.5** Harness de golden files (`qa/golden`)
  - Dépend de : P0.4
  - Terminé quand : `cargo test` rejoue chaque capture dans le parser v2 et compare à un snapshot
    commité ; un octet modifié dans un parser fait échouer le test.
- [ ] **P0.6** Golden files pour les tablettes supportées (plusieurs PR)
  - Dépend de : P0.5
  - Terminé quand : chaque tablette dont on a une capture réelle (ou des rapports réels dans les
    tests existants) a ses golden files ; les tablettes sans capture sont listées dans
    `qa/golden/MISSING.md` (règle 103 : on n'invente pas de capture).
- [ ] **P0.7** Référence de latence
  - Dépend de : —
  - Terminé quand : un benchmark criterion couvre le chemin parsing → pipeline → sortie (fake) ;
    les résultats et la machine de mesure sont notés dans `docs/perf-baseline.md`.
- [ ] **P0.8** Générer les types TS du contrat v2 (§5.2, étape 1)
  - Dépend de : —
  - Terminé quand : les 26 commandes et 5 événements actuels ont des types générés depuis Rust
    dans `frontend/src/lib/infrastructure/tauri/bindings.ts`, **sans changement de comportement** ;
    un job CI régénère et échoue s'il y a une différence.
- [ ] **P0.9** Le frontend utilise les types générés
  - Dépend de : P0.8
  - Terminé quand : `commands.ts` et `events.ts` n'ont plus d'interface DTO écrite à la main ;
    `npm run check`, `npm test`, `npm run e2e` verts.
- [ ] **PORTE 0**
  - Terminé quand : P0.1 à P0.9 sont `[x]`, la CI est verte, et `docs/perf-baseline.md` existe.

---

### Phase 1 — Workspace, qualité, CI, règles de contribution

Objectif : un seul workspace et toute la politique de qualité en place, **sans changer le
comportement** (§7, phase 1 ; §12, §13, §14).

- [ ] **P1.1** Créer le workspace et déplacer les applications
  - Dépend de : PORTE 0
  - Terminé quand : `[workspace]` à la racine ; `src-tauri` → `apps/desktop`, `sdk` → `apps/sdk`,
    `sdk/plugin_api` → `crates/ntd-plugin-api`, plugins officiels membres ; **un seul**
    `Cargo.lock` ; `cargo build --workspace` et `cargo test --workspace` verts ; diff de
    déplacement pur (`git diff -M`).
- [ ] **P1.2** Intégrer `tools/ntd-recorder` au workspace
  - Dépend de : P1.1
  - Terminé quand : son `Cargo.lock` séparé est supprimé.
- [ ] **P1.3** `[workspace.dependencies]` et `[workspace.package]`
  - Dépend de : P1.1
  - Terminé quand : aucun crate membre ne déclare de version de dépendance (règle 22) ;
    métadonnées héritées.
- [ ] **P1.4** Toolchain `stable` (§12.6)
  - Dépend de : P1.1
  - Terminé quand : `rust-toolchain.toml` sur `channel = "stable"`, aucune version de Rust dans les
    workflows, `rust-version` déclaré une fois dans `[workspace.package]`.
- [ ] **P1.5** Formatage et éditeur (§12.1)
  - Dépend de : P1.1
  - Terminé quand : `rustfmt.toml` et `.editorconfig` conformes à §12.1 ; `cargo fmt --all
    --check` vert.
- [ ] **P1.6** Introduire `[workspace.lints]` en `warn` (§12.3)
  - Dépend de : P1.3
  - Terminé quand : le bloc de §12.3 est en place avec les groupes en `warn` **temporairement** ;
    le nombre d'avertissements par crate est noté dans la PR.
- [ ] **P1.7** Corriger les avertissements de lints (plusieurs PR, un domaine par PR)
  - Dépend de : P1.6
  - Terminé quand : zéro avertissement sur le workspace ; chaque PR ne corrige que des lints,
    sans changement de comportement (golden files et E2E identiques).
- [ ] **P1.8** Passer les lints en `deny`
  - Dépend de : P1.7
  - Terminé quand : `[workspace.lints]` est exactement celui de §12.3.
- [ ] **P1.9** `xtask` et `cargo xtask lint-style` (§12.5)
  - Dépend de : P1.1
  - Terminé quand : la commande vérifie les 5 points de §12.5, a ses tests, et passe sur le dépôt.
- [ ] **P1.10** `typos` et `cargo-machete`
  - Dépend de : P1.1
  - Terminé quand : `typos.toml` existe, les deux outils passent sur le dépôt.
- [ ] **P1.11** `cargo xtask ci-plan` (§15.6)
  - Dépend de : P1.9
  - Terminé quand : la commande produit la liste des crates touchés + dépendants et la matrice, a
    ses tests sur un graphe factice.
- [ ] **P1.12** Refondre `ci.yml` (§13.1, §13.2)
  - Dépend de : P1.8, P1.9, P1.10, P1.11
  - Terminé quand : les jobs de §13.2 qui ont déjà un objet existent (fmt, style, typos, machete,
    clippy, clippy-release, doc, test, golden, bindings, coverage, frontend, e2e, build) ;
    actions épinglées par SHA ; `ci-success` agrège tout.
- [ ] **P1.13** Refondre `security.yml` (§13.3)
  - Dépend de : P1.1
  - Terminé quand : un seul `cargo deny check`, `npm audit`, dependency review sur les PR ; secret
    scanning et push protection activés.
- [ ] **P1.14** `toolchain-beta.yml` (§12.6)
  - Dépend de : P1.12
  - Terminé quand : job hebdomadaire non bloquant qui ouvre une issue en cas d'échec.
- [ ] **P1.15** Protection de `main`
  - Dépend de : P1.12
  - Terminé quand : `ci-success` est la seule check requise ; push direct sur `main` interdit.
- [ ] **P1.16** `AGENTS.md` racine et `CLAUDE.md` (§14.1)
  - Dépend de : P1.1
  - Terminé quand : `AGENTS.md` décrit le projet, la disposition, les commandes, renvoie à §9 et
    §17 ; `CLAUDE.md` est un lien vers lui.
- [ ] **P1.17** Contribution : politique IA, critères de fermeture, modèle de PR (§14.2 à §14.5)
  - Dépend de : —
  - Terminé quand : `.github/CONTRIBUTING.md` et `.github/pull_request_template.md` sont à jour.
- [ ] **P1.18** Branche `release/2.x`
  - Dépend de : P1.1
  - Terminé quand : la branche existe à partir du dernier tag 2.x, et `docs/releasing.md` explique
    comment y livrer un correctif.
- [ ] **PORTE 1**
  - Terminé quand : P1.1 à P1.18 sont `[x]` ; CI verte ; golden files, latence et E2E identiques
    à la phase 0.

---

### Phase 2 — Extraire les crates purs

Objectif : `ntd-core`, `ntd-tablets`, `ntd-pipeline` sans aucune I/O (§7, phase 2).

- [ ] **P2.1** Extraire `ntd-core`
  - Dépend de : PORTE 1
  - Terminé quand : `core/input`, `core/math` et les types réellement partagés sont dans
    `crates/ntd-core` ; `cargo tree -p ntd-core` ne montre aucune dépendance d'I/O.
- [ ] **P2.2** Supprimer la dépendance `drivers` → `engine::state`
  - Dépend de : P2.1
  - Terminé quand : `parsers/wacom/intuos_v1.rs` et `pl.rs` n'importent plus `engine::state` (état
    local au parser, ou utilitaire déplacé dans `ntd-core`) ; golden files identiques.
- [ ] **P2.3** Extraire `ntd-tablets`
  - Dépend de : P2.2
  - Terminé quand : parsers, modèles et chargement des descripteurs dans `crates/ntd-tablets` ;
    aucune I/O ; golden files branchés dessus.
- [ ] **P2.4** `ntd-codegen` (§15.1)
  - Dépend de : P2.3
  - Terminé quand : `tablets/*.json` est validé et génère `crates/ntd-tablets/src/generated/` ;
    `include_dir` et le parsing JSON au démarrage sont supprimés ; job CI `codegen` ;
    `tools/ntd-codegen/AGENTS.md` et `crates/ntd-tablets/AGENTS.md` écrits.
- [ ] **P2.5** Extraire `ntd-pipeline` avec `trait Filter` (§4.4)
  - Dépend de : P2.1
  - Terminé quand : `engine/pipeline` et la chaîne de filtres sont dans `crates/ntd-pipeline` ; le
    cycle `filters` ↔ `engine::plugins` n'existe plus ; latence ≤ référence.
- [ ] **P2.6** Horloge injectée dans le pipeline
  - Dépend de : P2.5
  - Terminé quand : aucun `Instant::now()` ni `sleep` dans `ntd-pipeline` (règle 37).
- [ ] **P2.7** Fuzzing des parsers (§15.4)
  - Dépend de : P2.3
  - Terminé quand : une cible par famille de parser, corpus initialisé avec les captures, job CI
    `fuzz` de 60 s par cible.
- [ ] **PORTE 2**
  - Terminé quand : P2.1 à P2.7 sont `[x]` ; `cargo tree` confirme l'absence d'I/O dans les trois
    crates ; golden files identiques ; latence ≤ référence.

---

### Phase 3 — Extraire les adaptateurs I/O

Objectif : chaque accès à l'OS derrière un trait, avec un fake et la place pour chaque plateforme
(§7, phase 3 ; §16.3).

- [ ] **P3.1** `ntd-test-support`
  - Dépend de : PORTE 2
  - Terminé quand : fakes de `HidBackend`, `OutputSink`, `Clock` utilisables par tous les crates
    en dev-dependency.
- [ ] **P3.2** `ntd-platform` (§16.4)
  - Dépend de : P3.1
  - Terminé quand : démarrage auto, instance unique, priorité de thread et timer haute résolution
    sont dans ce crate, avec la structure `windows` / `linux` / `macos` / `unsupported`.
- [ ] **P3.3** `ntd-hid`
  - Dépend de : P3.1
  - Terminé quand : trait `HidBackend` avec option « exclusif » et erreur `PermissionDenied` ;
    seul crate qui dépend de `hidapi` ; structure §16.3.
- [ ] **P3.4** `ntd-output`
  - Dépend de : P3.1
  - Terminé quand : trait `OutputSink` avec `capabilities()` ; implémentations Windows et Linux ;
    `macos` renvoie `Unsupported`.
- [ ] **P3.5** `ntd-interop`
  - Dépend de : P3.1
  - Terminé quand : protocole pur séparé du transport ; noms de mémoire partagée ≤ 31
    caractères ; cible fuzz du protocole.
- [ ] **P3.6** `ntd-settings`
  - Dépend de : P3.1
  - Terminé quand : `Paths` injecté (plus aucun `get_*_dir()` global) ; fichiers versionnés ;
    écriture atomique ; un fichier corrompu est sauvegardé et signalé (§8).
- [ ] **P3.7** `ntd-plugin-host`
  - Dépend de : P2.5, P3.6
  - Terminé quand : signature vérifiée avant chargement ; target triple dans le manifeste et
    refus des autres cibles ; un plugin qui panique est désactivé sans arrêter le pipeline ;
    `crates/ntd-plugin-api/AGENTS.md` écrit.
- [ ] **P3.8** Supprimer la feature `desktop`
  - Dépend de : P3.3, P3.4, P3.5
  - Terminé quand : la feature n'existe plus ; `apps/sdk` ne dépend que de `ntd-core`,
    `ntd-tablets`, `ntd-hid`, `ntd-pipeline`.
- [ ] **P3.9** Job CI `check-targets` (§16.2)
  - Dépend de : P3.2, P3.3, P3.4, P3.5
  - Terminé quand : `cargo clippy` passe sur toutes les cibles de niveau 3, et le job est bloquant.
- [ ] **PORTE 3**
  - Terminé quand : P3.1 à P3.9 sont `[x]` ; aucun code spécifique à un OS hors des crates
    autorisés (règle 108) ; golden files, latence et E2E identiques.

---

### Phase 4 — Le moteur

Objectif : `ntd-engine` piloté uniquement par `EngineHandle` et `EngineEvent` (§4.5 ; §7, phase 4).

- [ ] **P4.1** Squelette `ntd-engine`
  - Dépend de : PORTE 3
  - Terminé quand : `Engine::start`, `EngineHandle`, `EngineEvent`, `EngineDeps` existent comme
    en §4.5 ; plus aucun `Arc<Mutex<…>>` exposé hors du crate.
- [ ] **P4.2** Découper `polling.rs` et `owner.rs` (plusieurs PR)
  - Dépend de : P4.1
  - Terminé quand : lecture, parsing, pipeline, sortie et boutons sont des modules séparés, aucun
    fichier de plus d'environ 400 lignes (règle 29).
- [ ] **P4.3** Configuration sans verrou dans le chemin chaud
  - Dépend de : P4.1
  - Terminé quand : la configuration arrive par canal ou swap atomique (règle 35) ; aucune
    allocation dans la boucle (règle 33), vérifié par un test ou un compteur d'allocations.
- [ ] **P4.4** Télémétrie sortie du moteur
  - Dépend de : P4.1
  - Terminé quand : `ntd-engine` ne référence plus la télémétrie ; il publie des `EngineEvent`.
- [ ] **P4.5** `ntd-tester` (§15.2)
  - Dépend de : P4.1, P3.1
  - Terminé quand : chaque capture a au moins un scénario dans `qa/scenarios/` ; job CI `tester` ;
    `crates/ntd-engine/AGENTS.md` écrit.
- [ ] **PORTE 4**
  - Terminé quand : P4.1 à P4.5 sont `[x]` ; `ntd-tester` vert sur toutes les captures ;
    latence ≤ référence.

---

### Phase 5 — Services et application

Objectif : `apps/desktop` ne fait plus que du câblage (§7, phase 5).

- [ ] **P5.1** `ntd-services`
  - Dépend de : PORTE 4
  - Terminé quand : websocket (lié à `127.0.0.1` par défaut), autoupdate, télémétrie (opt-in) et
    supervisor sont dans ce crate et consomment des `EngineEvent`.
- [ ] **P5.2** `ntd-ipc`
  - Dépend de : PORTE 4
  - Terminé quand : les types générés en P0.8 viennent de ce crate ; `crates/ntd-ipc/AGENTS.md`
    écrit.
- [ ] **P5.3** `apps/desktop` en câblage seul
  - Dépend de : P5.1, P5.2
  - Terminé quand : aucune logique métier dans `apps/desktop` ; chaque commande fait au plus
    10 lignes.
- [ ] **P5.4** Manifeste racine virtuel
  - Dépend de : P5.3
  - Terminé quand : le `Cargo.toml` racine n'a plus de `[package]` ; le dossier `src/` racine
    n'existe plus.
- [ ] **P5.5** `ntd-stress` (§15.5)
  - Dépend de : P5.1
  - Terminé quand : l'outil couvre les modes de §15.5 et ne fait pas tomber un NTD lancé.
- [ ] **PORTE 5**
  - Terminé quand : P5.1 à P5.5 sont `[x]` ; le graphe `cargo tree` correspond à §4.3 ; E2E,
    `ntd-tester` et golden files verts.

---

### Phase 6 — Contrat v3 et frontend

Objectif : le contrat découpé et typé, et la couche de données du frontend corrigée (§5, §6.3).

- [ ] **P6.1** Découper `Config` par domaine côté backend
  - Dépend de : PORTE 5
  - Terminé quand : `Mapping`, `PenSettings`, `Bindings`, `FilterChain`, `AppPreferences`,
    `WebSocketSettings` ont chacun leurs commandes `get_*` / `update_*` ; `set_config(tout)`
    n'existe plus.
- [ ] **P6.2** Types forts dans le contrat
  - Dépend de : P6.1
  - Terminé quand : bindings, modes et types de courbe sont des `enum` ; les commandes renvoient
    `Result<T, IpcError>` typé.
- [ ] **P6.3** Flux temps réel par `Channel`
  - Dépend de : P6.1
  - Terminé quand : `tablet-event` passe par `tauri::ipc::Channel`, décimé côté backend à environ
    60 Hz.
- [ ] **P6.4** Une seule porte vers Tauri dans le frontend
  - Dépend de : P6.1
  - Terminé quand : règle ESLint `no-restricted-imports` active ; plus d'`invoke` dans
    `MenuBar.svelte` ni de `listen` dans `TabletVisualizer.svelte`.
- [ ] **P6.5** Stores par domaine
  - Dépend de : P6.2, P6.4
  - Terminé quand : un store par domaine ; plus de `JSON.stringify` pour comparer ni de
    `setTimeout` de synchronisation ; le frontend affiche l'état renvoyé par le backend.
- [ ] **P6.6** Capacités et plateforme dans l'interface
  - Dépend de : P6.5
  - Terminé quand : `os.ts` expose OS, architecture et capacités ; une option non disponible est
    masquée ou désactivée ; `TitleBar.svelte` et les raccourcis ne testent plus l'OS eux-mêmes.
- [ ] **P6.7** Migration des profils v2 → v3
  - Dépend de : P6.1
  - Terminé quand : de vrais fichiers v2 en fixtures sont importés sans perte ; l'original est
    sauvegardé ; cible fuzz de chargement des profils.
- [ ] **PORTE 6**
  - Terminé quand : P6.1 à P6.7 sont `[x]` ; E2E verts ; profils v2 réels importés sans perte.

---

### Phase 7 — Release 3.0

Objectif : publier la v3 sans laisser un seul utilisateur v2 derrière (§2, §13).

- [ ] **P7.1** Canaux d'update `beta` et `nightly`
  - Dépend de : PORTE 6
  - Terminé quand : chaque canal a son endpoint stable ; l'option « recevoir les bêtas » existe dans
    les paramètres ; le générateur de manifeste prend la liste des cibles en entrée (§16.3).
- [ ] **P7.2** `prerelease.yml` (§13.4)
  - Dépend de : P7.1
  - Terminé quand : le workflow vérifie le tag, la version et la signature avec la clé embarquée,
    puis publie une pre-release avec les SHA-256.
- [ ] **P7.3** `nightly.yml` (§13.5)
  - Dépend de : P7.1
  - Terminé quand : build de nuit seulement s'il y a eu des commits, tag `nightly` écrasé, fuzzing
    long et `ntd-stress` exécutés.
- [ ] **P7.4** `release.yml` (§13.6)
  - Dépend de : P7.2
  - Terminé quand : publication en `latest`, `latest.json` multi-cibles, notes générées avec
    `rustc -Vv`, winget.
- [ ] **P7.5** Dernière 2.x de transition
  - Dépend de : P7.4
  - Terminé quand : une 2.x publiée sait recevoir la 3.0 par l'auto-update (et pointe vers un
    nouvel endpoint si l'endpoint change).
- [ ] **P7.6** `3.0.0-beta.1` et période de bêta
  - Dépend de : P7.2, P7.5
  - Terminé quand : la bêta est publiée ; aucun bug bloquant ouvert sur les plateformes de niveau 1
    pendant la durée de bêta annoncée.
- [ ] **P7.7** `3.0.0`
  - Dépend de : P7.6
  - Terminé quand : la release est publiée ; README et `docs/` à jour, avec les niveaux de support
    de §16.2 (règle 112).
- [ ] **P7.8** Annonce de maintenance de la 2.x
  - Dépend de : P7.7
  - Terminé quand : la durée de maintenance de `release/2.x` est annoncée dans les notes de
    version.
- [ ] **PORTE 7 — v3.0 terminée**

---

### Après la 3.0 — Plateformes (§16)

Ces tâches n'ont pas d'ordre imposé entre elles. Chacune fait passer une plateforme au niveau
supérieur selon les critères de §16.2.

- [ ] **P8.1** Windows ARM64 et Linux ARM64 : niveau 2 → 1
- [ ] **P8.2** macOS Apple Silicon : niveau 3 → 2 (permissions, ouverture exclusive, injection,
  signature et notarisation, décision sur les plugins tiers, §16.5)
- [ ] **P8.3** macOS Intel : niveau 3 → 2 (binaire universel avec P8.2)
- [ ] **P8.4** Windows x86 32 bits : niveau 3 → 2
- [ ] **P8.5** Linux x86 32 bits : niveau 3 → 2
- [ ] **P8.6** Linux ARM 32 bits : niveau 3 → 2
- [ ] **P8.7** macOS : niveau 2 → 1

