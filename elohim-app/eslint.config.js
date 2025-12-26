// @ts-check
const eslint = require("@eslint/js");
const tseslint = require("typescript-eslint");
const angular = require("@angular-eslint/eslint-plugin");
const angularTemplate = require("@angular-eslint/eslint-plugin-template");
const angularTemplateParser = require("@angular-eslint/template-parser");
const importPlugin = require("eslint-plugin-import");

module.exports = tseslint.config(
  {
    // Global ignores
    ignores: [
      "dist/**",
      "node_modules/**",
      "**/*.spec.ts",
      "coverage/**",
      ".angular/**"
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
      "import": importPlugin
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

      // General best practices
      "no-console": ["warn", { allow: ["warn", "error"] }],
      "prefer-const": "error",
      "no-var": "error"
    }
  },
  {
    // HTML templates
    files: ["**/*.html"],
    plugins: {
      "@angular-eslint/template": angularTemplate
    },
    languageOptions: {
      parser: angularTemplateParser
    },
    rules: {
      "@angular-eslint/template/banana-in-box": "error",
      "@angular-eslint/template/no-negated-async": "error",
      "@angular-eslint/template/eqeqeq": "error"
    }
  }
);
