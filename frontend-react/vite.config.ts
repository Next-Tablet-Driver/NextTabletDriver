import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/setupTests.ts"],
    // The end-to-end specs are run by Playwright (`npm run e2e`).
    exclude: ["**/node_modules/**", "e2e/**", "e2e-native/**"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/components/base/**",
        "src/components/foundations/**",
        "src/dev/**",
        "src/**/__tests__/**",
        "src/**/*.test.{ts,tsx}",
        "src/**/*.d.ts",
        "src/main.tsx",
        "src/setupTests.ts",
      ],
      reporter: ["text-summary", "json-summary", "lcov"],
    },
  },
});
