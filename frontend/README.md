# Frontend

This folder has the interface of NextTabletDriver. It is a [Svelte](https://svelte.dev) 5 and TypeScript app, built with [Vite](https://vite.dev). The desktop app ([`apps/desktop`](../apps/desktop)) shows it inside a window. The engine itself is written in Rust and lives in the root crate. To see how the pieces fit together, read [Architecture](../docs/architecture.md).

## Stack

| Tool                                                    | Used for                                                            |
| ------------------------------------------------------- | ------------------------------------------------------------------- |
| Svelte 5 (with runes) and TypeScript                    | The interface.                                                      |
| Vite                                                    | Dev server and production build.                                    |
| Vitest and Testing Library (jsdom)                      | Unit and component tests.                                           |
| Playwright                                              | End-to-end tests of every screen, and a smoke test of the real app. |
| ESLint (`typescript-eslint`, `eslint-plugin-svelte`)    | Lint rules.                                                         |
| Tauri API and plugins (dialog, process, shell, updater) | Talk to the Rust side and the operating system.                     |

## Run it

Install the packages once. All commands run from the root of the repository.

```bash
npm --prefix frontend ci
```

| You want to...                                               | Command                                                                                                       |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| Run the full app (Rust and interface) with hot reload        | `npm run tauri-dev`                                                                                           |
| Run the interface alone in your browser, with a fake backend | `npm --prefix frontend run dev`, then open `http://localhost:5173/?mock`                                      |
| Build the interface                                          | `npm --prefix frontend run build`                                                                             |
| Check TypeScript and Svelte types                            | `npm --prefix frontend run check`                                                                             |
| Lint                                                         | `npm --prefix frontend run lint`                                                                              |
| Run the unit tests                                           | `npm --prefix frontend run test -- --run`                                                                     |
| Run the end-to-end tests                                     | `npm --prefix frontend run e2e`                                                                               |
| Run the smoke test of the real built app                     | `npm --prefix frontend run e2e:native` (needs `tauri-driver` and a built app, see `.github/workflows/ci.yml`) |
| See what is in the bundle                                    | `npm --prefix frontend run build:audit`                                                                       |

## Folders

```text
frontend/
├── index.html, vite.config.ts, svelte.config.js, tsconfig*.json, eslint.config.js
├── public/             Fonts, favicon, and theme-boot.js (see "Themes")
├── e2e/                Playwright tests (browser and fake backend)
├── e2e-native/         Smoke test of the real app (WebDriver)
└── src/
    ├── main.ts         Entry point
    ├── App.svelte      The shell: title bar, menus, tabs, and the page for the selected tab
    ├── styles/         Design tokens (tokens.css) and the built-in themes (themes.css)
    ├── dev/            Tools for developers only, never shipped (see below)
    └── lib/
        ├── components/        Shell parts (TitleBar, MenuBar, TabBar, Footer) and small shared controls in ui/
        ├── pages/             One file per tab, plus the Debugger and Performance windows
        ├── features/          Parts of each page: mapping, pen, filters, console, settings, credits, release
        ├── infrastructure/    The only code that talks to Tauri: commands, events, window, os
        ├── theme/             Theme tokens, validation, loading and applying
        └── test-support/      Helpers for tests
```

Tests sit next to the code they test, in `__tests__` folders, and are named `*.test.ts`.

## Talking to the engine

The interface never reads files or touches the system itself. It calls Rust _commands_ and listens to _events_.

- **Commands** are typed wrappers in [`src/lib/infrastructure/tauri/commands.ts`](src/lib/infrastructure/tauri/commands.ts). Use them in your components instead of calling `invoke` with a string. Each wrapper matches one `#[tauri::command]` function in [`apps/desktop/src/commands.rs`](../apps/desktop/src/commands.rs).
- **Events** from Rust, such as `tablet-event` (pen data, about 60 times a second) and `device-changed`, are in `infrastructure/events.ts`.
- **The settings type** `Config` in `commands.ts` mirrors `MappingConfig` in `src/core/config/models.rs`. If you add a field on one side, add it on the other side, and give it a default in Rust so old settings files still load.

### The fake backend

With `?mock` in the address, `src/dev/tauri-mock.ts` replaces Tauri. It answers every command with sample data and records each call. The end-to-end tests use it, so they need no tablet and no native window. When you add a command, add it to the mock too. Tests fail on any `console.error` or uncaught error, which catches a missing mock quickly.

## Themes

Colors, corner shapes and fonts are CSS custom properties (design tokens). `src/lib/theme/tokens.ts` lists every token that a theme may change, `validate.ts` checks user theme files, and `apply.ts` applies the result. `public/theme-boot.js` runs before the first paint and restores the last theme, so a light theme does not flash dark at start-up. The page for users is [Themes](../docs/themes.md).

When you write CSS, use the tokens (`var(--accent)`, `var(--space-4)`) and not fixed colors or sizes, so every theme works.

## Developer tools

In `src/dev/`. They only exist in development and are removed from production builds.

- `tauri-mock.ts`: the fake backend described above.
- `DesignSystem.svelte`: a gallery of the shared controls. Open `http://localhost:5173/?view=design-system` in the dev server.
- `style-snapshot.ts`: saves and compares the computed styles of a screen, to prove that a CSS refactor did not change how it looks. The snapshots go to `frontend/.snapshots/`, which git ignores.

## Rules

- Write TypeScript. `npm run lint` and `npm run check` must pass with no warnings.
- Put new Tauri calls in `infrastructure/` and use them from components. Some older components still import Tauri plugins directly. Do not copy that.
- Do not trust data that comes from outside the app: theme files, plugin manifests and anything the user imports. Validate it (see `theme/validate.ts`).
- Write tests for new components and for logic. Unit tests should not need a real backend.
- The window has no native frame. The title bar and window buttons are part of the interface (`TitleBar.svelte`).
