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
const boundaries = require("eslint-plugin-boundaries");

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
      "unicorn": unicorn,
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
      },
      // Path aliases that resolve OUTSIDE this package are not npm dependencies.
      // Without this they are classified as external deps of the root package and
      // flagged extraneous (`import/no-extraneous-dependencies`):
      //   `@workspace/runtime` → app/workspace-runtime, the dev-workspace vendor
      //     fence: dev tooling composed into the app, not a dependency.
      //   `@app/lamad/*`       → app/lamad/src/app, a separate EPR-app bundle
      //     reached by tsconfig path alias. Declaring `lamad` an npm dependency
      //     would DEEPEN the bundle-seam violation the workspace-imports rail
      //     refuses (see app/CLAUDE.md §Bundle seams are not domain seams, and
      //     the open placement decision in arch-frontend-bundle-seams-backlog).
      //     The lint fix is to state what the alias is, not to invent a dep.
      // The in-workspace `@app/*` pillars are internal by construction, and
      // `import/order` above already declares `@app/**` group "internal" — this
      // makes the resolver agree with the ordering rule instead of contradicting it.
      "import/internal-regex": "^@(workspace|app)/"
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
      // DISABLED, deliberately: this rule is autofixable, and its fix changes
      // runtime semantics for zero correctness gain. Adding `async` to a
      // function that already returns a promise wraps the result in one extra
      // microtask tick and converts a synchronous throw into a rejection.
      // Running `lint:fix` applied it to exactly two sites and broke both:
      // StabilityLensComponent.fromStorage (the extra tick lands after
      // `fixture.whenStable()` resolves, so the spec reads 'loading' instead of
      // 'real') and main.server.ts's SSR bootstrap (the entry that
      // lint-ssr-entry.mjs exists to protect). A style warning must not be able
      // to retime the SSR entry point behind a reviewer's back.
      "@typescript-eslint/promise-function-async": "off",
      "@typescript-eslint/require-await": "error",              // S2486/S4123 - async functions need await

      // Type assertion hygiene
      "@typescript-eslint/no-unnecessary-type-assertion": "error", // S4325 - Redundant as/! assertions
      "@typescript-eslint/await-thenable": "error",             // S4123 - Await non-thenable
      "@typescript-eslint/max-params": ["error", { max: 7 }],  // S107 - Too many parameters

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
          { pattern: "@elohim/**", group: "internal", position: "after" },
          { pattern: "@workspace/**", group: "internal", position: "after" }
        ],
        pathGroupsExcludedImportTypes: ["type"],
        "newlines-between": "always",
        alphabetize: { order: "asc", caseInsensitive: true }
      }],
      "import/no-duplicates": "error",                          // S1128 - Duplicate imports
      "import/no-useless-path-segments": "error",
      "import/no-extraneous-dependencies": ["error", {          // Catch pnpm hoist phantom deps
        devDependencies: ["**/*.spec.ts", "**/*.test.ts", "**/vite.config.ts"],
        peerDependencies: true,
      }],

      // ============================================================
      // SSR-SAFETY RULES
      // Browser globals absent in the V8/deno_core Angular SSR render
      // runtime (root cause of the 2026-07-05 empty-landing incident).
      // ============================================================
      "no-restricted-syntax": [
        "error",
        { "selector": "MemberExpression[object.name='document']", "message": "SSR-unsafe: global `document` is undefined in the V8 SSR render runtime (2026-07-05 empty-render root cause). Inject Angular's DOCUMENT token (`inject(DOCUMENT)` → `this.doc`) or guard with `isPlatformBrowser(inject(PLATFORM_ID))`. If already guarded / provably browser-only, add `// eslint-disable-next-line no-restricted-syntax -- SSR-safe: <reason>`." },
        { "selector": "MemberExpression[object.name='window']", "message": "SSR-unsafe: global `window` is undefined in the V8 SSR runtime. Guard with isPlatformBrowser / inject DOCUMENT.defaultView. If guarded/browser-only, disable with a justification." },
        { "selector": "MemberExpression[object.name='localStorage']", "message": "SSR-unsafe: `localStorage` is undefined in the SSR runtime. Guard with isPlatformBrowser. A try/catch is not enough for the linter — add a justified disable if guarded." },
        { "selector": "MemberExpression[object.name='sessionStorage']", "message": "SSR-unsafe: `sessionStorage` is undefined in the SSR runtime. Guard with isPlatformBrowser or add a justified disable." },
        { "selector": "MemberExpression[object.name='navigator']", "message": "SSR-unsafe: `navigator` is undefined in the SSR runtime. Guard with isPlatformBrowser or add a justified disable." },
        { "selector": "MemberExpression[object.name='globalThis'][property.name=/^(document|window|matchMedia|addEventListener|removeEventListener|dispatchEvent|localStorage|sessionStorage|navigator|location|innerWidth|innerHeight|scrollY|scrollX|scrollTo|getComputedStyle|open|alert|confirm|prompt)$/]", "message": "SSR-unsafe: this `globalThis.*` browser API is undefined in the V8 SSR runtime. Guard with isPlatformBrowser or add a justified disable." },
        { "selector": "CallExpression[callee.name='matchMedia']", "message": "SSR-unsafe: bare `matchMedia` is undefined in the SSR runtime. Guard with `typeof globalThis.matchMedia === 'function'` or isPlatformBrowser." },
        { "selector": "NewExpression[callee.name=/^(IntersectionObserver|ResizeObserver|MutationObserver)$/]", "message": "SSR-unsafe: IntersectionObserver/ResizeObserver/MutationObserver are undefined in the SSR runtime. Guard with isPlatformBrowser or add a justified disable." }
      ],

      // ============================================================
      // GENERAL BEST PRACTICES
      // ============================================================
      "no-console": ["error", { allow: ["warn", "error"] }],    // S106 - No console.log
      "prefer-const": "error",                                   // S3353 - Prefer const
      "no-var": "error",                                         // S3504 - Use let/const
      // S1244 - Use === instead of ==, EXCEPT against null. `x == null` is the
      // deliberate "null or undefined" idiom and every one of this app's 13
      // violations was that idiom, several inside type predicates
      // (`filter((n): n is number => n != null)`). Rewriting those to `!==`
      // would let `undefined` through a guard asserting `is number` -- a real
      // bug introduced to satisfy a style rule. The null exemption is the
      // standard way to say this; `==` against anything else stays an error.
      "eqeqeq": ["error", "always", { null: "ignore" }],
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

      // ---- sonarjs/recommended rules downgraded, with the reason at the site ----

      // OFF: S3735 directly contradicts @typescript-eslint/no-floating-promises,
      // which is enabled as an error above. That rule's documented remedy for an
      // intentionally-unawaited promise is to mark it `void promise` -- exactly
      // what this rule forbids. A developer cannot satisfy both, which is why
      // this app carried 13 unfixed floating promises alongside 29 void-use
      // errors: whichever you chose, a rule failed you. no-floating-promises is
      // a correctness rule and wins; this is a style rule and goes. The other
      // 20 sites are `void assertExhaustive;` in a .typetest.ts whose entire
      // purpose is to be compiled and not run.
      "sonarjs/void-use": "off",

      // WARN: S3800 fires on TypeScript union return types -- `LogLevel | null`
      // from a try/catch read, `string | null` from a parse-or-null helper,
      // `boolean | UrlTree` because that IS Angular's guard signature. Those are
      // declared, compiler-checked contracts, not the untyped polymorphic
      // returns the rule was written for. Every in-repo suppression of it says
      // some version of "intentional"; 90 more would say the same. Kept visible
      // as a warning rather than deleted, so a genuinely messy return still
      // shows up.
      "sonarjs/function-return-type": "warn",

      // WARN: S1135 makes every TODO an error. The 68 here are accurate
      // architectural breadcrumbs, most of them in services that declare
      // themselves Phase 1 stubs at the top of the file -- "TODO: Persist to
      // Holochain DHT" is true and worth reading. An error-level TODO rule does
      // not get the work done; it teaches people to delete the marker. Tracked
      // work belongs in genesis/data/timeline; the inline note stays a warning.
      "sonarjs/todo-tag": "warn",

      // Keep this off - arrow functions in RxJS pipes are idiomatic
      "sonarjs/no-nested-functions": "off",

      // ============================================================
      // UNICORN RULES - SonarQube Parity (typescript:S77xx series)
      // ============================================================
      "unicorn/prefer-set-has": "error",                        // S7776 - Use Set.has() over Array.includes()
      "unicorn/no-zero-fractions": "error",                     // S7748 - Remove .0 from numbers
      "unicorn/prefer-number-properties": "error",              // S7773 - Number.isNaN over isNaN
      "unicorn/prefer-code-point": "error",                     // S7758 - codePointAt over charCodeAt
      "unicorn/prefer-array-index-of": "error",                 // S7753 - indexOf over findIndex
      "unicorn/no-typeof-undefined": "error",                   // S7741 - === undefined over typeof
      "unicorn/prefer-export-from": "error",                    // S7763 - Re-export directly
      // DISABLED, deliberately (S7764). Like promise-function-async above, this
      // rule is autofixable and its fix rewrites SSR-critical code. It cannot
      // tell a plain `window.foo` reference from the `typeof window ===
      // 'undefined'` guard idiom, and it rewrites the guard into
      // `globalThis.window === undefined` -- which TypeScript types as
      // statically always-false (sonarjs/different-types-comparison then fires
      // on the result, which is how this was caught). Running `lint:fix`
      // rewrote both of this app's SSR guards: app.config.ts's doorway-origin
      // resolver, and login.component.ts's five-clause browser check, where it
      // also folded away a defensive clause and left the surrounding comments
      // referring to a "preceding typeof check" that no longer existed.
      // This whole rule block exists because of the 2026-07-05 empty-landing
      // incident; a style autofix must not be able to rewrite its guards.
      "unicorn/prefer-global-this": "off",
      "unicorn/no-negated-condition": "error",                   // S7735 - Swap branches to remove negation
      "unicorn/no-array-push-push": "error",                     // S7778 - Single push call
      "unicorn/prefer-string-raw": "error",                      // S7780 - String.raw for backslashes
      "unicorn/prefer-blob-reading-methods": "error",            // S7756 - Blob.text() over FileReader
      "unicorn/prefer-dom-node-remove": "error",                 // S7762 - child.remove() over parent.removeChild
      "unicorn/prefer-array-some": "error",                      // S7765/S7754 - .some() over .find() for boolean
      "unicorn/prefer-negative-index": "error",                  // S7771 - .slice(-n)
      "unicorn/prefer-at": "error",                              // S7755 - .at(-1) over [arr.length-1]
      "unicorn/prefer-structured-clone": "error",                // S7784 - structuredClone over JSON roundtrip
      "unicorn/prefer-top-level-await": "off",                   // S7785 - Off: Angular modules don't support TLA

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
  },
  // ============================================================
  // PILLAR BOUNDARY RULES
  // Enforces the architectural import boundaries between domain pillars.
  //
  // Rules:
  //   elohim   → only @elohim/*, @app/generated, @app/testing (no sibling pillars)
  //   imagodei → elohim + above
  //   account  → imagodei + elohim + above  (account pillar: Task 17, not yet created)
  //   lamad    → elohim + above  (NOT imagodei, account, shefa, qahal)
  //   shefa    → elohim + above  (NOT imagodei, account, lamad, qahal)
  //   qahal    → elohim + above  (NOT imagodei, account, lamad, shefa)
  //
  // NOTE: 174 pre-existing violations exist across the codebase. This rule is set
  //       to "warn" to make the violations visible without blocking CI.
  //       Target: migrate to "error" once the backlog is cleared.
  //       Audit script: scripts/audit-pillar-imports.mjs
  //       Tracking: see violation list in commit message for task 16 of M5 plan.
  // ============================================================
  {
    files: ["src/app/**/*.ts"],
    plugins: {
      "boundaries": boundaries,
    },
    settings: {
      // Each element type matches ALL files recursively within a pillar directory.
      // eslint-import-resolver-typescript (already configured) resolves @app/* aliases
      // to absolute paths; the plugin then matches the resolved path against these patterns.
      "boundaries/elements": [
        { type: "elohim",   pattern: "src/app/elohim/**/*" },
        { type: "imagodei", pattern: "src/app/imagodei/**/*" },
        { type: "account",  pattern: "src/app/account/**/*" },
        { type: "lamad",    pattern: "src/app/lamad/**/*" },
        { type: "shefa",    pattern: "src/app/shefa/**/*" },
        { type: "qahal",    pattern: "src/app/qahal/**/*" },
      ],
    },
    rules: {
      // boundaries/element-types is deprecated in v6 in favour of boundaries/dependencies.
      // Using boundaries/dependencies (v6 name) with object-based selectors.
      // Rule set to "warn" — 174 pre-existing violations exist (see audit-pillar-imports.mjs).
      // Target: flip to "error" once the violation backlog in M5 tracking is cleared.
      "boundaries/dependencies": ["warn", {
        // default: deny all cross-pillar imports unless a rule below explicitly allows it
        default: "disallow",
        rules: [
          // Every pillar may always import from itself (intra-pillar imports are fine)
          {
            from: { type: ["elohim", "imagodei", "account", "lamad", "shefa", "qahal"] },
            allow: { to: { type: "{{from.type}}" } },
          },
          // imagodei: may import elohim
          { from: { type: "imagodei" }, allow: { to: { type: "elohim" } } },
          // account: may import imagodei + elohim
          { from: { type: "account" },  allow: { to: { type: ["imagodei", "elohim"] } } },
          // lamad, shefa, qahal: may import elohim only (NOT each other, NOT imagodei, NOT account)
          { from: { type: "lamad" },    allow: { to: { type: "elohim" } } },
          { from: { type: "shefa" },    allow: { to: { type: "elohim" } } },
          { from: { type: "qahal" },    allow: { to: { type: "elohim" } } },
          // elohim: no sibling pillars (covered by self-allow above + default disallow)
        ],
      }],
    },
  },

  // ============================================================
  // GENERATED CODE (src/app/generated)
  // ============================================================
  // Emitted from the protocol view schemas by `pnpm run schema:codegen:ts`,
  // carrying codegen's own `/* eslint-disable @typescript-eslint/consistent-
  // indexed-object-style */` banner. That banner is written for the schemas'
  // canonical consumer config; a consumer that does not enable the rule must
  // not then report the banner as dead code. Without this, 99 generated files
  // each raise an unused-directive warning, and `eslint --fix` STRIPS the
  // banner the next codegen run writes back — an oscillation, not a fix.
  // Scoped to directive reporting only: every rule stays on, because a
  // generator can still emit something genuinely wrong.
  {
    files: ["src/app/generated/**/*.ts", "src/app/**/generated/**/*.ts"],
    linterOptions: { reportUnusedDisableDirectives: "off" },
    rules: {
      // Style rules a generator's output trips. They cannot be satisfied
      // without hand-editing a file the next codegen run overwrites, so they
      // are not findings -- they are unfixable errors that red the gate for
      // everyone. Correctness rules stay on: a generator CAN emit something
      // genuinely wrong, and that is worth hearing about.
      "@typescript-eslint/array-type": "off",
      "sonarjs/no-duplicate-string": "off",
      "sonarjs/redundant-type-aliases": "off",
      "import/order": "off",
    },
  }
);
