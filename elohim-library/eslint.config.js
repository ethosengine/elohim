// @ts-check
const eslint = require("@eslint/js");
const tseslint = require("typescript-eslint");
const angular = require("@angular-eslint/eslint-plugin");
const angularTemplate = require("@angular-eslint/eslint-plugin-template");
const angularTemplateParser = require("@angular-eslint/template-parser");
const importPlugin = require("eslint-plugin-import");
const prettierPlugin = require("eslint-plugin-prettier");
const prettierConfig = require("eslint-config-prettier");
const sonarjs = require("eslint-plugin-sonarjs");
const unicorn = require("eslint-plugin-unicorn").default;

// ============================================================
// SHARED TYPESCRIPT RULES
// ============================================================
// Extracted to avoid duplication between elohim-service and lamad-ui/html5-app-plugin blocks.
const sharedTsRules = {
  // ── Type safety ──────────────────────────────────────────
  "@typescript-eslint/no-explicit-any": "error",
  "@typescript-eslint/no-unsafe-assignment": "warn",
  "@typescript-eslint/no-unsafe-member-access": "warn",
  "@typescript-eslint/no-unsafe-call": "warn",
  "@typescript-eslint/no-unsafe-return": "warn",
  "@typescript-eslint/no-unsafe-argument": "warn",

  // ── Unused code ──────────────────────────────────────────
  "@typescript-eslint/no-unused-vars": ["error", {
    argsIgnorePattern: "^_",
    varsIgnorePattern: "^_",
    caughtErrorsIgnorePattern: "^_"
  }],
  "@typescript-eslint/no-empty-function": "warn",

  // ── Code style (SonarQube parity) ────────────────────────
  "@typescript-eslint/consistent-type-definitions": ["error", "interface"],
  "@typescript-eslint/prefer-nullish-coalescing": "error",
  "@typescript-eslint/prefer-optional-chain": "error",
  "@typescript-eslint/prefer-readonly": "error",
  "@typescript-eslint/no-array-constructor": "error",
  "@typescript-eslint/prefer-for-of": "error",
  "@typescript-eslint/prefer-includes": "error",
  "@typescript-eslint/prefer-string-starts-ends-with": "error",

  // ── Promise handling ─────────────────────────────────────
  "@typescript-eslint/no-misused-promises": "error",
  "@typescript-eslint/no-floating-promises": "error",
  "@typescript-eslint/promise-function-async": "warn",
  "@typescript-eslint/require-await": "error",

  // ── Type assertion hygiene ───────────────────────────────
  "@typescript-eslint/no-unnecessary-type-assertion": "error",
  "@typescript-eslint/await-thenable": "error",
  "@typescript-eslint/max-params": ["error", { max: 7 }],

  // ── Deprecated APIs ──────────────────────────────────────
  "@typescript-eslint/no-deprecated": "warn",

  // ── Naming conventions ───────────────────────────────────
  "@typescript-eslint/naming-convention": [
    "error",
    {
      selector: "interface",
      format: ["PascalCase"],
      custom: { regex: "^[A-Z]", match: true }
    },
    {
      selector: "class",
      format: ["PascalCase"]
    },
    {
      selector: "typeAlias",
      format: ["PascalCase"]
    }
  ],

  // ── Import rules ─────────────────────────────────────────
  "import/order": ["error", {
    groups: ["builtin", "external", "internal", "parent", "sibling", "index", "type"],
    "newlines-between": "always",
    alphabetize: { order: "asc", caseInsensitive: true }
  }],
  "import/no-duplicates": "error",
  "import/no-useless-path-segments": "error",

  // ── General best practices ───────────────────────────────
  "no-console": ["error", { allow: ["warn", "error"] }],
  "prefer-const": "error",
  "no-var": "error",
  "eqeqeq": ["error", "always"],
  "no-eval": "error",
  "no-implied-eval": "error",
  "no-new-func": "error",
  "no-throw-literal": "error",

  // ── SonarJS ──────────────────────────────────────────────
  "sonarjs/cognitive-complexity": ["error", 15],
  "sonarjs/no-duplicate-string": ["error", { threshold: 3 }],
  "sonarjs/no-identical-functions": "error",
  "sonarjs/no-collapsible-if": "error",
  "sonarjs/no-redundant-jump": "error",
  "sonarjs/prefer-immediate-return": "error",
  "sonarjs/no-inverted-boolean-check": "error",
  "sonarjs/no-nested-conditional": "error",
  "sonarjs/no-gratuitous-expressions": "error",
  "sonarjs/prefer-single-boolean-return": "error",
  "sonarjs/no-ignored-exceptions": "error",
  "sonarjs/no-unused-vars": "error",
  "sonarjs/no-nested-functions": "off",

  // ── Unicorn (shared subset) ──────────────────────────────
  "unicorn/prefer-set-has": "error",
  "unicorn/no-zero-fractions": "error",
  "unicorn/prefer-number-properties": "error",
  "unicorn/prefer-code-point": "error",
  "unicorn/prefer-array-index-of": "error",
  "unicorn/no-typeof-undefined": "error",
  "unicorn/prefer-export-from": "error",
  "unicorn/prefer-global-this": "error",
  "unicorn/no-negated-condition": "error",
  "unicorn/no-array-push-push": "error",
  "unicorn/prefer-string-raw": "error",
  "unicorn/prefer-array-some": "error",
  "unicorn/prefer-negative-index": "error",
  "unicorn/prefer-at": "error",
  "unicorn/prefer-structured-clone": "error",
  "unicorn/prefer-top-level-await": "off",

  // ── Prettier ─────────────────────────────────────────────
  "prettier/prettier": [process.env.CI === "true" ? "off" : "error"],
  ...prettierConfig.rules
};

module.exports = tseslint.config(
  // ================================================================
  // 1. GLOBAL IGNORES
  // ================================================================
  {
    ignores: [
      "dist/**",
      "node_modules/**",
      "**/*.spec.ts",
      "projects/perseus-plugin/**",
      "src/**"
    ]
  },

  // ================================================================
  // 2. ELOHIM-SERVICE (Node.js library — CommonJS, node moduleResolution)
  // ================================================================
  {
    files: ["projects/elohim-service/src/**/*.ts"],
    extends: [
      eslint.configs.recommended,
      ...tseslint.configs.recommended,
      ...tseslint.configs.stylistic,
      sonarjs.configs.recommended,
    ],
    plugins: {
      "import": importPlugin,
      "prettier": prettierPlugin,
      "unicorn": unicorn,
    },
    languageOptions: {
      parserOptions: {
        project: ["./projects/elohim-service/tsconfig.json"],
        tsconfigRootDir: __dirname
      }
    },
    settings: {
      "import/resolver": {
        typescript: {
          project: "./projects/elohim-service/tsconfig.json"
        }
      }
    },
    rules: {
      ...sharedTsRules,
      // DOM-specific unicorn rules off — this is a Node.js library
      "unicorn/prefer-blob-reading-methods": "off",
      "unicorn/prefer-dom-node-remove": "off",

      // ── Downgrades for existing codebase ─────────────────────
      // These are enforced as errors in elohim-app but need gradual migration here.
      // TODO: Promote to error as violations are fixed.
      "@typescript-eslint/prefer-nullish-coalescing": "warn",   // 137 instances, || → ?? changes semantics
      "@typescript-eslint/no-explicit-any": "warn",             // 67 instances, WASM interop + dynamic content
      "no-console": ["warn", { allow: ["warn", "error"] }],    // 41 instances, no injected logger yet
      "@typescript-eslint/no-misused-promises": "warn",         // 19 instances, need careful review
      "@typescript-eslint/require-await": "warn",               // 13 instances, need review
      "@typescript-eslint/no-require-imports": "off",           // CJS modules use require()
      "sonarjs/cognitive-complexity": ["warn", 15],             // 18 instances, refactoring task
      "sonarjs/no-duplicate-string": ["warn", { threshold: 3 }], // 24 instances
      "sonarjs/slow-regex": "warn",                             // 13 instances, advisory
      "sonarjs/different-types-comparison": "warn",             // 6 instances
      "sonarjs/no-nested-template-literals": "warn",            // 4 instances
      "sonarjs/no-identical-functions": "warn",                 // 4 instances
      "sonarjs/no-alphabetical-sort": "warn",                   // 4 instances
      "sonarjs/os-command": "warn",                             // 2 instances
      "sonarjs/todo-tag": "off",                                // TODOs tracked separately
      "sonarjs/unused-import": "off",                           // Handled by @typescript-eslint/no-unused-vars
      "sonarjs/no-unused-vars": "off",                          // Handled by @typescript-eslint/no-unused-vars
    }
  },
  // CLI files: allow console.log
  {
    files: ["projects/elohim-service/src/cli/**/*.ts"],
    rules: {
      "no-console": "off"
    }
  },

  // ================================================================
  // 3. LAMAD-UI + HTML5-APP-PLUGIN (Angular library — bundler moduleResolution)
  // ================================================================
  {
    files: [
      "projects/lamad-ui/src/**/*.ts",
      "projects/html5-app-plugin/src/**/*.ts"
    ],
    extends: [
      eslint.configs.recommended,
      ...tseslint.configs.recommended,
      ...tseslint.configs.stylistic,
      sonarjs.configs.recommended,
    ],
    plugins: {
      "@angular-eslint": angular,
      "import": importPlugin,
      "prettier": prettierPlugin,
      "unicorn": unicorn,
    },
    languageOptions: {
      parserOptions: {
        project: ["./tsconfig.json"],
        tsconfigRootDir: __dirname
      }
    },
    settings: {
      "import/resolver": {
        typescript: {
          project: "./tsconfig.json"
        }
      }
    },
    rules: {
      ...sharedTsRules,
      // DOM-specific unicorn rules — these are browser libraries
      "unicorn/prefer-blob-reading-methods": "error",
      "unicorn/prefer-dom-node-remove": "error",
      // Angular rules
      "@angular-eslint/directive-selector": [
        "error",
        { type: "attribute", prefix: "lamad", style: "camelCase" }
      ],
      "@angular-eslint/component-selector": [
        "error",
        { type: "element", prefix: "lamad", style: "kebab-case" }
      ],
      "@angular-eslint/no-empty-lifecycle-method": "error",
      "@angular-eslint/use-lifecycle-interface": "error",
    }
  },

  // ================================================================
  // 4. HTML TEMPLATES (lamad-ui)
  // ================================================================
  {
    files: ["projects/lamad-ui/src/**/*.html"],
    plugins: {
      "@angular-eslint/template": angularTemplate,
      "prettier": prettierPlugin
    },
    languageOptions: {
      parser: angularTemplateParser
    },
    rules: {
      // Core template rules
      "@angular-eslint/template/banana-in-box": "error",
      "@angular-eslint/template/no-negated-async": "error",
      "@angular-eslint/template/eqeqeq": "error",
      "@angular-eslint/template/no-any": "error",

      // Accessibility
      "@angular-eslint/template/no-positive-tabindex": "error",
      "@angular-eslint/template/click-events-have-key-events": "error",
      "@angular-eslint/template/mouse-events-have-key-events": "error",
      "@angular-eslint/template/interactive-supports-focus": "error",
      "@angular-eslint/template/role-has-required-aria": "error",
      "@angular-eslint/template/valid-aria": "error",
      "@angular-eslint/template/alt-text": "error",
      "@angular-eslint/template/label-has-associated-control": "warn",
      "@angular-eslint/template/button-has-type": "error",
      "@angular-eslint/template/table-scope": "error",

      // Quality
      "@angular-eslint/template/no-duplicate-attributes": "error",
      "@angular-eslint/template/no-distracting-elements": "error",
      "@angular-eslint/template/no-autofocus": "warn",
      "@angular-eslint/template/elements-content": "error",

      // Prettier for HTML templates
      "prettier/prettier": [process.env.CI === "true" ? "off" : "error"]
    }
  }
);
