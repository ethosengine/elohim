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

module.exports = tseslint.config(
  {
    // Global ignores
    ignores: [
      "dist/**",
      "node_modules/**",
      "**/*.spec.ts",
      "coverage/**",
      ".angular/**",
      "src/assets/**/*.js",  // Vendor bundles (perseus-plugin, etc.)
      "src/assets/**/*.umd.js"
    ]
  },
  {
    // TypeScript files
    files: ["**/*.ts"],
    extends: [
      eslint.configs.recommended,
      ...tseslint.configs.recommended,
      ...tseslint.configs.stylistic,
      sonarjs.configs.recommended,  // Full SonarQube parity (~266 rules)
    ],
    plugins: {
      "@angular-eslint": angular,
      "import": importPlugin,
      "prettier": prettierPlugin,
      // Note: sonarjs plugin is registered by sonarjs.configs.recommended in extends
    },
    languageOptions: {
      parserOptions: {
        project: ["./tsconfig.json", "./tsconfig.app.json"],
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
      // ============================================================
      // ANGULAR RULES
      // ============================================================
      "@angular-eslint/directive-selector": [
        "error",
        { type: "attribute", prefix: "app", style: "camelCase" }
      ],
      "@angular-eslint/component-selector": [
        "error",
        { type: "element", prefix: "app", style: "kebab-case" }
      ],
      "@angular-eslint/no-empty-lifecycle-method": "error",
      "@angular-eslint/use-lifecycle-interface": "error",

      // ============================================================
      // TYPESCRIPT-ESLINT RULES - SonarQube Parity
      // ============================================================

      // Type safety (stricter settings)
      "@typescript-eslint/no-explicit-any": "error",            // S6609 - "any" should not be used
      "@typescript-eslint/no-unsafe-assignment": "warn",        // Type safety
      "@typescript-eslint/no-unsafe-member-access": "warn",     // Type safety
      "@typescript-eslint/no-unsafe-call": "warn",              // Type safety
      "@typescript-eslint/no-unsafe-return": "warn",            // Type safety
      "@typescript-eslint/no-unsafe-argument": "warn",          // Type safety

      // Unused code detection
      "@typescript-eslint/no-unused-vars": ["error", {          // S1481 - Unused local variables
        argsIgnorePattern: "^_",
        varsIgnorePattern: "^_",
        caughtErrorsIgnorePattern: "^_"
      }],
      "@typescript-eslint/no-empty-function": "warn",           // S1186 - Empty functions

      // Code style matching SonarQube
      "@typescript-eslint/consistent-type-definitions": ["error", "interface"],
      "@typescript-eslint/prefer-nullish-coalescing": "error",  // S6606 - Use nullish coalescing
      "@typescript-eslint/prefer-optional-chain": "error",      // S6582 - Use optional chaining
      "@typescript-eslint/prefer-readonly": "error",            // S2933 - Mark fields readonly when possible
      "@typescript-eslint/no-array-constructor": "error",       // S7723 - Use new Array()
      "@typescript-eslint/prefer-for-of": "error",              // S4138 - Prefer for-of loops
      "@typescript-eslint/prefer-includes": "error",            // Prefer .includes() over .indexOf()
      "@typescript-eslint/prefer-string-starts-ends-with": "error", // S5850 - Use startsWith/endsWith

      // Promise handling - matches S6544
      "@typescript-eslint/no-misused-promises": "error",        // S6544 - Promise in void context
      "@typescript-eslint/no-floating-promises": "error",       // Unhandled promises must be awaited
      "@typescript-eslint/promise-function-async": "warn",      // Functions returning promises should be async
      "@typescript-eslint/require-await": "error",              // S2486/S4123 - async functions need await

      // Deprecated APIs - matches S1874
      "@typescript-eslint/no-deprecated": "warn",               // S1874 - Deprecated API usage

      // Naming conventions - matches S101
      "@typescript-eslint/naming-convention": [
        "error",
        {
          selector: "interface",
          format: ["PascalCase"],
          custom: {
            regex: "^[A-Z]",
            match: true
          }
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

      // ============================================================
      // IMPORT RULES
      // ============================================================
      "no-restricted-imports": ["error", {
        patterns: [
          {
            group: ["../../../elohim/*", "../../../imagodei/*", "../../../lamad/*", "../../../qahal/*", "../../../shefa/*", "../../../doorway/*"],
            message: "Use @app/{pillar} aliases instead of deep relative imports (e.g., @app/elohim/services/...)"
          }
        ]
      }],
      "import/order": ["error", {
        groups: ["builtin", "external", "internal", "parent", "sibling", "index", "type"],
        pathGroups: [
          { pattern: "@angular/**", group: "external", position: "before" },
          { pattern: "rxjs/**", group: "external", position: "before" },
          { pattern: "@app/**", group: "internal", position: "before" },
          { pattern: "@elohim/**", group: "internal", position: "after" }
        ],
        pathGroupsExcludedImportTypes: ["type"],
        "newlines-between": "always",
        alphabetize: { order: "asc", caseInsensitive: true }
      }],
      "import/no-duplicates": "error",                          // S1128 - Duplicate imports
      "import/no-useless-path-segments": "error",

      // ============================================================
      // GENERAL BEST PRACTICES
      // ============================================================
      "no-console": ["error", { allow: ["warn", "error"] }],    // S106 - No console.log
      "prefer-const": "error",                                   // S3353 - Prefer const
      "no-var": "error",                                         // S3504 - Use let/const
      "eqeqeq": ["error", "always"],                            // S1244 - Use === instead of ==
      "no-eval": "error",                                        // S1523 - No eval
      "no-implied-eval": "error",                               // No implied eval
      "no-new-func": "error",                                   // No Function constructor
      "no-throw-literal": "error",                              // S3696 - Throw Error objects

      // ============================================================
      // SONARJS RULES - Strict Configuration
      // ============================================================
      "sonarjs/cognitive-complexity": ["error", 15],            // S3776 - Cognitive complexity
      "sonarjs/no-duplicate-string": ["error", { threshold: 3 }],
      "sonarjs/no-identical-functions": "error",                // S4144 - Identical functions
      "sonarjs/no-collapsible-if": "error",                     // S1066 - Collapsible if
      "sonarjs/no-redundant-jump": "error",                     // S3626 - Redundant jumps
      "sonarjs/prefer-immediate-return": "error",               // S1488 - Return immediately
      "sonarjs/no-inverted-boolean-check": "error",             // S1940 - Inverted boolean
      "sonarjs/no-nested-conditional": "error",                 // S3358 - Nested ternary
      "sonarjs/no-gratuitous-expressions": "error",             // S2589 - Gratuitous expressions
      "sonarjs/prefer-single-boolean-return": "error",          // S1126 - Prefer single boolean return
      "sonarjs/no-ignored-exceptions": "error",                 // S2486 - Ignored exceptions
      "sonarjs/no-unused-vars": "error",                        // S1481 - Unused variables

      // Keep this off - arrow functions in RxJS pipes are idiomatic
      "sonarjs/no-nested-functions": "off",

      // ============================================================
      // PRETTIER
      // ============================================================
      "prettier/prettier": [process.env.CI === "true" ? "off" : "error"],
      ...prettierConfig.rules
    }
  },
  {
    // HTML templates - with accessibility rules matching SonarQube Web rules
    files: ["**/*.html"],
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
      "@angular-eslint/template/no-any": "error",               // No any in templates

      // ============================================================
      // ACCESSIBILITY RULES - Matching SonarQube Web:* rules
      // ============================================================
      // S6845 - tabIndex on non-interactive elements
      "@angular-eslint/template/no-positive-tabindex": "error",

      // MouseEventWithoutKeyboardEquivalentCheck - keyboard accessibility
      "@angular-eslint/template/click-events-have-key-events": "error",
      "@angular-eslint/template/mouse-events-have-key-events": "error",

      // S6819/S6842 - Interactive roles and focus support
      "@angular-eslint/template/interactive-supports-focus": "error",
      "@angular-eslint/template/role-has-required-aria": "error",
      "@angular-eslint/template/valid-aria": "error",

      // S6844 - Alt text for images
      "@angular-eslint/template/alt-text": "error",

      // S6827 - Label associations
      "@angular-eslint/template/label-has-associated-control": "warn",

      // S6823 - Button types
      "@angular-eslint/template/button-has-type": "error",

      // S6828 - Table scope
      "@angular-eslint/template/table-scope": "error",

      // Additional quality rules
      "@angular-eslint/template/no-duplicate-attributes": "error",
      "@angular-eslint/template/no-distracting-elements": "error",
      "@angular-eslint/template/no-autofocus": "warn",
      "@angular-eslint/template/elements-content": "error",      // Non-empty elements

      // Prettier for HTML templates (disabled in CI)
      "prettier/prettier": [process.env.CI === "true" ? "off" : "error"]
    }
  }
);
