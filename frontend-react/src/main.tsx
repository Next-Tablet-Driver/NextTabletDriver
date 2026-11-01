import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@/app.css";
import App from "@/App";

// Development-only tooling (stripped from production builds): `/?mock` runs the real app against
// fixture data in a plain browser, which is also how the end-to-end tests run it.
if (import.meta.env.DEV && new URLSearchParams(window.location.search).has("mock")) {
  const { installTauriMock } = await import("@/dev/tauri-mock");
  installTauriMock();
}

const root = document.getElementById("app");
if (root === null) {
  throw new Error("index.html has no #app element");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
