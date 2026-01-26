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
    ],
    plugins: {
      "@angular-eslint": angular,
      "import": importPlugin,
      "prettier": prettierPlugin,
      "sonarjs": sonarjs
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
      // Angular recommended rules
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

      // TypeScript rules - relaxed for gradual adoption
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": ["warn", {
        argsIgnorePattern: "^_",
        varsIgnorePattern: "^_"
      }],
      "@typescript-eslint/no-empty-function": "off",
      "@typescript-eslint/consistent-type-definitions": ["warn", "interface"],

      // Import rules - enforce aliases over deep relative imports crossing pillars
      "no-restricted-imports": ["error", {
        patterns: [
          {
            group: ["../../../elohim/*", "../../../imagodei/*", "../../../lamad/*", "../../../qahal/*", "../../../shefa/*", "../../../doorway/*"],
            message: "Use @app/{pillar} aliases instead of deep relative imports (e.g., @app/elohim/services/...)"
          }
          // Note: 4+ levels rule removed - catches too many false positives (environments, elohim-library)
        ]
      }],

      // Import organization - auto-fixable with --fix
      "import/order": ["warn", {
        groups: [
          "builtin",      // Node.js built-ins
          "external",     // npm packages
          "internal",     // @app/* aliases
          "parent",       // ../
          "sibling",      // ./
          "index",        // ./index
          "type"          // type imports
        ],
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
      "import/no-duplicates": "error",
      "import/no-useless-path-segments": "warn",

      // General best practices
      "no-console": ["warn", { allow: ["warn", "error"] }],
      "prefer-const": "error",
      "no-var": "error",

      // SonarJS rules - mirrors SonarQube analysis for local catching
      "sonarjs/cognitive-complexity": ["warn", 15],
      "sonarjs/no-duplicate-string": ["warn", { threshold: 3 }],
      "sonarjs/no-identical-functions": "warn",
      "sonarjs/no-collapsible-if": "warn",
      "sonarjs/no-redundant-jump": "warn",
      "sonarjs/no-nested-template-literals": "warn",
      "sonarjs/prefer-immediate-return": "warn",
      "sonarjs/prefer-single-boolean-return": "warn",

      // Prettier - auto-fix formatting (disabled in CI for performance)
      "prettier/prettier": [process.env.CI === "true" ? "off" : "error"],

      // Disable rules that conflict with Prettier
      ...prettierConfig.rules
    }
  },
  {
    // HTML templates
    files: ["**/*.html"],
    plugins: {
      "@angular-eslint/template": angularTemplate,
      "prettier": prettierPlugin
    },
    languageOptions: {
      parser: angularTemplateParser
    },
    rules: {
      "@angular-eslint/template/banana-in-box": "error",
      "@angular-eslint/template/no-negated-async": "error",
      "@angular-eslint/template/eqeqeq": "error",

      // Prettier for HTML templates (disabled in CI)
      "prettier/prettier": [process.env.CI === "true" ? "off" : "error"]
    }
  }
);
