# frontend-react

The React 19 rewrite of the interface, designed with
[Untitled UI React](https://www.untitledui.com/react/docs/installation). It grows beside the Svelte
`frontend/` until the switch; see [`docs/frontend-react.md`](../docs/frontend-react.md) for the
plan, the sub-tasks and where the old frontend is kept.

```bash
npm install
npm run dev            # Vite dev server
npm run check          # tsc
npm run lint
npm run format:check
npm run test -- --run
```

Add an Untitled UI component with `npx untitledui@latest add <name> -y`. What the CLI copies into
`src/components/base/`, `src/components/foundations/` and `src/utils/` belongs to the library: it
is not linted or reformatted by hand, and `--overwrite` updates it.
