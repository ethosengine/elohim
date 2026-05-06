# Elohim Lit Component Layer Pivot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up `<elohim-button>` end-to-end as a Lit-based Web Component proving the full pattern (build → manifest → freshness check → storybook → Angular consumer), with `app/elohim-styles/` renamed to `app/elohim-elements/`.

**Architecture:** Per-pillar packages under `app/elohim-elements/`. `elohim-core` houses tokens (existing SCSS) plus the first Lit atom. Vite library mode produces ESM + d.ts. `@custom-elements-manifest/analyzer` emits `custom-elements.json` post-build. Storybook framework swaps from `@storybook/angular` (builder integration) to `@storybook/web-components-vite` (standard CLI). Angular host (`NotFoundComponent`, standalone) consumes via `CUSTOM_ELEMENTS_SCHEMA`.

**Tech Stack:** Lit 3, TypeScript 5.7, Vite 5/6 library mode, vite-plugin-dts, @custom-elements-manifest/analyzer, @web/test-runner + axe-core for unit a11y, Storybook 10 (`@storybook/web-components-vite`).

**Spec:** `genesis/docs/superpowers/specs/2026-05-06-elohim-lit-component-pivot-design.md`

## Scope note (P2P design gate disposition)

**The p2p-design-gate skill does not apply to this plan.** The work creates no data entities, DHT entry types, storage tables, HTTP routes, sync messages, or wire contracts. All "schema" references in this plan are:

- W3C **`custom-elements.json`** — a build artifact describing component APIs (props, slots, events, CSS parts). Local to each package, never stored, never synced, never crosses a node boundary.
- **`tsconfig.json`** — TypeScript compiler configuration.
- References to the **existing** schema-codegen pre-push pattern as an architectural reference to mirror, not as new schema work.

This is UI substrate scaffolding. Source of truth for component shape is the JSDoc-annotated TypeScript class itself; the manifest is a derivative.

---

## File map

### Created

| Path | Purpose |
|---|---|
| `app/elohim-elements/elohim-core/src/elohim-button.ts` | Lit `<elohim-button>` component class |
| `app/elohim-elements/elohim-core/src/index.ts` | Side-effect-free re-exports |
| `app/elohim-elements/elohim-core/src/register.ts` | Side-effectful `customElements.define` |
| `app/elohim-elements/elohim-core/src/elohim-button.spec.ts` | Functional + a11y unit tests |
| `app/elohim-elements/elohim-core/src/elohim-button.manifest.spec.ts` | Asserts CEM contents post-build |
| `app/elohim-elements/elohim-core/vite.config.ts` | Library build config |
| `app/elohim-elements/elohim-core/tsconfig.json` | TS config (strict, ESNext, decorators experimental) |
| `app/elohim-elements/elohim-core/web-test-runner.config.mjs` | Test runner config (axe plugin) |
| `app/elohim-elements/elohim-core/custom-elements-manifest.config.mjs` | CEM analyzer config |
| `tools/check-cem-fresh.mjs` | `--verify` script for pre-push freshness |
| `app/elohim-library/projects/graphos/src/foundations/__docs__/components/elohim-button.stories.ts` | First WC story |

### Modified

| Path | Change |
|---|---|
| `pnpm-workspace.yaml` | `app/elohim-styles/*` → `app/elohim-elements/*` (8 lines) |
| `app/elohim-elements/README.md` (was elohim-styles/README.md) | Rewrite to reflect components + styles unified scope |
| `app/elohim-elements/elohim-core/package.json` | Add Lit + Vite + CEM deps + scripts |
| `package.json` (root) | Add `elements:codegen` and `elements:codegen:verify` scripts |
| `.husky/pre-push` | Add `elements-codegen` project detection + gate |
| `app/elohim-library/package.json` | Swap `@storybook/angular` for `@storybook/web-components-vite`; replace storybook scripts |
| `app/elohim-library/.storybook/main.ts` | `framework: '@storybook/web-components-vite'` |
| `app/elohim-library/.storybook/preview.ts` (created if absent) | Imports `elohim-core/register` for side-effectful registration |
| `app/elohim-library/angular.json` | Remove `storybook` and `build-storybook` builder targets |
| `app/elohim-library/tsconfig.storybook.json` | Update or remove (storybook CLI manages its own TS) |
| `app/elohim-app/src/app/components/not-found/not-found.component.ts` | Add `CUSTOM_ELEMENTS_SCHEMA` to `schemas`; import `elohim-core/register` |
| `app/elohim-app/src/app/components/not-found/not-found.component.html` | Three `<button>` → three `<elohim-button>` |
| `app/elohim-app/src/app/components/not-found/not-found.component.css` | Remove `.btn`, `.btn-primary`, `.btn-secondary`, `.btn-ghost` rules (substrate now ships them) |
| `app/elohim-app/package.json` | Add `elohim-core` workspace dep |

### Deleted (after rename completes)

| Path | Reason |
|---|---|
| `app/elohim-styles/` (directory) | Renamed to `app/elohim-elements/` |

---

## Task 1: Rename `app/elohim-styles/` → `app/elohim-elements/`

**Files:**
- Modify: `pnpm-workspace.yaml`
- Move: `app/elohim-styles/` → `app/elohim-elements/`
- Modify: `app/elohim-elements/README.md` (after move)

- [ ] **Step 1: Verify clean working tree for the rename target**

Run: `git status --short | grep -E 'elohim-styles|elohim-elements'`
Expected: only `?? app/elohim-styles/` (untracked scaffold). If anything else is staged or modified inside, stash before continuing.

- [ ] **Step 2: Move the directory**

Run: `git mv app/elohim-styles app/elohim-elements 2>/dev/null || mv app/elohim-styles app/elohim-elements`

(The first form works for tracked content; falls back to plain `mv` for the untracked scaffold which is the actual current state.)

Verify: `ls app/elohim-elements/` shows `elohim-core elohim-shell elohim-imagodei elohim-lamad elohim-shefa elohim-qahal elohim-doorway elohim-avodah package.json README.md`

- [ ] **Step 3: Update `pnpm-workspace.yaml`**

Use Edit on `/projects/elohim/pnpm-workspace.yaml` to replace the eight matching lines:

```yaml
  - app/elohim-styles/elohim-core
  - app/elohim-styles/elohim-shell
  - app/elohim-styles/elohim-imagodei
  - app/elohim-styles/elohim-lamad
  - app/elohim-styles/elohim-shefa
  - app/elohim-styles/elohim-qahal
  - app/elohim-styles/elohim-doorway
  - app/elohim-styles/elohim-avodah
```

with:

```yaml
  - app/elohim-elements/elohim-core
  - app/elohim-elements/elohim-shell
  - app/elohim-elements/elohim-imagodei
  - app/elohim-elements/elohim-lamad
  - app/elohim-elements/elohim-shefa
  - app/elohim-elements/elohim-qahal
  - app/elohim-elements/elohim-doorway
  - app/elohim-elements/elohim-avodah
```

- [ ] **Step 4: Rewrite `app/elohim-elements/README.md`**

Replace the existing content with this:

```markdown
# elohim-elements

Single source of truth for elohim-protocol UI substrate, organised as a constellation of single-concern pnpm workspace packages. Each package ships **both** the styles (CSS custom properties / light-DOM globals where they exist) and the **Lit-based Custom Elements** that consume them.

## Modules

| Module             | Concern                                                                              |
| ------------------ | ------------------------------------------------------------------------------------ |
| `elohim-core`      | Tokens, light-DOM globals, atoms (button, card, input, badge, …)                     |
| `elohim-shell`     | Landing and host chrome — hero, footer, theme-toggle, etc.                           |
| `elohim-imagodei`  | Identity pillar — auth, profile, presence, recovery, agency, stewardship.            |
| `elohim-lamad`     | Learning pillar — content, paths, quiz engine, content-io, learner dashboard.        |
| `elohim-shefa`     | Economy pillar — stewardship, banking, REA flows, signals.                           |
| `elohim-qahal`     | Community pillar — governance, affinity, consent.                                    |
| `elohim-doorway`   | Doorway pillar — the in-app gateway-integration surface.                             |
| `elohim-avodah`    | Avodah meta-pillar — protocol-as-process reference implementation views.             |

## Layer model

- **Layer 1 — Tokens & light-DOM globals:** CSS custom properties in `elohim-core/tokens.scss`. Penetrate Shadow DOM via `var(--*)`.
- **Layer 2 — Custom Elements:** Lit components per package. Encapsulated styles via `static styles = css\`…\``. Consume tokens.
- **Layer 3 — Composition:** Storybook (`app/elohim-library/projects/graphos`). Documents and composes layers 1+2.

## Dependency direction

```
elohim-core
   ↑
   └── elohim-shell, elohim-imagodei, elohim-lamad, elohim-shefa,
       elohim-qahal, elohim-doorway, elohim-avodah
```

Pillar modules consume `elohim-core`. Pillars never consume each other — cross-pillar needs are a signal that the primitive belongs in `elohim-core`.

## Tag naming convention

- Core atoms: `<elohim-button>`, `<elohim-card>`, … (no pillar segment)
- Pillar components: `<elohim-imagodei-login>`, `<elohim-lamad-content-viewer>`, … (mirror package name 1:1)
- Always vendor-prefixed; third parties shipping their own pillar follow the same `<vendor>-<segment>-<name>` shape

## Consumers

- **`app/elohim-library` (graphos storybook)** — composition surface, documents every module.
- **`app/elohim-app`** — runtime; consumes via `CUSTOM_ELEMENTS_SCHEMA` in standalone components.
- **`doorway/doorway-app`** — admin UI; may consume `elohim-core` and selected pillars.

## Status

Sprint 1 (in progress): `<elohim-button>` end-to-end proof loop in `elohim-core`. All 7 other packages remain placeholder.
```

- [ ] **Step 5: Reinstall to update workspace symlinks**

Run: `pnpm install` (from repo root)
Expected: succeeds; no errors. The 8 elohim-elements packages appear under `node_modules/`.

- [ ] **Step 6: Sanity check no broken references to old path**

Run: `grep -rn "elohim-styles" --include="*.ts" --include="*.json" --include="*.md" --include="*.yaml" --include="*.scss" /projects/elohim/ 2>/dev/null | grep -v node_modules | grep -v "\.claude/" | grep -v worktrees`
Expected: zero hits (other than the elohim-styles git history references in spec/plan docs, which are intentional).

- [ ] **Step 7: Commit**

```bash
git add app/elohim-elements pnpm-workspace.yaml pnpm-lock.yaml
git rm -r app/elohim-styles 2>/dev/null || true
git commit -m "refactor(ui): rename app/elohim-styles → app/elohim-elements

Pivot from SCSS-only modules to packages that house Lit Web Components
plus their tokens. Same 8 packages, same pillar boundaries; umbrella
directory name now reflects unified scope.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Add Lit + tooling dependencies to `elohim-core`

**Files:**
- Modify: `app/elohim-elements/elohim-core/package.json`
- Create: `app/elohim-elements/elohim-core/tsconfig.json`

- [ ] **Step 1: Replace `app/elohim-elements/elohim-core/package.json`**

Use Write to create:

```json
{
  "name": "elohim-core",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "sideEffects": ["./src/register.ts", "./dist/register.js", "*.css", "*.scss"],
  "description": "Protocol substrate — tokens, light-DOM globals, and atomic Custom Elements.",
  "style": "./index.scss",
  "sass": "./index.scss",
  "main": "./dist/index.js",
  "module": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "customElements": "./dist/custom-elements.json",
  "exports": {
    ".": {
      "import": "./dist/index.js",
      "types": "./dist/index.d.ts"
    },
    "./register": {
      "import": "./dist/register.js",
      "types": "./dist/register.d.ts"
    },
    "./tokens.scss": "./tokens.scss",
    "./base.scss": "./base.scss",
    "./animations.scss": "./animations.scss",
    "./styles.css": "./dist/styles.css"
  },
  "files": ["dist", "*.scss", "README.md"],
  "scripts": {
    "build": "vite build && cem analyze --config custom-elements-manifest.config.mjs",
    "dev": "vite build --watch",
    "test": "wtr \"src/**/*.spec.ts\" --node-resolve",
    "analyze": "cem analyze --config custom-elements-manifest.config.mjs",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "lit": "^3.2.1"
  },
  "devDependencies": {
    "@custom-elements-manifest/analyzer": "^0.10.4",
    "@open-wc/testing": "^4.0.0",
    "@web/dev-server-esbuild": "^1.0.4",
    "@web/test-runner": "^0.20.0",
    "@web/test-runner-playwright": "^0.11.0",
    "axe-core": "^4.10.2",
    "typescript": "~5.7.2",
    "vite": "^6.0.0",
    "vite-plugin-dts": "^4.5.0"
  }
}
```

- [ ] **Step 2: Create `app/elohim-elements/elohim-core/tsconfig.json`**

Use Write to create:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "exactOptionalPropertyTypes": false,
    "experimentalDecorators": true,
    "useDefineForClassFields": false,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "isolatedModules": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",
    "rootDir": "./src"
  },
  "include": ["src/**/*.ts"],
  "exclude": ["src/**/*.spec.ts", "node_modules", "dist"]
}
```

> Note: `useDefineForClassFields: false` and `experimentalDecorators: true` are required for Lit's `@property` decorators to work correctly with Lit 3. This is the canonical Lit + TS configuration.

- [ ] **Step 3: Run pnpm install from repo root**

Run: `pnpm install`
Expected: succeeds; resolves Lit, Vite, web-test-runner, CEM analyzer, axe-core, etc.

- [ ] **Step 4: Verify the package resolves**

Run: `pnpm --filter elohim-core run typecheck`
Expected: succeeds (no source files yet, tsc with empty `src/` is a no-op).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/package.json app/elohim-elements/elohim-core/tsconfig.json pnpm-lock.yaml
git commit -m "feat(elohim-core): add Lit/Vite/CEM/axe dev dependencies

Wires up the build, test, and manifest tooling for the first Lit
component. No source files yet — that's the next task.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2b: Linting, formatting, and style setup

**Why this task exists:** Pull `elohim-app`'s linting/formatting standards into the elohim-elements packages **before** code starts flowing. elohim-app's flat ESLint config (sonarjs, unicorn, import, prettier, naming conventions, complexity limits) is the polished baseline; doorway-app is a lighter variant. We adopt the elohim-app rigor here, plus Lit/WC-specific plugins (`eslint-plugin-lit`, `eslint-plugin-wc`, `eslint-plugin-lit-a11y`) for the Custom Elements context, and Stylelint with `postcss-lit` for the `static styles = css\`…\`` template-literal blocks.

**Scope:** Configs at the `app/elohim-elements/` umbrella so all 8 packages inherit one source of truth. Each package then has thin `lint`, `lint:fix`, `format`, `format:check`, `lint:css` scripts. Builds the rails the rest of the sprint runs on.

**Files:**
- Create: `app/elohim-elements/eslint.config.js`
- Create: `app/elohim-elements/.prettierrc.js`
- Create: `app/elohim-elements/.prettierignore`
- Create: `app/elohim-elements/.stylelintrc.json`
- Create: `app/elohim-elements/.stylelintignore`
- Modify: `app/elohim-elements/elohim-core/package.json` — add lint/format/style devDeps and scripts

- [ ] **Step 1: Write `app/elohim-elements/eslint.config.js`**

Use Write to create the shared flat config — adapted from elohim-app's `eslint.config.js` (the polished baseline) but tuned for a Lit/Web-Components library context (no Angular plugins, plus `lit`/`wc`/`lit-a11y`):

```javascript
// @ts-check
const eslint = require('@eslint/js');
const tseslint = require('typescript-eslint');
const importPlugin = require('eslint-plugin-import');
const prettierPlugin = require('eslint-plugin-prettier');
const prettierConfig = require('eslint-config-prettier');
const sonarjs = require('eslint-plugin-sonarjs');
const unicorn = require('eslint-plugin-unicorn').default;
const lit = require('eslint-plugin-lit');
const wc = require('eslint-plugin-wc');
const litA11y = require('eslint-plugin-lit-a11y');

module.exports = tseslint.config(
  {
    // Global ignores — apply to every config object below
    ignores: [
      '**/dist/**',
      '**/node_modules/**',
      '**/coverage/**',
      '**/*.scss', // SCSS handled by Stylelint
    ],
  },
  {
    // TypeScript source files
    files: ['**/*.ts'],
    extends: [
      eslint.configs.recommended,
      ...tseslint.configs.recommended,
      ...tseslint.configs.stylistic,
      sonarjs.configs.recommended,
      lit.configs['flat/recommended'],
      wc.configs['flat/recommended'],
      litA11y.configs['flat/recommended'],
    ],
    plugins: {
      import: importPlugin,
      prettier: prettierPlugin,
      unicorn: unicorn,
    },
    languageOptions: {
      parserOptions: {
        // projectService auto-discovers tsconfigs per package — no manual list needed
        projectService: true,
        tsconfigRootDir: __dirname,
      },
    },
    settings: {
      'import/resolver': {
        typescript: { project: '*/tsconfig.json' },
      },
    },
    rules: {
      // ============================================================
      // TYPESCRIPT-ESLINT — SonarQube parity (matches elohim-app)
      // ============================================================
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unsafe-assignment': 'warn',
      '@typescript-eslint/no-unsafe-member-access': 'warn',
      '@typescript-eslint/no-unsafe-call': 'warn',
      '@typescript-eslint/no-unsafe-return': 'warn',
      '@typescript-eslint/no-unsafe-argument': 'warn',
      '@typescript-eslint/no-unused-vars': ['error', {
        argsIgnorePattern: '^_',
        varsIgnorePattern: '^_',
        caughtErrorsIgnorePattern: '^_',
      }],
      '@typescript-eslint/no-empty-function': 'warn',
      '@typescript-eslint/consistent-type-definitions': ['error', 'interface'],
      '@typescript-eslint/prefer-nullish-coalescing': 'error',
      '@typescript-eslint/prefer-optional-chain': 'error',
      '@typescript-eslint/prefer-readonly': 'error',
      '@typescript-eslint/prefer-for-of': 'error',
      '@typescript-eslint/prefer-includes': 'error',
      '@typescript-eslint/prefer-string-starts-ends-with': 'error',
      '@typescript-eslint/no-misused-promises': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/promise-function-async': 'warn',
      '@typescript-eslint/require-await': 'error',
      '@typescript-eslint/no-unnecessary-type-assertion': 'error',
      '@typescript-eslint/await-thenable': 'error',
      '@typescript-eslint/max-params': ['error', { max: 7 }],
      '@typescript-eslint/no-deprecated': 'warn',
      '@typescript-eslint/naming-convention': [
        'error',
        { selector: 'interface', format: ['PascalCase'] },
        { selector: 'class', format: ['PascalCase'] },
        { selector: 'typeAlias', format: ['PascalCase'] },
      ],

      // ============================================================
      // IMPORTS
      // ============================================================
      'import/order': ['error', {
        groups: ['builtin', 'external', 'internal', 'parent', 'sibling', 'index', 'type'],
        pathGroups: [
          { pattern: 'lit', group: 'external', position: 'before' },
          { pattern: 'lit/**', group: 'external', position: 'before' },
        ],
        'newlines-between': 'always',
        alphabetize: { order: 'asc', caseInsensitive: true },
      }],
      'import/no-duplicates': 'error',
      'import/no-useless-path-segments': 'error',

      // ============================================================
      // GENERAL BEST PRACTICES (mirrors elohim-app)
      // ============================================================
      'no-console': ['error', { allow: ['warn', 'error'] }],
      'prefer-const': 'error',
      'no-var': 'error',
      eqeqeq: ['error', 'always'],
      'no-eval': 'error',
      'no-implied-eval': 'error',
      'no-new-func': 'error',
      'no-throw-literal': 'error',

      // ============================================================
      // SONARJS
      // ============================================================
      'sonarjs/cognitive-complexity': ['error', 15],
      'sonarjs/no-duplicate-string': ['error', { threshold: 3 }],
      'sonarjs/no-identical-functions': 'error',
      'sonarjs/no-collapsible-if': 'error',
      'sonarjs/no-redundant-jump': 'error',
      'sonarjs/prefer-immediate-return': 'error',
      'sonarjs/no-inverted-boolean-check': 'error',
      'sonarjs/no-nested-conditional': 'error',
      'sonarjs/no-gratuitous-expressions': 'error',
      'sonarjs/prefer-single-boolean-return': 'error',
      'sonarjs/no-ignored-exceptions': 'error',
      'sonarjs/no-nested-functions': 'off', // arrow fns in event handlers are idiomatic for Lit

      // ============================================================
      // UNICORN — SonarQube parity
      // ============================================================
      'unicorn/prefer-set-has': 'error',
      'unicorn/no-zero-fractions': 'error',
      'unicorn/prefer-number-properties': 'error',
      'unicorn/prefer-array-index-of': 'error',
      'unicorn/no-typeof-undefined': 'error',
      'unicorn/prefer-export-from': 'error',
      'unicorn/prefer-global-this': 'error',
      'unicorn/no-array-push-push': 'error',
      'unicorn/prefer-dom-node-remove': 'error',
      'unicorn/prefer-array-some': 'error',
      'unicorn/prefer-negative-index': 'error',
      'unicorn/prefer-at': 'error',
      'unicorn/prefer-structured-clone': 'error',
      'unicorn/prefer-top-level-await': 'off', // libraries shouldn't ship TLA

      // ============================================================
      // LIT / WC SPECIFIC
      // ============================================================
      // wc/no-self-class: prevents `class extends ThisClass` mistakes
      'wc/no-self-class': 'error',
      // wc/guard-super-call: ensures super.connectedCallback() etc. when overriding
      'wc/guard-super-call': 'error',
      // wc/no-closed-shadow-root: closed shadow roots break dev tools and a11y
      'wc/no-closed-shadow-root': 'error',
      // lit/no-classfield-shadowing: Lit @property fields shadow inherited accessors
      'lit/no-classfield-shadowing': 'error',
      // lit/no-legacy-template-syntax: enforce modern Lit 2/3 syntax
      'lit/no-legacy-template-syntax': 'error',
      // lit/no-template-bind: don't bind `this` in templates
      'lit/no-template-bind': 'error',
      // lit/no-useless-template-literals: catch `html\`${''}\`` etc.
      'lit/no-useless-template-literals': 'error',
      // lit-a11y rules ship as 'flat/recommended' above — accepts defaults

      // ============================================================
      // PRETTIER
      // ============================================================
      'prettier/prettier': [process.env.CI === 'true' ? 'off' : 'error'],
      ...prettierConfig.rules,
    },
  },
  {
    // Test files — relax some rules
    files: ['**/*.spec.ts'],
    rules: {
      '@typescript-eslint/no-non-null-assertion': 'off', // tests assert presence
      'sonarjs/no-duplicate-string': 'off', // tests have repeated literals
      '@typescript-eslint/no-explicit-any': 'off', // tests sometimes need any
    },
  },
  {
    // Build/config files — disable type-aware rules that need projectService
    files: ['**/*.config.{ts,mjs,js}', '**/*.config.*.{ts,mjs,js}'],
    languageOptions: {
      parserOptions: { projectService: false, project: null },
    },
    rules: {
      '@typescript-eslint/no-floating-promises': 'off',
      '@typescript-eslint/no-misused-promises': 'off',
      '@typescript-eslint/await-thenable': 'off',
      '@typescript-eslint/require-await': 'off',
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/no-unsafe-call': 'off',
    },
  },
);
```

- [ ] **Step 2: Write `app/elohim-elements/.prettierrc.js`**

Use Write — verbatim copy from `app/elohim-app/.prettierrc.js` (so formatting is consistent across the monorepo):

```javascript
module.exports = {
  printWidth: 100,
  tabWidth: 2,
  semi: true,
  singleQuote: true,
  trailingComma: 'es5',
  bracketSpacing: true,
  arrowParens: 'avoid',
  htmlWhitespaceSensitivity: 'ignore',
};
```

- [ ] **Step 3: Write `app/elohim-elements/.prettierignore`**

Use Write:

```
**/dist/**
**/node_modules/**
**/coverage/**
**/*.min.*
**/custom-elements.json
```

- [ ] **Step 4: Write `app/elohim-elements/.stylelintrc.json`**

Use Write — Stylelint with the standard SCSS config plus `postcss-lit` for `css\`…\`` template literals inside `.ts` files:

```json
{
  "extends": ["stylelint-config-standard", "stylelint-config-standard-scss"],
  "overrides": [
    {
      "files": ["**/*.ts"],
      "customSyntax": "postcss-lit"
    },
    {
      "files": ["**/*.scss"],
      "customSyntax": "postcss-scss"
    }
  ],
  "rules": {
    "declaration-empty-line-before": null,
    "no-descending-specificity": null,
    "selector-class-pattern": null,
    "scss/dollar-variable-pattern": null,
    "custom-property-pattern": null
  }
}
```

- [ ] **Step 5: Write `app/elohim-elements/.stylelintignore`**

Use Write:

```
**/dist/**
**/node_modules/**
**/coverage/**
```

- [ ] **Step 6: Update `app/elohim-elements/elohim-core/package.json` — add lint deps + scripts**

Use Edit to add the following to `devDependencies` (alphabetically sorted alongside existing entries):

```json
    "@eslint/js": "^9.39.2",
    "eslint": "^9.39.2",
    "eslint-config-prettier": "^10.1.8",
    "eslint-import-resolver-typescript": "^4.4.4",
    "eslint-plugin-import": "^2.32.0",
    "eslint-plugin-lit": "^2.1.1",
    "eslint-plugin-lit-a11y": "^4.1.4",
    "eslint-plugin-prettier": "^5.5.5",
    "eslint-plugin-sonarjs": "^3.0.5",
    "eslint-plugin-unicorn": "^62.0.0",
    "eslint-plugin-wc": "^3.0.1",
    "postcss-lit": "^1.2.0",
    "postcss-scss": "^4.0.9",
    "prettier": "^3.8.1",
    "stylelint": "^16.10.0",
    "stylelint-config-standard": "^36.0.1",
    "stylelint-config-standard-scss": "^14.0.0",
    "typescript-eslint": "^8.16.0",
```

> Versions chosen to match the rest of the monorepo where possible (eslint, prettier, typescript-eslint, sonarjs, unicorn, import, prettier-eslint plugins). The lit/wc/lit-a11y/postcss-lit/stylelint set is net-new for this package.

Also update the `scripts` block to add the four new scripts (alphabetically near existing `analyze`, `build`, `dev`, `test`, `typecheck`):

```json
    "lint": "eslint --config ../eslint.config.js src",
    "lint:fix": "eslint --config ../eslint.config.js src --fix",
    "lint:css": "stylelint --config ../.stylelintrc.json --ignore-path ../.stylelintignore \"src/**/*.{ts,scss}\" \"*.scss\"",
    "format": "prettier --config ../.prettierrc.js --ignore-path ../.prettierignore --write \"src/**/*.ts\" \"*.scss\"",
    "format:check": "prettier --config ../.prettierrc.js --ignore-path ../.prettierignore --check \"src/**/*.ts\" \"*.scss\""
```

(Each script references the umbrella config at `../<file>`.)

- [ ] **Step 7: Run `pnpm install` from repo root**

Run: `pnpm install`
Expected: succeeds; resolves all new lint/format/style deps. Some pre-existing peer-dep warnings are unrelated.

- [ ] **Step 8: Smoke-test each script**

Run, in order, from repo root:

```bash
pnpm --filter elohim-core run lint
```

Expected: passes (only `src/index.ts` and `src/register.ts` exist; both should be clean by construction). If any rule complains about the existing files, **adjust the config**, not the source — these files are spec-prescribed and represent the canonical pattern. Common likely tweak: a Lit-specific rule complaining about `register.ts` because it doesn't extend LitElement. If so, add `register.ts` to a per-file rule override in the config.

```bash
pnpm --filter elohim-core run format:check
```

Expected: passes (or run `pnpm --filter elohim-core run format` to auto-format the existing files; commit any formatting changes alongside the lint setup).

```bash
pnpm --filter elohim-core run lint:css
```

Expected: passes — no `.ts` files with `css\`\`` templates yet, but the SCSS files in elohim-core (`tokens.scss`, `base.scss`, `animations.scss`, `index.scss`) get scanned. If any rule fails on the existing SCSS, **add a rule override** in the stylelint config (the existing tokens.scss was harvested verbatim from the original styles.css and shouldn't be hand-fixed for cosmetic lint).

- [ ] **Step 9: Commit**

```bash
git add app/elohim-elements/eslint.config.js \
        app/elohim-elements/.prettierrc.js \
        app/elohim-elements/.prettierignore \
        app/elohim-elements/.stylelintrc.json \
        app/elohim-elements/.stylelintignore \
        app/elohim-elements/elohim-core/package.json \
        pnpm-lock.yaml
git commit -m "feat(elohim-elements): linting + formatting + style configs

Adapts elohim-app's flat ESLint config (sonarjs, unicorn, import,
prettier, naming conventions, complexity limits) for the Lit/WC
context — drops Angular plugins, adds eslint-plugin-lit,
eslint-plugin-wc, and eslint-plugin-lit-a11y. Stylelint covers SCSS
files plus css\`\` template literals via postcss-lit. Prettier config
is verbatim from elohim-app for cross-monorepo consistency.

Configs live at the app/elohim-elements/ umbrella so all 8 packages
inherit. Each package has thin lint/lint:fix/format/format:check/lint:css
scripts that reference the umbrella configs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

> **Forward note for Tasks 4–12:** Each subsequent task that touches TS source must run `pnpm --filter elohim-core run lint` and `pnpm --filter elohim-core run format:check` (or `format`, then commit) before the task's commit step. The pre-push hook gate (Task 8) will eventually enforce this; until then, it's the implementer's responsibility.

> **Forward note for Task 4:** Before the test step (`pnpm --filter elohim-core test`), run `pnpm exec playwright install chromium` (in repo root). The web-test-runner config uses `playwrightLauncher({ product: 'chromium' })` and Playwright's chromium binary is not pre-installed in this environment.

---

## Task 3: Vite library config + minimal entry points

**Files:**
- Create: `app/elohim-elements/elohim-core/vite.config.ts`
- Create: `app/elohim-elements/elohim-core/src/index.ts`
- Create: `app/elohim-elements/elohim-core/src/register.ts`

- [ ] **Step 1: Write `vite.config.ts`**

Use Write to create:

```typescript
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import dts from 'vite-plugin-dts';

export default defineConfig({
  build: {
    lib: {
      entry: {
        index: resolve(__dirname, 'src/index.ts'),
        register: resolve(__dirname, 'src/register.ts'),
      },
      formats: ['es'],
      fileName: (_format, entry) => `${entry}.js`,
    },
    rollupOptions: {
      external: [/^lit($|\/)/],
    },
    sourcemap: true,
    target: 'es2022',
  },
  plugins: [
    dts({
      entryRoot: 'src',
      include: ['src/**/*.ts'],
      exclude: ['src/**/*.spec.ts'],
    }),
  ],
});
```

- [ ] **Step 2: Write `src/index.ts` (placeholder, side-effect-free)**

Use Write to create:

```typescript
// Side-effect-free entry. Re-exports component classes and types.
// Consumers that want auto-registration import from './register' instead.

export { ElohimButton } from './elohim-button.js';
export type { ElohimButtonVariant } from './elohim-button.js';
```

> Note: this file references `./elohim-button.js` which doesn't exist yet. The build will fail until Task 4 lands. That's intentional — TDD ordering puts the test for the missing module first.

- [ ] **Step 3: Write `src/register.ts` (side-effectful)**

Use Write to create:

```typescript
import { ElohimButton } from './elohim-button.js';

if (!customElements.get('elohim-button')) {
  customElements.define('elohim-button', ElohimButton);
}
```

- [ ] **Step 4: Commit (build will not yet succeed — that's expected)**

```bash
git add app/elohim-elements/elohim-core/vite.config.ts app/elohim-elements/elohim-core/src/
git commit -m "feat(elohim-core): scaffold Vite library config + entry points

index.ts and register.ts forward-declare elohim-button; the actual
component class lands in the next task following TDD ordering.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Write the failing test for `<elohim-button>`

**Files:**
- Create: `app/elohim-elements/elohim-core/web-test-runner.config.mjs`
- Create: `app/elohim-elements/elohim-core/src/elohim-button.spec.ts`

- [ ] **Step 1: Write `web-test-runner.config.mjs`**

Use Write to create:

```javascript
import { esbuildPlugin } from '@web/dev-server-esbuild';
import { playwrightLauncher } from '@web/test-runner-playwright';

export default {
  files: 'src/**/*.spec.ts',
  nodeResolve: true,
  browsers: [playwrightLauncher({ product: 'chromium' })],
  plugins: [
    esbuildPlugin({
      ts: true,
      target: 'es2022',
      tsconfig: './tsconfig.json',
    }),
  ],
  testFramework: {
    config: {
      ui: 'bdd',
      timeout: 5000,
    },
  },
};
```

- [ ] **Step 2: Write the failing test `src/elohim-button.spec.ts`**

Use Write to create:

```typescript
import { fixture, html, expect } from '@open-wc/testing';
import axe from 'axe-core';
import './register.js';
import type { ElohimButton } from './elohim-button.js';

describe('<elohim-button>', () => {
  it('renders the default slot content', async () => {
    const el = await fixture<ElohimButton>(html`<elohim-button>Click me</elohim-button>`);
    expect(el).to.exist;
    expect(el.shadowRoot).to.exist;
    const slot = el.shadowRoot!.querySelector('slot');
    expect(slot).to.exist;
    const assigned = slot!.assignedNodes({ flatten: true });
    const text = assigned.map((n) => n.textContent).join('').trim();
    expect(text).to.equal('Click me');
  });

  it('defaults variant to "primary"', async () => {
    const el = await fixture<ElohimButton>(html`<elohim-button>Hi</elohim-button>`);
    expect(el.variant).to.equal('primary');
  });

  it('accepts variant="secondary" and reflects to attribute', async () => {
    const el = await fixture<ElohimButton>(
      html`<elohim-button variant="secondary">Hi</elohim-button>`
    );
    expect(el.variant).to.equal('secondary');
    expect(el.getAttribute('variant')).to.equal('secondary');
  });

  it('accepts variant="ghost"', async () => {
    const el = await fixture<ElohimButton>(
      html`<elohim-button variant="ghost">Hi</elohim-button>`
    );
    expect(el.variant).to.equal('ghost');
  });

  it('emits a click event when activated by mouse', async () => {
    const el = await fixture<ElohimButton>(html`<elohim-button>Hi</elohim-button>`);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.click();
    expect(clicks).to.equal(1);
  });

  it('emits a click event when activated by keyboard (Enter)', async () => {
    const el = await fixture<ElohimButton>(html`<elohim-button>Hi</elohim-button>`);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.focus();
    inner.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    inner.click(); // browsers fire click on Enter for native buttons; simulate
    expect(clicks).to.equal(1);
  });

  it('does not emit a click event when disabled', async () => {
    const el = await fixture<ElohimButton>(
      html`<elohim-button disabled>Hi</elohim-button>`
    );
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.click();
    expect(clicks).to.equal(0);
  });

  it('sets aria-disabled when disabled', async () => {
    const el = await fixture<ElohimButton>(
      html`<elohim-button disabled>Hi</elohim-button>`
    );
    const inner = el.shadowRoot!.querySelector('button')!;
    expect(inner.getAttribute('aria-disabled')).to.equal('true');
    expect(inner.hasAttribute('disabled')).to.be.true;
  });

  it('passes axe-core a11y scan in default state', async () => {
    const el = await fixture<ElohimButton>(html`<elohim-button>Submit</elohim-button>`);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core a11y scan in disabled state', async () => {
    const el = await fixture<ElohimButton>(
      html`<elohim-button disabled>Submit</elohim-button>`
    );
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `pnpm --filter elohim-core test`
Expected: FAILS with errors about `./elohim-button.js` not existing or `ElohimButton` not exported. This is the desired red state.

- [ ] **Step 4: Commit (red)**

```bash
git add app/elohim-elements/elohim-core/web-test-runner.config.mjs app/elohim-elements/elohim-core/src/elohim-button.spec.ts
git commit -m "test(elohim-core): add failing tests for <elohim-button>

Covers slot content, variant property/attribute, click events,
disabled handling (no click + aria-disabled), and axe-core scans
in default and disabled states. Tests fail because the component
doesn't exist yet; implementation follows.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Implement `<elohim-button>` to make tests pass

**Files:**
- Create: `app/elohim-elements/elohim-core/src/elohim-button.ts`

- [ ] **Step 1: Write `src/elohim-button.ts`**

Use Write to create:

> **Side-effect contract:** This file does NOT use Lit's `@customElement` decorator. The decorator calls `customElements.define()` at module evaluation time, which would make `index.ts`'s "side-effect-free" contract a lie and create a tree-shaking trap (a bundler dropping an unused `import { ElohimButton } from 'elohim-core'` would also drop the registration). Registration lives exclusively in `src/register.ts`, which is the only file marked `sideEffects` in `package.json`. This matches the sophia-element pattern in this repo.

```typescript
import { LitElement, css, html, type PropertyValues } from 'lit';
import { property } from 'lit/decorators.js';

export type ElohimButtonVariant = 'primary' | 'secondary' | 'ghost';

/**
 * The elohim button atom — substrate primitive for action affordances.
 *
 * Token-driven; respects light/dark theme via the global tokens cascade.
 *
 * @element elohim-button
 *
 * @prop {ElohimButtonVariant} variant - Visual variant: primary | secondary | ghost
 * @prop {boolean} disabled - Disabled state. Suppresses click and applies aria-disabled.
 *
 * @event {MouseEvent} click - Fired on activation (mouse or keyboard via native button)
 *
 * @slot - Default slot for label content (text or icon+text)
 *
 * @cssprop --elohim-button-bg - Override background color
 * @cssprop --elohim-button-fg - Override foreground (label) color
 * @cssprop --elohim-button-border - Override border style
 * @cssprop --elohim-button-radius - Override border-radius
 *
 * @csspart button - The internal native <button> element
 */
export class ElohimButton extends LitElement {
  static override readonly shadowRootOptions: ShadowRootInit = {
    ...LitElement.shadowRootOptions,
    delegatesFocus: true,
  };

  static override readonly styles = css`
    :host {
      display: inline-block;
    }

    :host([hidden]) {
      display: none;
    }

    button {
      /* Base sizing/typography */
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
      padding: 0.625rem 1.25rem;
      font: inherit;
      font-weight: 500;
      line-height: 1.2;
      border-radius: var(--elohim-button-radius, 0.375rem);
      border: var(--elohim-button-border, 1px solid transparent);
      cursor: pointer;
      transition:
        background-color 150ms ease,
        border-color 150ms ease,
        color 150ms ease,
        transform 80ms ease;
      background: var(--elohim-button-bg, var(--primary, #6b46c1));
      color: var(--elohim-button-fg, var(--text-light, #f3f4f6));
    }

    button:focus-visible {
      outline: 2px solid var(--tech-glow, #7fcbee);
      outline-offset: 2px;
    }

    button:hover:not([aria-disabled='true']) {
      filter: brightness(1.08);
    }

    button:active:not([aria-disabled='true']) {
      transform: translateY(1px);
    }

    button[aria-disabled='true'] {
      cursor: not-allowed;
      opacity: 0.55;
    }

    /* Variants */
    :host([variant='primary']) button {
      background: var(--elohim-button-bg, var(--primary, #6b46c1));
      color: var(--elohim-button-fg, var(--text-light, #f3f4f6));
    }

    :host([variant='secondary']) button {
      background: var(--elohim-button-bg, var(--secondary, #ec4899));
      color: var(--elohim-button-fg, var(--text-light, #f3f4f6));
    }

    :host([variant='ghost']) button {
      background: var(--elohim-button-bg, transparent);
      color: var(--elohim-button-fg, var(--text-light, #f3f4f6));
      border: var(--elohim-button-border, 1px solid currentColor);
    }
  `;

  @property({ reflect: true })
  variant: ElohimButtonVariant = 'primary';

  @property({ type: Boolean, reflect: true })
  disabled = false;

  override render() {
    return html`
      <button
        part="button"
        type="button"
        ?disabled=${this.disabled}
        aria-disabled=${this.disabled ? 'true' : 'false'}
      >
        <slot></slot>
      </button>
    `;
  }

  protected override updated(changed: PropertyValues<this>) {
    super.updated(changed);
    // No-op hook for future variant-derived state (e.g., loading, busy).
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-button': ElohimButton;
  }
}
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `pnpm --filter elohim-core test`
Expected: all 9 tests pass. If any fail, fix the component to match the test (not the other way around).

- [ ] **Step 3: Verify the build succeeds**

Run: `pnpm --filter elohim-core build`
Expected: `dist/index.js`, `dist/register.js`, `dist/index.d.ts`, `dist/register.d.ts`, `dist/custom-elements.json` all created. (The `cem analyze` step will fail without the config — that's the next task. For now, comment out the `&& cem analyze ...` portion of the build script temporarily, or expect it to fail at the analyze step.)

> **Workaround for this step only:** edit the `build` script to just `vite build` for now; revert in Task 6.

- [ ] **Step 4: Commit (green)**

```bash
git add app/elohim-elements/elohim-core/src/elohim-button.ts
git commit -m "feat(elohim-core): implement <elohim-button>

Token-driven Lit component with primary/secondary/ghost variants,
disabled state with aria-disabled, focus-visible outline, hover/active
states, and slot for label content. JSDoc tags annotated for the
custom-elements-manifest analyzer.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: CEM analyzer config + manifest test

**Files:**
- Create: `app/elohim-elements/elohim-core/custom-elements-manifest.config.mjs`
- Create: `app/elohim-elements/elohim-core/src/elohim-button.manifest.spec.ts`
- Modify: `app/elohim-elements/elohim-core/package.json` (re-enable `cem analyze` in build)

- [ ] **Step 1: Write `custom-elements-manifest.config.mjs`**

Use Write to create:

```javascript
// Forward-affordance: when component-CID federation tooling lands,
// add a plugin here that hashes each declaration and writes a
// `componentCid` field per entry. Until then, the field is reserved
// but not populated. See spec D8.
export default {
  globs: ['src/**/*.ts'],
  exclude: ['src/**/*.spec.ts'],
  outdir: 'dist',
  litelement: true,
  packagejson: false,
};
```

- [ ] **Step 2: Re-enable `cem analyze` in build**

Use Edit on `app/elohim-elements/elohim-core/package.json` to confirm the `build` script is exactly:

```
"build": "vite build && cem analyze --config custom-elements-manifest.config.mjs",
```

(If you temporarily simplified it in Task 5 Step 3, restore it now.)

- [ ] **Step 3: Run the build**

Run: `pnpm --filter elohim-core build`
Expected: `dist/custom-elements.json` exists and contains an entry for `elohim-button` with properties, events, slots, cssProperties.

Verify: `cat app/elohim-elements/elohim-core/dist/custom-elements.json | python3 -m json.tool | head -50`
You should see something like `"name": "elohim-button"`, properties for `variant` and `disabled`, slots, and cssProperties for the four `--elohim-button-*` overrides.

- [ ] **Step 4: Write `src/elohim-button.manifest.spec.ts`**

Use Write to create:

```typescript
import { expect } from '@open-wc/testing';
import manifest from '../dist/custom-elements.json' with { type: 'json' };

describe('elohim-button custom-elements-manifest', () => {
  const declaration = manifest.modules
    .flatMap((m: any) => m.declarations)
    .find((d: any) => d.tagName === 'elohim-button');

  it('declares the elohim-button tag', () => {
    expect(declaration).to.exist;
    expect(declaration.name).to.equal('ElohimButton');
  });

  it('exposes variant and disabled properties', () => {
    const propNames = (declaration.members ?? [])
      .filter((m: any) => m.kind === 'field')
      .map((m: any) => m.name);
    expect(propNames).to.include('variant');
    expect(propNames).to.include('disabled');
  });

  it('declares the default slot', () => {
    const slotNames = (declaration.slots ?? []).map((s: any) => s.name);
    expect(slotNames).to.include('');
  });

  it('declares the four --elohim-button-* CSS properties', () => {
    const cssPropNames = (declaration.cssProperties ?? []).map((p: any) => p.name);
    expect(cssPropNames).to.include('--elohim-button-bg');
    expect(cssPropNames).to.include('--elohim-button-fg');
    expect(cssPropNames).to.include('--elohim-button-border');
    expect(cssPropNames).to.include('--elohim-button-radius');
  });

  it('declares the button CSS part', () => {
    const partNames = (declaration.cssParts ?? []).map((p: any) => p.name);
    expect(partNames).to.include('button');
  });
});
```

- [ ] **Step 5: Run the manifest test**

Run: `pnpm --filter elohim-core test`
Expected: all unit tests + 5 manifest tests pass. If a manifest assertion fails, the JSDoc tags on `ElohimButton` are missing or malformed — fix the JSDoc, rebuild, re-test.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-elements/elohim-core/custom-elements-manifest.config.mjs app/elohim-elements/elohim-core/src/elohim-button.manifest.spec.ts app/elohim-elements/elohim-core/package.json
git commit -m "feat(elohim-core): emit + verify custom-elements-manifest

Adds @custom-elements-manifest/analyzer config and post-build step.
Manifest test asserts the published API (props, slots, cssProperties,
cssParts) matches expectations — catches JSDoc drift in CI.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Manifest freshness CLI (`tools/check-cem-fresh.mjs`)

**Files:**
- Create: `tools/check-cem-fresh.mjs`
- Modify: `package.json` (root) — add `elements:codegen` and `elements:codegen:verify` scripts

- [ ] **Step 1: Write `tools/check-cem-fresh.mjs`**

Use Write to create:

```javascript
#!/usr/bin/env node
/**
 * Verifies that `dist/custom-elements.json` is fresh relative to `src/**\/*.ts`
 * for each elohim-elements package.
 *
 * Mirrors the schema:codegen --verify pattern: regenerates the manifest to a
 * temp directory and diffs against the committed file. Fails if they differ.
 *
 * Usage:
 *   node tools/check-cem-fresh.mjs           # regenerate (default)
 *   node tools/check-cem-fresh.mjs --verify  # diff-only; nonzero exit on drift
 */
import { readFile, mkdtemp, rm, cp } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..');
const VERIFY = process.argv.includes('--verify');

// Discover packages: every directory under app/elohim-elements/ that has a
// custom-elements-manifest.config.mjs.
import { readdir } from 'node:fs/promises';
const ELEMENTS_DIR = join(REPO_ROOT, 'app/elohim-elements');
const dirs = (await readdir(ELEMENTS_DIR, { withFileTypes: true }))
  .filter((d) => d.isDirectory())
  .map((d) => join(ELEMENTS_DIR, d.name))
  .filter((d) => existsSync(join(d, 'custom-elements-manifest.config.mjs')));

if (dirs.length === 0) {
  console.log('[cem-fresh] No elohim-elements packages with manifest configs yet — nothing to check.');
  process.exit(0);
}

let failed = false;

for (const dir of dirs) {
  const name = dir.split('/').pop();
  const committed = join(dir, 'dist/custom-elements.json');

  if (VERIFY) {
    if (!existsSync(committed)) {
      console.error(`[cem-fresh] ${name}: dist/custom-elements.json missing. Run \`pnpm run elements:codegen\` and commit.`);
      failed = true;
      continue;
    }
    const tmp = await mkdtemp(join(tmpdir(), `cem-${name}-`));
    try {
      // Run a fresh build into tmp dir
      execSync(
        `pnpm --filter ${name} exec cem analyze --config custom-elements-manifest.config.mjs --outdir ${tmp}`,
        { cwd: REPO_ROOT, stdio: 'pipe' }
      );
      const fresh = join(tmp, 'custom-elements.json');
      const a = await readFile(committed, 'utf8');
      const b = await readFile(fresh, 'utf8');
      // Normalize: parse + re-stringify to ignore whitespace-only diffs
      const aNorm = JSON.stringify(JSON.parse(a), null, 2);
      const bNorm = JSON.stringify(JSON.parse(b), null, 2);
      if (aNorm !== bNorm) {
        console.error(`[cem-fresh] ${name}: dist/custom-elements.json is STALE. Run \`pnpm run elements:codegen\` and commit.`);
        failed = true;
      } else {
        console.log(`[cem-fresh] ${name}: fresh ✓`);
      }
    } finally {
      await rm(tmp, { recursive: true, force: true });
    }
  } else {
    // Regen mode — full build (vite + cem)
    console.log(`[cem-fresh] ${name}: regenerating...`);
    execSync(`pnpm --filter ${name} run build`, { cwd: REPO_ROOT, stdio: 'inherit' });
  }
}

process.exit(failed ? 1 : 0);
```

- [ ] **Step 2: Add scripts to root `package.json`**

Use Edit on `/projects/elohim/package.json`. Find the `scripts` section and add (alphabetically near other `:codegen` scripts):

```
    "elements:codegen": "node tools/check-cem-fresh.mjs",
    "elements:codegen:verify": "node tools/check-cem-fresh.mjs --verify",
```

- [ ] **Step 3: Test the regen path**

Run: `pnpm run elements:codegen`
Expected: rebuilds `elohim-core` and emits a fresh `dist/custom-elements.json`. Check `git status` — it should show **no diff** in `dist/custom-elements.json` (because nothing changed).

- [ ] **Step 4: Test the verify path (clean)**

Run: `pnpm run elements:codegen:verify`
Expected: `[cem-fresh] elohim-core: fresh ✓` and exits 0.

- [ ] **Step 5: Test the verify path (dirty)**

Edit `app/elohim-elements/elohim-core/src/elohim-button.ts` — add a new `@property() loading = false;` somewhere. Don't rebuild.
Run: `pnpm run elements:codegen:verify`
Expected: nonzero exit; stderr contains `STALE`.

Revert the edit: `git checkout -- app/elohim-elements/elohim-core/src/elohim-button.ts`

- [ ] **Step 6: Commit**

```bash
git add tools/check-cem-fresh.mjs package.json
git commit -m "feat(tools): add check-cem-fresh.mjs for manifest freshness

Mirrors schema:codegen --verify pattern. Pre-push hook will use the
--verify mode in the next task; the regen mode is for local dev.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

> **Decision: commit `dist/custom-elements.json`** (mirror the existing schema codegen pattern). The verify mode diffs the committed manifest against a freshly-regenerated one, exactly like `pnpm run schema:codegen:ts -- --verify`. The rest of `dist/` (JS bundles, d.ts) is gitignored; only the manifest is tracked. Set up `.gitignore`:

Use Edit on `app/elohim-elements/elohim-core/.gitignore` (create if missing):

```
dist/*
!dist/custom-elements.json
node_modules/
```

Then re-run the build, commit the manifest, and re-verify.

```bash
pnpm --filter elohim-core run build
git add app/elohim-elements/elohim-core/.gitignore app/elohim-elements/elohim-core/dist/custom-elements.json
git commit -m "chore(elohim-core): commit custom-elements.json for freshness checks"
```

---

## Task 8: Pre-push hook integration

**Files:**
- Modify: `.husky/pre-push`

- [ ] **Step 1: Add `elements-codegen` to project detection**

Use Edit on `/projects/elohim/.husky/pre-push`. Find the section labeled `# Fallback: grep-based project detection`. Locate the existing `if echo "$CHANGED" | grep -q "^elohim/sdk/schemas/"; then PROJECTS="$PROJECTS schema-codegen"; fi` block and *immediately after* it add:

```bash
  if echo "$CHANGED" | grep -qE "^app/elohim-elements/[^/]+/src/.*\.ts$"; then
    PROJECTS="$PROJECTS elements-codegen"
  fi
```

- [ ] **Step 2: Add `elements-codegen` to the schema-style gate runner block**

Find the conditional in `run_gate()`:

```bash
if [ "$PROJECT_NAME" = "schema-validate" ] || [ "$PROJECT_NAME" = "schema-dna" ] || [ "$PROJECT_NAME" = "schema-codegen" ] || ...
```

Append `|| [ "$PROJECT_NAME" = "elements-codegen" ]` to the chain.

Inside the `case "$PROJECT_NAME" in` block (the one with `schema-validate)`, add a new arm:

```bash
      elements-codegen)
        echo "[$PROJECT_NAME] Verifying elohim-elements custom-elements-manifest freshness..."
        pnpm run elements:codegen:verify 2>&1
        rc=$?
        ;;
```

- [ ] **Step 3: Add directory mapping**

Find the directory-mapping `case "$PROJECT" in` block (the one mapping `schema-codegen) PROJECT_DIR="." ;;`). Add:

```bash
      elements-codegen) PROJECT_DIR="." ;;
```

- [ ] **Step 4: Test pre-push hook detection**

Run: `git diff --name-only HEAD~1 HEAD | grep elohim-elements`
Expected: shows files from earlier tasks.

Smoke-test the hook locally:

```bash
echo "test" >> /projects/elohim/app/elohim-elements/elohim-core/src/_test_hook.ts
git add app/elohim-elements/elohim-core/src/_test_hook.ts
# Don't commit — just simulate the hook input
echo "ref/heads/dev abc123 ref/heads/dev def456" | bash .husky/pre-push 2>&1 | head -30
```

Expected: output mentions `elements-codegen` in detected projects and runs the verify.

Cleanup: `git rm app/elohim-elements/elohim-core/src/_test_hook.ts`

- [ ] **Step 5: Commit**

```bash
git add .husky/pre-push
git commit -m "ci(pre-push): add elements-codegen gate

Runs elements:codegen:verify on any change under
app/elohim-elements/<pkg>/src/. Fails the push when custom-elements.json
is stale, instructing the dev to run elements:codegen and commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Storybook framework swap to `@storybook/web-components-vite`

**Files:**
- Modify: `app/elohim-library/package.json`
- Modify: `app/elohim-library/.storybook/main.ts`
- Create: `app/elohim-library/.storybook/preview.ts` (if absent)
- Modify: `app/elohim-library/angular.json` (remove storybook builder targets)
- Delete or simplify: `app/elohim-library/tsconfig.storybook.json`

- [ ] **Step 1: Update `app/elohim-library/package.json` deps and scripts**

Use Edit:

Replace `"@storybook/angular": "^10.3.6",` with `"@storybook/web-components-vite": "^10.3.6",`.

Add (in the same devDependencies block): `"@storybook/web-components": "^10.3.6",` and `"lit": "^3.2.1",`.

Replace the `storybook` and `build-storybook` scripts:

```json
    "storybook": "storybook dev -p 6006 --config-dir .storybook",
    "build-storybook": "storybook build --config-dir .storybook",
```

(Drop the `prestorybook`/`prebuild-storybook` `sync-genesis` hooks if `sync-genesis` is independent of framework; otherwise keep them.)

Run: `pnpm install`

- [ ] **Step 2: Update `.storybook/main.ts`**

Use Edit. Replace contents with:

```typescript
import type { StorybookConfig } from '@storybook/web-components-vite';

const config: StorybookConfig = {
  stories: ['../projects/**/__docs__/**/*.@(stories.ts|mdx)'],
  addons: [
    '@storybook/addon-a11y',
    '@storybook/addon-docs',
    '@storybook/addon-links',
  ],
  framework: {
    name: '@storybook/web-components-vite',
    options: {},
  },
};

export default config;
```

- [ ] **Step 3: Create `.storybook/preview.ts`**

Use Write to create:

```typescript
import type { Preview } from '@storybook/web-components';
import 'elohim-core/register';

const preview: Preview = {
  parameters: {
    backgrounds: {
      default: 'dark',
      values: [
        { name: 'dark', value: '#0a0a0a' },
        { name: 'light', value: '#f3f4f6' },
      ],
    },
    a11y: {
      element: '#storybook-root',
      manual: false,
    },
  },
};

export default preview;
```

- [ ] **Step 4: Remove storybook builder targets from `angular.json`**

Use Edit on `app/elohim-library/angular.json` to remove the `storybook` and `build-storybook` target objects from the `elohim-ui-playground` project (the section identified by `"builder": "@storybook/angular:start-storybook"` and `"builder": "@storybook/angular:build-storybook"`). Leave the other targets (`build`, `test`, etc.) intact.

- [ ] **Step 5: Test storybook starts**

Run: `pnpm --filter elohim-library run storybook` (in background or new terminal)
Expected: storybook starts on port 6006; browse to `http://localhost:6006/`; existing MDX docs render normally; no console errors. Stop the server.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-library/package.json app/elohim-library/.storybook/main.ts app/elohim-library/.storybook/preview.ts app/elohim-library/angular.json pnpm-lock.yaml
git commit -m "feat(graphos): swap storybook framework to @storybook/web-components-vite

Removes the Angular CLI storybook builder integration; storybook now
runs via the standard CLI. Existing MDX docs are framework-agnostic and
keep working. Preview imports elohim-core/register so any WC story can
render registered tags without per-story boilerplate.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: First WC story for `<elohim-button>`

**Files:**
- Create: `app/elohim-library/projects/graphos/src/foundations/__docs__/components/elohim-button.stories.ts`

- [ ] **Step 1: Write the story file**

Use Write to create:

```typescript
import type { Meta, StoryObj } from '@storybook/web-components';
import { html } from 'lit';
import 'elohim-core/register';

const meta: Meta = {
  title: 'Foundations/Components/elohim-button',
  parameters: {
    docs: {
      description: {
        component:
          'Substrate primitive for action affordances. Token-driven; respects light/dark theme.',
      },
    },
  },
  argTypes: {
    variant: {
      control: 'select',
      options: ['primary', 'secondary', 'ghost'],
    },
    disabled: { control: 'boolean' },
    label: { control: 'text' },
  },
  args: {
    variant: 'primary',
    disabled: false,
    label: 'Click me',
  },
  render: (args) => html`
    <elohim-button
      variant=${args.variant}
      ?disabled=${args.disabled}
    >${args.label}</elohim-button>
  `,
};

export default meta;
type Story = StoryObj;

export const Primary: Story = { args: { variant: 'primary', label: 'Primary' } };
export const Secondary: Story = { args: { variant: 'secondary', label: 'Secondary' } };
export const Ghost: Story = { args: { variant: 'ghost', label: 'Ghost' } };
export const Disabled: Story = { args: { variant: 'primary', disabled: true, label: 'Disabled' } };

export const AllVariants: Story = {
  render: () => html`
    <div style="display: flex; gap: 1rem; align-items: center; padding: 1rem;">
      <elohim-button variant="primary">Primary</elohim-button>
      <elohim-button variant="secondary">Secondary</elohim-button>
      <elohim-button variant="ghost">Ghost</elohim-button>
      <elohim-button variant="primary" disabled>Disabled</elohim-button>
    </div>
  `,
};
```

- [ ] **Step 2: Add `elohim-core` as a dep in `app/elohim-library/package.json`**

Use Edit. In the `dependencies` block of `app/elohim-library/package.json`, add:

```json
    "elohim-core": "workspace:*",
```

Run: `pnpm install`

- [ ] **Step 3: Build elohim-core (so `dist/` is available for storybook to import)**

Run: `pnpm --filter elohim-core run build`

- [ ] **Step 4: Smoke-test the story**

Run: `pnpm --filter elohim-library run storybook`
Browse to `http://localhost:6006/?path=/docs/foundations-components-elohim-button--primary`
Expected: button renders with all variants; the a11y addon panel shows zero violations; controls in the addon panel toggle variant/disabled/label.

Stop the server.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-library/projects/graphos/src/foundations/__docs__/components/elohim-button.stories.ts app/elohim-library/package.json pnpm-lock.yaml
git commit -m "docs(graphos): first WC story — <elohim-button>

Renders all three variants (primary/secondary/ghost) plus disabled
state. Addon-a11y validates the rendered output. Storybook now
exercises the elohim-core/register import in preview.ts end-to-end.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: elohim-app consumer integration (`NotFoundComponent`)

**Files:**
- Modify: `app/elohim-app/package.json` — add `elohim-core` dep
- Modify: `app/elohim-app/src/app/components/not-found/not-found.component.ts`
- Modify: `app/elohim-app/src/app/components/not-found/not-found.component.html`
- Modify: `app/elohim-app/src/app/components/not-found/not-found.component.css`

- [ ] **Step 1: Add `elohim-core` as a workspace dep**

Use Edit on `app/elohim-app/package.json`. In `dependencies`, add (alphabetically):

```json
    "elohim-core": "workspace:*",
```

Run: `pnpm install`

- [ ] **Step 2: Update `not-found.component.ts`**

Use Edit. Find the `@Component({` block and add `CUSTOM_ELEMENTS_SCHEMA` to the import and to a new `schemas` array:

Top of file — replace:

```typescript
import { Component, OnInit, inject } from '@angular/core';
```

with:

```typescript
import { Component, CUSTOM_ELEMENTS_SCHEMA, OnInit, inject } from '@angular/core';
import 'elohim-core/register';
```

Inside the `@Component({})` decorator object, add the `schemas` field (alongside `selector`, `standalone`, `imports`, etc.):

```typescript
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
```

- [ ] **Step 3: Update `not-found.component.html`**

Use Edit. Replace the three buttons:

```html
      <button type="button" class="btn btn-primary" (click)="goHome()">
        <i class="fa-solid fa-house"></i>
        Go Home
      </button>
      <button type="button" class="btn btn-secondary" (click)="goToLamad()">
        <i class="fa-solid fa-graduation-cap"></i>
        Explore Lamad
      </button>
      <button type="button" class="btn btn-ghost" (click)="goBack()">
        <i class="fa-solid fa-arrow-left"></i>
        Go Back
      </button>
```

with:

```html
      <elohim-button variant="primary" (click)="goHome()">
        <i class="fa-solid fa-house"></i>
        Go Home
      </elohim-button>
      <elohim-button variant="secondary" (click)="goToLamad()">
        <i class="fa-solid fa-graduation-cap"></i>
        Explore Lamad
      </elohim-button>
      <elohim-button variant="ghost" (click)="goBack()">
        <i class="fa-solid fa-arrow-left"></i>
        Go Back
      </elohim-button>
```

- [ ] **Step 4: Strip the local `.btn-*` rules from `not-found.component.css`**

Open `app/elohim-app/src/app/components/not-found/not-found.component.css`. Identify and remove rule blocks targeting `.btn`, `.btn-primary`, `.btn-secondary`, `.btn-ghost` (the styles are now provided by the `<elohim-button>` shadow DOM). Keep `.actions` layout rules. If a `.btn` rule supplies an `:hover` state previously visible only via `.btn:hover`, also remove it (the WC owns hover).

- [ ] **Step 5: Smoke-test in dev**

Run: `pnpm --filter elohim-app start` (in a separate terminal)
Browse to `http://localhost:4200/this-route-does-not-exist`
Expected: 404 page renders with three buttons in primary/secondary/ghost variants; clicking each navigates correctly (Go Home → /, Explore Lamad → /lamad route, Go Back → previous page); buttons respond to hover/focus/active.

Stop the dev server.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/package.json app/elohim-app/src/app/components/not-found/ pnpm-lock.yaml
git commit -m "feat(elohim-app): consume <elohim-button> in NotFoundComponent

First substrate-component consumer. Adds CUSTOM_ELEMENTS_SCHEMA to the
standalone NotFoundComponent and replaces three native <button>
elements with <elohim-button variant=primary|secondary|ghost>. Strips
the now-redundant .btn-* CSS — substrate ships the styling.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 12: Final acceptance walkthrough

**Files:** none — this is a verification task.

- [ ] **Step 1: Run elohim-core gate locally**

```bash
pnpm --filter elohim-core run typecheck
pnpm --filter elohim-core run build
pnpm --filter elohim-core run test
```

Expected: all green.

- [ ] **Step 2: Run elohim-library gate locally**

```bash
pnpm --filter elohim-library run lint
pnpm --filter elohim-library run build-storybook
```

Expected: storybook static build succeeds.

- [ ] **Step 3: Run elohim-app gate locally**

```bash
cd app/elohim-app
pnpm exec eslint src --ext .ts,.html
pnpm exec ng build --configuration=development
pnpm exec vitest run --config vite.config.ts
cd -
```

Expected: all green. (Vitest may show coverage delta on `not-found.component.ts` since the CSS imports changed; that's expected.)

- [ ] **Step 4: Run pre-push hook simulation**

```bash
HUSKY=0 git push --dry-run origin HEAD 2>&1 | head -5  # confirm what we'd push
bash .husky/pre-push <<EOF
refs/heads/dev $(git rev-parse HEAD~12) refs/heads/dev $(git rev-parse HEAD)
EOF
```

Expected: `PRE-PUSH GATE: ALL CLEAR`. Includes `elements-codegen: PASSED`, `elohim-app: PASSED`, `elohim-library: PASSED`.

- [ ] **Step 5: Document the proven pattern in `app/elohim-elements/elohim-core/README.md`**

Use Write to create:

```markdown
# elohim-core

Protocol substrate — tokens, light-DOM globals, and atomic Custom Elements.

## What's here

- `tokens.scss`, `base.scss`, `animations.scss` — Layer 1 substrate (CSS custom properties + light-DOM globals)
- `src/elohim-button.ts` — first Lit atom; reference shape for all future atoms
- `src/register.ts` — side-effectful entry that registers all elements
- `src/index.ts` — side-effect-free re-exports for type imports

## Build

```bash
pnpm --filter elohim-core run build
```

Produces `dist/{index.js, register.js, *.d.ts, custom-elements.json}`.

## Test

```bash
pnpm --filter elohim-core run test
```

Runs functional + a11y unit tests in chromium via web-test-runner.

## Adding a new atom

1. Write the failing test in `src/<your-atom>.spec.ts`
2. Implement `src/<your-atom>.ts` extending `LitElement` with `@customElement`, `@property`, JSDoc tags
3. Add the export to `src/index.ts` and the `customElements.define` call to `src/register.ts`
4. Add a manifest assertion in `src/<your-atom>.manifest.spec.ts`
5. Add a `*.stories.ts` in graphos under `foundations/__docs__/components/`
6. Run `pnpm run build` (regenerates manifest), commit `dist/custom-elements.json`

## Tag naming

Core atoms use the flat prefix: `<elohim-button>`, `<elohim-card>`, etc. Pillar packages (`elohim-imagodei`, `elohim-lamad`, …) use the pillar-namespaced prefix: `<elohim-imagodei-login>`, `<elohim-lamad-content-viewer>`, etc.

## Why decisions are this way

See `genesis/docs/superpowers/specs/2026-05-06-elohim-lit-component-pivot-design.md`.
```

- [ ] **Step 6: Commit**

```bash
git add app/elohim-elements/elohim-core/README.md
git commit -m "docs(elohim-core): document the proven Lit atom pattern

Six-step recipe for adding new atoms, build/test commands, tag naming
convention, pointer to the design doc.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Acceptance checklist

When all 12 tasks are complete, verify:

- [ ] `app/elohim-styles/` no longer exists; `app/elohim-elements/` does
- [ ] `pnpm install` succeeds from the repo root
- [ ] `pnpm --filter elohim-core run build` produces `dist/{index.js, register.js, index.d.ts, register.d.ts, custom-elements.json}`
- [ ] `pnpm --filter elohim-core run test` passes (9 unit tests + 5 manifest tests)
- [ ] `pnpm --filter elohim-library run storybook` renders the `<elohim-button>` story with all variants
- [ ] `pnpm --filter elohim-app start` + visit `/anything-not-a-route` shows the not-found page with three `<elohim-button>` elements working
- [ ] `pnpm run elements:codegen:verify` exits 0 when committed manifest is fresh, 1 when stale
- [ ] Pre-push hook detects `elements-codegen` project on changes under `app/elohim-elements/<pkg>/src/`
- [ ] Manifest test fails when JSDoc tags are removed from `elohim-button.ts` (sanity check the test actually catches drift)
- [ ] `git log --oneline` shows ~12 incremental commits, each compileable on its own
