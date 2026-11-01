import eslint from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  // Code copied in by `npx untitledui add` is the library's, not ours: it is updated with
  // `--overwrite`, so it is not linted or reformatted by hand.
  {
    ignores: [
      "dist",
      "coverage",
      "src/components/**",
      "src/hooks/use-*.ts",
      "src/utils/is-react-component.ts",
      // Generated from the Rust code.
      "src/infrastructure/tauri/bindings.ts",
    ],
  },

  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  reactHooks.configs.flat.recommended,

  {
    languageOptions: {
      globals: { ...globals.browser },
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
  },

  // `console` belongs to the logging module only (see docs/frontend-react.md).
  {
    files: ["src/**/*.{ts,tsx}"],
    ignores: ["src/infrastructure/log.ts", "src/**/__tests__/**"],
    rules: { "no-console": "error" },
  },

  // Playwright fixtures are functions called `use`, which the hooks rules mistake for React's.
  {
    files: ["e2e/**/*.ts"],
    rules: { "react-hooks/rules-of-hooks": "off" },
  },

  // Config files are not part of the app project.
  {
    files: ["*.config.{js,ts}"],
    extends: [tseslint.configs.disableTypeChecked],
    languageOptions: { globals: { ...globals.node } },
  },
);
