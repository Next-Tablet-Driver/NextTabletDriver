# Frontend rewrite: Svelte 5 to React + Untitled UI

Status: **in progress** on the branch `feat/frontend-react`. Nothing here is merged; the maintainer
decides after seeing the result. This is a decision of the maintainer taken on 2026-11-01, between
phase 3 and phase 4 of [`v3-rewrite.md`](v3-rewrite.md). It overrides rule 6 of §9.1 ("DO NOT
rewrite the frontend") for this branch only.

## Goals

- Rebuild the whole interface in **React 19 + TypeScript**, designed with
  [Untitled UI React](https://www.untitledui.com/react/docs/installation) (`untitledui@latest`:
  Tailwind CSS 4, React Aria Components).
- Re-design freely: the current look does not have to be kept, as long as the interface respects
  the application's purpose and the UI/UX rules (keyboard access, contrast, focus, reduced motion,
  no state hidden from the user, honest empty and error states).
- **Do not lose the Svelte frontend**: see [Where the old frontend is](#where-the-old-frontend-is).

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

- [x] **R0** Back up the Svelte frontend, create the branch
  - Done when: tag `frontend-svelte-v3` and branch `archive/frontend-svelte` exist;
    `feat/frontend-react` is pushed to `origin`; this plan is written.
- [ ] **R1** Foundation: Vite + React 19 + Tailwind 4 + Untitled UI in `frontend-react/`
  - Done when: `npx untitledui@latest init --vite` ran; ESLint, Prettier, TypeScript (strict,
    `noUncheckedIndexedAccess`) and Vitest + Testing Library for React are configured; `npm run
    dev`, `build`, `check`, `lint`, `test` pass on an empty app.
- [ ] **R2** Infrastructure layer (ported, not redesigned)
  - Depends on: R1
  - Done when: `tauri/{bindings,commands,events,window}.ts`, `os.ts`, `log.ts` and the dev
    `tauri-mock` are in `frontend-react/src/infrastructure/` with their unit tests; hooks for
    events and commands (`useCommand`, `useTabletEvent`) exist; no component imports `@tauri-apps`.
- [ ] **R3** Design system and theming
  - Depends on: R1
  - Done when: design tokens (Untitled UI theme variables) with light, dark and the user themes
    (`theme/`: `apply`, `contrast`, `validate`, `template` ported with tests); the components the
    app needs exist (button, input, select, checkbox, tabs, tooltip, dropdown menu, slider/curve
    inputs, card, badge, table, empty state, toast) from Untitled UI where it has them; a dev page
    shows them all in every theme; contrast of every token pair is tested.
- [ ] **R4** Application shell
  - Depends on: R2, R3
  - Done when: title bar (window controls), menu bar, tab bar, footer (tablet status, version),
    page switching, detached windows (`?view=`), keyboard navigation and focus order work; the
    `shell` e2e passes.
- [ ] **R5** Settings page (general, language, theme, WebSocket)
  - Depends on: R4
  - Done when: the `settings` e2e passes; the theme editor validates contrast and imports/exports.
- [ ] **R6** Output page and the mapping visualisers (tablet and display areas)
  - Depends on: R4
  - Done when: `mapping` state is a store (`useSyncExternalStore`), the two visualisers are
    accessible (keyboard moves and resizes the area, values are also numeric inputs); the `output`
    e2e passes.
- [ ] **R7** Pen settings page (bindings, pressure curve, thresholds, hardware toggles)
  - Depends on: R4
  - Done when: the `pen` e2e passes; binding capture is keyboard-accessible.
- [ ] **R8** Filters page (plugin list, details, properties, untrusted plugins)
  - Depends on: R4
  - Done when: the `filters` e2e passes; a plugin refused for its ABI or target is explained.
- [ ] **R9** Console, Debugger and Performance pages
  - Depends on: R4
  - Done when: the `console` e2e passes; large logs are virtualised; the live stream is throttled
    to the display rate.
- [ ] **R10** Release and Credits pages
  - Depends on: R4
  - Done when: the `pages` e2e passes.
- [ ] **R11** UX review
  - Depends on: R5 to R10
  - Done when: keyboard-only pass of every page, `prefers-reduced-motion`, 200 % zoom, narrow
    window (the minimum window size of the app), contrast checked in every theme, empty / loading
    / error states for every command that can fail.
- [ ] **R12** Test parity
  - Depends on: R5 to R10
  - Done when: the whole Playwright suite passes against the React app; unit tests cover what the
    Svelte unit tests covered; `e2e-native` smoke passes.
- [ ] **R13** The switch (only after the maintainer accepts the result)
  - Depends on: R11, R12, maintainer decision
  - Done when: `frontend/` (Svelte) moved out of the tree (it stays in the tag and the archive
    branch), `frontend-react/` renamed `frontend/`, `tauri.conf.json`, CI, `AGENTS.md`,
    `docs/development.md`, `docs/architecture.md`, `.github/CONTRIBUTING.md` and dependabot
    updated; Svelte dependencies gone.

## Mapping of the old structure

| Svelte (`frontend/src`) | React (`frontend-react/src`) |
|---|---|
| `App.svelte`, `main.ts` | `App.tsx`, `main.tsx` |
| `lib/infrastructure/*` | `infrastructure/*` (same files) |
| `lib/theme/*`, `styles/*` | `theme/*`, `styles/theme.css` (Untitled UI tokens) |
| `lib/components/ui/*` | `components/base/*` (Untitled UI) |
| `lib/components/{TitleBar,MenuBar,TabBar,Footer}.svelte` | `components/shell/*` |
| `lib/pages/*.svelte` | `pages/*.tsx` |
| `lib/features/*/*.svelte` | `features/*/*.tsx` |
| `*.svelte.ts` stores (runes) | `*.store.ts` with `useSyncExternalStore` |
| `dev/DesignSystem.svelte` | `dev/DesignSystem.tsx` |

## Risks

- The interface is large (about 7,000 lines of Svelte, 4,500 of them in the feature components):
  expect many sessions. Each sub-task is small enough to be reviewed on its own.
- Untitled UI PRO components need a licence; only the free components are used unless the
  maintainer provides a key.
- Hot path: the live pen stream must not re-render the tree on every packet (R2 and R6 decide how
  the stream is decimated and consumed).
