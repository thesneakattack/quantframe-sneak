// @ts-check
import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";

// This project had no lint configuration at all: upstream shipped a `lint` script
// but no config file and never declared eslint as a dependency, so `pnpm lint`
// exited 2 on a clean checkout. This is the first config the codebase has ever had,
// so it starts from the recommended sets rather than a stricter house style.
export default tseslint.config(
  {
    ignores: ["dist/**", "src-tauri/**", "node_modules/**", "public/**", "scripts/**"],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    // Build/config files run in Node, not the browser.
    files: ["*.{js,cjs,mjs}", "scripts/**/*.{js,mjs}"],
    languageOptions: { globals: globals.node },
  },
  {
    // The type surface uses declaration-merging namespaces (TauriTypes, QfTypes,
    // WarframeMarketTypes). Converting them to ES modules would ripple through
    // every import in the app; the pattern is deliberate.
    files: ["src/types/**/*.ts"],
    rules: { "@typescript-eslint/no-namespace": "off" },
  },
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      globals: { ...globals.browser, ...globals.es2021 },
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],

      // Demoted to warnings, deliberately. Both are real debt rather than style, and
      // both are too large to resolve inside a lint pass - so they stay visible as a
      // running count instead of being switched off. See docs/FORK.md.
      //
      // ~600 hits. The codebase uses `any` pervasively; typing it properly is its own
      // project and needs test coverage first.
      "@typescript-eslint/no-explicit-any": "warn",
      //
      // ~455 hits, but ONE architectural pattern, not 455 defects: every module under
      // src/api/ is a class whose methods call useQuery/useMutation, e.g.
      // AppModule.get_settings(). eslint reports this as "cannot be called in a class
      // component", which is misleading - these are namespace classes, not components.
      // The genuine risk is that the methods are not named use*, so nothing stops a
      // caller invoking one from an event handler or a conditional, which would break
      // at runtime. It works today only because callers happen to use them in render.
      "react-hooks/rules-of-hooks": "warn",
      //
      // ~36 hits. Each needs judgement about the intended effect semantics; blanket
      // "fixing" dependency arrays is a reliable way to introduce loops or stale reads.
      "react-hooks/exhaustive-deps": "warn",
      //
      // eslint-plugin-react-hooks v7 ships the React Compiler rule set. These are
      // performance and purity observations about code written years before those
      // rules existed, not defects. Same treatment as the rest of the family.
      "react-hooks/set-state-in-effect": "warn",
      "react-hooks/immutability": "warn",
      "react-hooks/use-memo": "warn",

      // `cb && cb(value)` is used throughout as an optional-callback guard (~30
      // sites). It is intentional, not a stray expression. `cb?.(value)` would read
      // better and is worth a future sweep, but rewriting it is not a lint fix.
      "@typescript-eslint/no-unused-expressions": ["error", { allowShortCircuit: true, allowTernary: true }],

      // `{}` is used deliberately here, both as an empty props type
      // (`type HeaderProps = {}`) and as a generic default
      // (`<T extends Record<string, any> = {}>`). Interfaces still may not be empty.
      "@typescript-eslint/no-empty-object-type": ["error", { allowObjectTypes: "always" }],

      // Leading underscore marks an intentionally unused binding.
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" }],
    },
  },
);
