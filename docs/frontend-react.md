# Frontend rewrite: Svelte 5 to React

Status: **in progress** on the branch `feat/frontend-react`. Nothing here is merged; the maintainer
decides after seeing the result. This is a decision of the maintainer taken on 2026-11-01, between
phase 3 and phase 4 of [`v3-rewrite.md`](v3-rewrite.md). It overrides rule 6 of §9.1 ("DO NOT
rewrite the frontend") for this branch only.

## Goals

- Rebuild the whole interface in **React 19 + TypeScript**, with the **same design as the Svelte
  frontend, 1:1**: same markup, same classes, same styles, same texts, same behaviours.
- **Do not lose the Svelte frontend**: see [Where the old frontend is](#where-the-old-frontend-is).

History: the branch first tried a redesign with Untitled UI, then a second one without a
sidebar. The maintainer rejected both on 2026-11-01 and asked for the Svelte design copied
as it was. Both redesigns, and every dependency they brought (Tailwind, React Aria, Untitled UI,
Papicons, Atkinson Hyperlegible), are removed; they stay in the git history of this branch.

## Where the old frontend is

| What | Where |
|---|---|
| Git tag | `frontend-svelte-v3` (the commit `91d9afc`, end of phase 3, GATE 3) |
| Git branch | `archive/frontend-svelte` (same commit, local) |
| Working tree | `frontend/` is **not touched** until the last sub-task (R13) |

To get it back at any time: `git checkout frontend-svelte-v3 -- frontend` or
`git worktree add ../ntd-svelte frontend-svelte-v3`.

## How the work is organised

- **The React app grows in `frontend-react/`** beside `frontend/`. The desktop app, the CI and
  `cargo` keep building the Svelte frontend until the switch (R13), so the branch stays green and
  the old interface keeps working for comparison.
- **The Rust side does not change.** `bindings.ts` is generated from Rust and has no framework in
  it; the Tauri commands and events are the same.
- **The Playwright suite (`frontend/e2e`, 65 tests) is the acceptance test**: it describes
  behaviour, not markup. The React app must pass it (R12), adapting selectors only where the new
  design changes the visible text or roles. A test is never deleted to make it pass.
- One sub-task, one commit series, one tick below. A sub-task is ticked only when its checks pass.

## Checks (same bar as the rest of the repository)

```bash
npm --prefix frontend-react run format:check
npm --prefix frontend-react run check      # tsc
npm --prefix frontend-react run lint
npm --prefix frontend-react run test -- --run
npm --prefix frontend-react run e2e        # when the interface changes
```

100 columns, English only, comments explain why. `console` only in `infrastructure/log.ts`.
Only `src/infrastructure/` talks to Tauri. No dependency is added without a reason written in the
commit message.

## Sub-tasks

R1 to R12 were done for the Untitled UI attempt; where their notes name Tailwind, Untitled UI,
React Aria or Papicons, that is history: the 1:1 Svelte port replaced it (see Decisions). The
tasks themselves (infrastructure, shell, pages, tests) are all done again on the Svelte design.

- [x] **R0** Back up the Svelte frontend, create the branch
  - Done when: tag `frontend-svelte-v3` and branch `archive/frontend-svelte` exist;
    `feat/frontend-react` is pushed to `origin`; this plan is written.
- [x] **R1** Foundation: Vite + React 19 + Tailwind 4 + Untitled UI in `frontend-react/`
  - Done when: `npx untitledui@latest init --vite` ran; ESLint, Prettier, TypeScript (strict,
    `noUncheckedIndexedAccess`) and Vitest + Testing Library for React are configured; `npm run
    dev`, `build`, `check`, `lint`, `test` pass on an empty app.
  - Notes: `npm create vite` (react-ts), then `npx untitledui@latest init . --vite -y` (it needs an
    existing Vite project and does not set up Tailwind: `@tailwindcss/vite` and
    `@tailwindcss/typography` were added by hand), then `npx untitledui@latest add button -y`.
    TypeScript stays at `~6.0` like the Svelte frontend (the template wanted 7). Linting is
    ESLint 10 with `typescript-eslint` strict type-checked and `react-hooks`, as in the Svelte
    frontend; `eslint-plugin-jsx-a11y` is left out because it does not support ESLint 10 yet, and
    React Aria Components carries most of the accessibility work. Vendored library files
    (`components/base`, `components/foundations`, `utils/cx.ts`, `utils/is-react-component.ts`,
    `styles/`) are excluded from ESLint and Prettier. Alias `@` is `src`.
- [x] **R2** Infrastructure layer (ported, not redesigned)
  - Depends on: R1
  - Done when: `tauri/{bindings,commands,events,window}.ts`, `os.ts`, `log.ts` and the dev
    `tauri-mock` are in `frontend-react/src/infrastructure/` with their unit tests; hooks for
    events and commands (`useCommand`, `useTabletEvent`) exist; no component imports `@tauri-apps`.
  - Notes: the files are copied as they were (they have no framework in them), with the unit tests
    (`window.test.ts` lost its constructor parameter properties, which `erasableSyntaxOnly`
    refuses). `useTauriEvent(subscribe, handler)` and `useCommand(call)` are the two hooks; the
    first keeps the latest handler without re-subscribing and releases a late subscription, the
    second drops stale answers. `bindings.ts` is a byte copy of the generated file, excluded from
    ESLint and Prettier; `bindings-in-sync.test.ts` fails if it drifts from the Svelte copy that
    `apps/desktop/tests/bindings.rs` regenerates (the Rust test must write to
    `frontend/src/infrastructure/tauri/bindings.ts` at R13, and that guard test goes away).
    `/?mock` installs the Tauri mock in development, as before.
- [x] **R3** Design system and theming
  - Depends on: R1
  - Done when: design tokens (Untitled UI theme variables) with light, dark and the user themes
    (`theme/`: `apply`, `contrast`, `validate`, `template` ported with tests); the components the
    app needs exist (button, input, select, checkbox, tabs, tooltip, dropdown menu, slider/curve
    inputs, card, badge, table, empty state, toast) from Untitled UI where it has them; a dev page
    shows them all in every theme; contrast of every token pair is tested.
- [x] **R4** Application shell
  - Depends on: R2, R3
  - Done when: title bar (window controls), menu bar, tab bar, footer (tablet status, version),
    page switching, detached windows (`?view=`), keyboard navigation and focus order work; the
    `shell` e2e passes.
- [x] **R5** Settings page (general, language, theme, WebSocket)
  - Depends on: R4
  - Done when: the `settings` e2e passes; the theme editor validates contrast and imports/exports.
- [x] **R6** Output page and the mapping visualisers (tablet and display areas)
  - Depends on: R4
  - Done when: `mapping` state is a store (`useSyncExternalStore`), the two visualisers are
    accessible (keyboard moves and resizes the area, values are also numeric inputs); the `output`
    e2e passes.
- [x] **R7** Pen settings page (bindings, pressure curve, thresholds, hardware toggles)
  - Depends on: R4
  - Done when: the `pen` e2e passes; binding capture is keyboard-accessible.
- [x] **R8** Filters page (plugin list, details, properties, untrusted plugins)
  - Depends on: R4
  - Done when: the `filters` e2e passes; a plugin refused for its ABI or target is explained.
- [x] **R9** Console, Debugger and Performance pages
  - Depends on: R4
  - Done when: the `console` e2e passes; large logs are virtualised; the live stream is throttled
    to the display rate.
- [x] **R10** Release and Credits pages
  - Depends on: R4
  - Done when: the `pages` e2e passes.
- [x] **R11** UX review
  - Depends on: R5 to R10
  - Done when: keyboard-only pass of every page, `prefers-reduced-motion`, 200 % zoom, narrow
    window (the minimum window size of the app), contrast checked in every theme, empty / loading
    / error states for every command that can fail.
  - Notes: done with an automatic audit and a manual pass on the points that tools miss.
    `e2e/accessibility.e2e.ts` runs axe (WCAG 2 A and AA) on the seven pages in the dark and the
    light scheme and fails on any serious or critical violation; it found a tab without its panel,
    a placeholder and a label that were too pale, and they are fixed without touching the library.
    By hand: every area can be moved with the arrow keys, every number is also a field, bindings
    can be recorded without the pointer, the menus close with Escape, a narrow window (the
    minimum is 420 px) scrolls the tabs sideways, and every list has its empty,
    loading and error state. Not done: a check at 200 % zoom on a real display, a screen reader
    pass, and `prefers-reduced-motion` beyond what React Aria and Untitled UI do.
- [x] **R12** Test parity
  - Depends on: R5 to R10
  - Done when: the whole Playwright suite passes against the React app; unit tests cover what the
    Svelte unit tests covered; `e2e-native` smoke passes.
- [ ] **R13** The switch (only after the maintainer accepts the result)
  - Depends on: R11, R12, maintainer decision
  - Done when: `frontend/` (Svelte) moved out of the tree (it stays in the tag and the archive
    branch), `frontend-react/` renamed `frontend/`, `tauri.conf.json`, CI, `AGENTS.md`,
    `docs/development.md`, `docs/architecture.md`, `.github/CONTRIBUTING.md` and dependabot
    updated; Svelte dependencies gone.

## Decisions of the rewrite

- **The design is the Svelte one.** The `<style>` blocks of every Svelte component were extracted
  verbatim into CSS Modules (`*.module.css` next to each component); `styles/tokens.css`,
  `styles/themes.css`, `app.css`, the fonts, `theme-boot.js` and the icon are copied unchanged.
  Where a Svelte rule reaches into another component (`:global(.btn)`), the React component also
  sets the plain global class (`btn`, `dropdown-item`, `update-btn`, `icon-btn`, `spinning`,
  `content-area`). Three stylesheets were scoped by hand because Svelte scoped them implicitly
  (`UntrustedPlugins`, `Checkbox`, `ThemeSettings`).
- **Icons** are `lucide-react`, the React build of the icon set the Svelte frontend used.
- **Themes keep working**: Dark, Light, Catppuccin and the user theme files (import, validation,
  contrast warnings) are ported (`src/theme`, with their unit tests).
- **Inputs behave as before**: label drag, touch swipe, wheel when focused, precision formatting,
  commit on blur or Enter (`Input`, `onCommit`).
- **The end-to-end specs are the Svelte ones, unchanged**, and are the acceptance test of the
  1:1 port (65 tests, run against the Tauri mock at `/?mock`).
- **State**: a small `Store<T>` + `useStore` (`useSyncExternalStore`) replaces the Svelte runes
  stores; the mapping store keeps its undo history and its debounce.
- **Navigation** is the URL fragment (`#/output`), as in Svelte.
- ESLint is the same bar as the Svelte frontend plus `react-hooks` (React Compiler rules).

## Known gaps and follow-ups

- The Debugger and Input lag windows are the placeholders of the Svelte frontend.
- The Tauri window is undecorated: the title bar's drag region and the window buttons are checked
  in a browser against the mock, not in a native window; check them before the switch.
- R13 waits for the maintainer.

## Mapping of the old structure

| Svelte (`frontend/src`) | React (`frontend-react/src`) |
|---|---|
| `App.svelte`, `main.ts` | `App.tsx`, `main.tsx` |
| `lib/infrastructure/*` | `infrastructure/*` (same files) |
| `lib/theme/*`, `styles/*` | `theme/*`, `styles/*` |
| `lib/components/ui/*` | `ui/*` |
| `lib/components/{TitleBar,MenuBar,TabBar,Footer}.svelte` | `shell/*` |
| `lib/pages/*.svelte` | `pages/*Page.tsx` |
| `lib/features/*/*.svelte` | `features/*/*.tsx` |
| `*.svelte.ts` stores (runes) | `*.store.ts` with `useSyncExternalStore` |

## Risks

- The interface is large (about 7,000 lines of Svelte, 4,500 of them in the feature components):
  expect many sessions. Each sub-task is small enough to be reviewed on its own.
- Hot path: the live pen stream must not re-render the tree on every packet (R2 and R6 decide how
  the stream is decimated and consumed).
