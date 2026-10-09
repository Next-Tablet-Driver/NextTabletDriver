import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'

// Development-only tooling (stripped from production builds): `/?mock` runs the real app
// against fixture data in a plain browser, and `window.__ntdSnapshot` compares computed
// styles before/after a CSS refactor.
if (import.meta.env.DEV) {
  if (new URLSearchParams(window.location.search).has('mock')) {
    const { installTauriMock } = await import('./dev/tauri-mock')
    installTauriMock()
  }
  const { installStyleSnapshot } = await import('./dev/style-snapshot')
  installStyleSnapshot()
}

const el = document.getElementById('app');
let app;
if (el !== null) {
  app = mount(App, {
    target: el,
  });
}

export default app
