import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import eslintPluginSvelte from "eslint-plugin-svelte";
import globals from "globals";

export default tseslint.config(
  // ─── Base ────────────────────────────────────────────────────────────────────
  eslint.configs.recommended,

  // ─── TypeScript strict + stylistic (type-aware) ──────────────────────────────
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,

  // ─── Svelte strict ───────────────────────────────────────────────────────────
  ...eslintPluginSvelte.configs["flat/recommended"],

  // ─── Global language options ─────────────────────────────────────────────────
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      parserOptions: {
        // Type-aware linting — required for strictTypeChecked rules
        projectService: true,
        extraFileExtensions: [".svelte"],
      },
    },
  },

  // ─── Svelte files: use TS parser inside <script> ─────────────────────────────
  {
    files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        projectService: true,
        extraFileExtensions: [".svelte"],
      },
    },
    rules: {
      // Svelte-specific strictness
      "svelte/require-each-key": "error",
      "svelte/prefer-writable-derived": "error",
      "svelte/no-unused-svelte-ignore": "error",
      "svelte/no-at-html-tags": "error",
      "svelte/no-reactive-reassign": ["error", { props: true }],
      "svelte/block-lang": ["error", { script: "ts" }],
      "svelte/no-useless-mustaches": "error",
      "svelte/prefer-class-directive": "error",
      "svelte/prefer-style-directive": "error",
      "svelte/shorthand-attribute": "error",
      "svelte/shorthand-directive": "error",
      "svelte/sort-attributes": "warn",
      "svelte/spaced-html-comment": "error",
    },
  },

  // ─── TypeScript & general rules ──────────────────────────────────────────────
  {
    files: ["**/*.ts", "**/*.tsx", "**/*.svelte"],
    rules: {
      // ── TypeScript strictness ──
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-unsafe-assignment": "error",
      "@typescript-eslint/no-unsafe-call": "error",
      "@typescript-eslint/no-unsafe-member-access": "error",
      "@typescript-eslint/no-unsafe-return": "error",
      "@typescript-eslint/no-unsafe-argument": "error",
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-misused-promises": "error",
      "@typescript-eslint/await-thenable": "error",
      "@typescript-eslint/require-await": "error",
      "@typescript-eslint/no-unnecessary-type-assertion": "error",
      "@typescript-eslint/no-unnecessary-condition": "error",
      "@typescript-eslint/no-unnecessary-type-parameters": "error",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
        },
      ],
      "@typescript-eslint/consistent-type-imports": [
        "error",
        {
          prefer: "type-imports",
          fixStyle: "separate-type-imports",
        },
      ],
      "@typescript-eslint/consistent-type-exports": "error",
      "@typescript-eslint/no-import-type-side-effects": "error",
      "@typescript-eslint/explicit-function-return-type": [
        "error",
        {
          allowExpressions: true,
          allowTypedFunctionExpressions: true,
        },
      ],
      "@typescript-eslint/prefer-readonly": "error",
      "@typescript-eslint/prefer-nullish-coalescing": "error",
      "@typescript-eslint/prefer-optional-chain": "error",
      "@typescript-eslint/no-non-null-assertion": "error",
      "@typescript-eslint/strict-boolean-expressions": [
        "error",
        {
          allowString: false,
          allowNumber: false,
          allowNullableObject: false,
        },
      ],
      "@typescript-eslint/switch-exhaustiveness-check": "error",
      "@typescript-eslint/no-shadow": "error",
      "@typescript-eslint/naming-convention": [
        "error",
        { selector: "default", format: ["camelCase"] },
        { selector: "variable", format: ["camelCase", "UPPER_CASE", "PascalCase"] },
        { selector: "parameter", format: ["camelCase"], leadingUnderscore: "allow" },
        // Keys that must be quoted anyway (`'bg-app'`, `'--accent'`) cannot be camelCase.
        { selector: "property", modifiers: ["requiresQuotes"], format: null },
        {
          selector: "property",
          format: ["camelCase", "snake_case", "PascalCase", "UPPER_CASE"],
          leadingUnderscore: "allowDouble",
          trailingUnderscore: "allowDouble",
        },
        { selector: "typeLike", format: ["PascalCase"] },
        { selector: "enumMember", format: ["PascalCase", "UPPER_CASE"] },
        { selector: "import", format: ["camelCase", "PascalCase"] },
      ],

      // ── General code quality ──
      eqeqeq: ["error", "always"],
      // Roadmap P1.12: warnings until the code is clean (P1.13), errors after (P1.14).
      "no-console": "warn",
      "no-restricted-imports": [
        "warn",
        {
          patterns: [
            {
              group: ["@tauri-apps/*"],
              message:
                "Import @tauri-apps/* only from infrastructure/tauri/ and infrastructure/os.ts.",
            },
          ],
        },
      ],
      "@typescript-eslint/ban-ts-comment": [
        "warn",
        {
          "ts-expect-error": "allow-with-description",
          "ts-ignore": true,
          "ts-nocheck": true,
          "ts-check": false,
          minimumDescriptionLength: 10,
        },
      ],
      "no-debugger": "error",
      "no-alert": "error",
      "no-var": "error",
      "prefer-const": "error",
      "prefer-template": "error",
      "no-duplicate-imports": "error",
      "no-return-assign": "error",
      "no-throw-literal": "error",
      "no-useless-rename": "error",
      "object-shorthand": "error",
      complexity: ["error", { max: 10 }],
      "max-depth": ["error", { max: 4 }],
    },
  },

  // ─── Svelte Specific Overrides ───────────────────────────────────────────────
  {
    files: ["**/*.svelte"],
    rules: {
      // Svelte 5 $props() destructured variables trigger prefer-const
      "prefer-const": "off",
      // Svelte 5 uses explicit undefined defaults for props sometimes
      "@typescript-eslint/no-useless-default-assignment": "off",
    },
  },

  // ─── Where the rules above do not apply ──────────────────────────────────────
  {
    // The only code allowed to talk to Tauri, and the tests that mock it.
    files: [
      "src/lib/infrastructure/tauri/**",
      "src/lib/infrastructure/os.ts",
      "**/__tests__/**",
      "**/*.test.ts",
      "e2e/**",
      "e2e-native/**",
    ],
    rules: { "no-restricted-imports": "off" },
  },
  {
    // The frontend's only logger.
    files: ["src/lib/infrastructure/log.ts"],
    rules: { "no-console": "off" },
  },

  // ─── Ignores ──────────────────────────────────────────────────────────────────
  {
    ignores: [
      "build/",
      ".svelte-kit/",
      "dist/",
      "public/",
      "src/lib/infrastructure/tauri/bindings.ts",
      "vite.config.ts",
      "vite.config.js",
      "vite.config.d.ts",
      "svelte.config.js",
      "eslint.config.js",
    ],
  },
);
