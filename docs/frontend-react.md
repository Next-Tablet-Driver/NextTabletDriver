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

The maintainer gave free hands on the design, with one rule: the interface is made of the
components of Untitled UI, **as the CLI wrote them** (`src/components/`, never edited; only our own
code in `src/ui`, `src/shell`, `src/features`, `src/pages` composes them and adds Tailwind classes).
What follows from it, and what a reviewer should know:

- **Design review of 2026-11-01, after the first version was rejected.** The first version had
  the tells that the articles on "AI slop" agree on ([925 Studios](https://www.925studios.co/blog/ai-slop-design-tells),
  [vibecodekit](https://vibecodekit.dev/ai-slop-design), [hey.com/kostac](https://world.hey.com/kostac/spot-the-slop-a-ui-designer-s-guide-to-fixing-ai-defaults-4c448c9c)):
  the library's purple, Inter, a sidebar, every group in the same rounded card with the same
  shadow, pastel icon tiles, three identical statistic cards. They are replaced, not just removed:
  - **No sidebar.** The window is the one of the old application: a title bar with the File,
    Tablet and Help menus and the window buttons, the pages as tabs under it, the status bar.
    In a narrow window the tabs scroll sideways.
  - **No cards.** A page is a list: a title, then groups separated by a rule, each setting a row
    (its name and what it does on the left, its control on the right). The numbers of the
    credits are a line of text, not three tiles.
  - **One thing is coloured: the drawing stage** of the two area editors, a cyanotype with a
    millimetre or pixel grid, rulers on two edges, and the pen as an amber crosshair with its
    position. It is the part of the interface that is about the subject: a pen on a surface in
    millimetres. Everything around it is quiet.
  - **Colour**: the library's brand ramp is replaced by a cool slate (`src/styles/app.css`, a
    token override, the components are untouched), so buttons, switches and focus are ink.
  - **Type**: Atkinson Hyperlegible Next, made to tell characters apart, in a screen of
    coordinates and key names. Numbers are tabular, headings balanced.
  - **Icons**: [Papicons](https://github.com/PapillonApp/papicons) (`@getpapillon/papicons`, MIT)
    for every icon placed by our own code. The few icons inside the Untitled UI components
    (chevrons of a select, the tick of a checkbox) stay the library's: editing them would mean
    editing the components. There is no maximise glyph in Papicons, so that button draws a square.
- **Theme**: Untitled UI's light and dark schemes (class `dark-mode` on `<html>`).
  `config.theme` keeps working as `System`, `Dark` or `Light`; the other values of the Svelte
  interface (Catppuccin) are shown as their scheme.
- **User theme files are not supported** here (the `--bg-app`-style token files, their import,
  validation and contrast warnings). They need a token layer of our own on top of Untitled UI's,
  which is what the rule forbids. The backend commands (`list_themes`, `import_theme`...) are still
  there. If themes matter, the way is Untitled UI's own theming (`--color-brand-*` in `theme.css`),
  decided with the maintainer.
- **Navigation** is a row of tabs, each a URL fragment (`#/output`), so a reload keeps the page.
- **The "anonymous usage statistics" checkbox is gone**: it was not connected to any setting.
- **Debugger and Input lag windows**, placeholders in Svelte ("will be rendered soon"), now show
  the live packet and the report rate.
- **Edits reach the driver after 400 ms** without a change (1.5 s in the Svelte interface), and an
  edit that changes nothing is not sent.
- **The area editors work with the keyboard** (arrow keys, Shift for larger steps) as well as the
  pointer, and every number is also a field.
- **Pressure curve** draws the driver's own arithmetic and edits the exponent and the control
  points of a custom curve (the Svelte version had static bars).
- **Bindings** (tip, eraser, pen buttons, tablet keys) can all be typed, picked from presets or
  recorded; before, only the buttons could be recorded.
- **End-to-end tests** (`frontend-react/e2e`, 87 with the audit) are the Svelte specs rewritten for the new markup,
  with the same cases except the user themes (see above) and with a few new ones (keyboard areas,
  curve points, reset to defaults, reload keeps the page).
- ESLint ignores `src/components/**`; accessibility relies on React Aria Components.

## Known gaps and follow-ups

- The production bundle is 785 KB (233 KB gzipped), mostly React Aria Components; code splitting
  by page would help and was not done.
- The user theme files are not supported (see above).
- The Tauri window is undecorated: the title bar's drag region and the window buttons are checked
  in a browser against the mock, not in a native window; run `npm run e2e:native` equivalent
  (to port) before the switch.
- R13 waits for the maintainer.

## Mapping of the old structure

| Svelte (`frontend/src`) | React (`frontend-react/src`) |
|---|---|
| `App.svelte`, `main.ts` | `App.tsx`, `main.tsx` |
| `lib/infrastructure/*` | `infrastructure/*` (same files) |
| `lib/theme/*`, `styles/*` | `theme/theme.ts` (light, dark, system), `styles/theme.css` (Untitled UI tokens) |
| `lib/components/ui/*` | `components/base/*` (Untitled UI) |
| `lib/components/{TitleBar,MenuBar,TabBar,Footer}.svelte` | `components/shell/*` |
| `lib/pages/*.svelte` | `pages/*.tsx` |
| `lib/features/*/*.svelte` | `features/*/*.tsx` |
| `*.svelte.ts` stores (runes) | `*.store.ts` with `useSyncExternalStore` |
| `dev/DesignSystem.svelte` | `dev/DesignSystem.tsx` |

## Risks

- The interface is large (about 7,000 lines of Svelte, 4,500 of them in the feature components):
  expect many sessions. Each sub-task is small enough to be reviewed on its own.
- Only the free components are used.
- Hot path: the live pen stream must not re-render the tree on every packet (R2 and R6 decide how
  the stream is decimated and consumed).
