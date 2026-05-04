# Graphos Narrative Scaffold (Sprint 1) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the graphos design surface inside Storybook 10 — a four-section IA (Narrative Flow / Foundations / Domains / Reference) populated by auto-imported genesis content, with empty domain component slots ready for sprint 2.

**Architecture:** New Angular library project `app/elohim-library/projects/graphos/` holds the IA wrappers. A pre-build script (`scripts/sync-genesis.mjs`) copies mapped genesis files into a gitignored `imported/` dir and generates per-feature MDX wrappers for glob sources. Static MDX wrappers (for single-file sources and domain landings) are committed; generated wrappers are gitignored. Storybook's existing glob `projects/**/__docs__/**/*.@(stories.ts|mdx)` picks them all up — no Storybook config change needed.

**Tech Stack:** Angular 19, Storybook 10.3 (Vite), MDX, ng-packagr, Node.js (ESM scripts), pnpm workspace, Jenkins via `app/elohim-library/Jenkinsfile`.

**Spec:** `genesis/docs/superpowers/specs/2026-05-04-design-surface-narrative-scaffold-design.md`

---

## File structure

**New files (committed):**

```
app/elohim-library/
  scripts/
    sync-genesis.mjs                                  ← path mapping + file copy + MDX gen
    sync-genesis.test.mjs                             ← unit tests
  projects/
    graphos/
      ng-package.json                                 ← ngPackagr config (not published)
      tsconfig.lib.json                               ← TS config
      src/
        public-api.ts                                 ← empty re-export (ng-packagr requires)
        narrative/
          why/__docs__/
            manifesto.mdx                             ← static wrapper
            constitution.mdx
            vision.mdx
          what/__docs__/
            brand.mdx
          how/__docs__/
            protocol-specification.mdx
            governance-layers.mdx
            epr-developer-guide.mdx
            hardware-spec.mdx
        foundations/__docs__/
          vocabulary-register.mdx                     ← static wrapper for real source
          epr-elements.mdx                            ← static placeholder
          rea-primitives.mdx                          ← static placeholder
          brand-atoms.mdx                             ← static placeholder
          component-atoms.mdx                         ← static placeholder
        domains/
          identity/__docs__/
            index.mdx                                 ← landing
            reference.mdx                             ← placeholder
            components.mdx                            ← placeholder
          learning/__docs__/
            index.mdx
            reference.mdx                             ← real source (lamad.md)
            components.mdx
          community/__docs__/
            index.mdx
            reference.mdx                             ← placeholder
            components.mdx
          economy/__docs__/
            index.mdx
            reference.mdx                             ← placeholder
            components.mdx
          doorway/__docs__/
            index.mdx
            reference.mdx                             ← placeholder
            components.mdx
        imported/
          .gitignore                                  ← `*\n!.gitignore`
```

**Generated files (gitignored, output of sync-genesis.mjs):**

```
projects/graphos/src/imported/                       ← copied/transformed genesis files
projects/graphos/src/**/__docs__/_generated/         ← per-feature MDX wrappers
                                                       (sub-dir per glob source)
```

**Modified files (existing repo):**

- `app/elohim-library/.storybook/theme.ts:7` — `brandTitle: 'Lamad UI'` → `brandTitle: 'graphos'`
- `app/elohim-library/package.json:5-13` — add `prestart` / `prebuild-storybook` hooks
- `app/elohim-library/angular.json` — register `graphos` project under `projects:`
- `app/elohim-library/build-manifest.json:8-17` — extend `inputs.sources` with genesis paths
- `genesis/orchestrator/orchestrator-strategy.mjs:81-97` — extend `elohim-storybook.changePatterns` with genesis paths
- `app/elohim-library/.gitignore` (or root `.gitignore`) — add `**/_generated/**` for graphos

---

## Task 1: Validate the MDX wrapper rendering pattern

Before scaffolding the whole project, confirm that the MDX `<Meta>` + content-import pattern works in this Storybook 10 setup. Use the existing `lamad-ui` project as a sandbox — one throwaway MDX file that we delete in Task 2 once the technique is proven.

**Files:**
- Create: `app/elohim-library/projects/lamad-ui/src/lib/components/__validation__/test-import.mdx`
- Create: `app/elohim-library/projects/lamad-ui/src/lib/components/__validation__/test-content.md` (a tiny markdown file)

- [ ] **Step 1: Create the test markdown source**

```bash
mkdir -p app/elohim-library/projects/lamad-ui/src/lib/components/__validation__
```

Write `app/elohim-library/projects/lamad-ui/src/lib/components/__validation__/test-content.md`:

```markdown
# Test Content

This is a paragraph from a markdown file imported into MDX.

- A list item
- Another list item
```

- [ ] **Step 2: Create the MDX wrapper using `Markdown` block from addon-docs**

Write `app/elohim-library/projects/lamad-ui/src/lib/components/__validation__/test-import.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from './test-content.md?raw';

<Meta title="__validation__/Test Import" />

<Markdown>{content}</Markdown>
```

Note: the existing `lamad-ui` MDX files import only `{ Meta }` from `@storybook/addon-docs/blocks`. This task verifies whether `Markdown` is also exported from that path. If the import fails at build time, fall through to Step 3.

- [ ] **Step 3: Build storybook and verify the page renders**

Run from `app/elohim-library/`:

```bash
pnpm run build-storybook
```

Expected: build succeeds. Search the bundle:

```bash
grep -l "Test Import" app/elohim-library/dist/storybook/index.json
```

Expected: matches. If `Markdown` is NOT exported from `@storybook/addon-docs/blocks`, the build fails with an "import not found" error. In that case:

- Try alternative import: `import { Markdown } from '@storybook/blocks'` (may need to add the package).
- If that doesn't work either, pivot to **fallback approach**: have `sync-genesis.mjs` GENERATE complete MDX wrappers (inline the markdown content directly into the MDX body) instead of using a `<Markdown>` block. Update Task 4 step 4 accordingly. The rest of the plan adapts: hand-authored static wrappers in Tasks 6, 9, 10 also become generated.

Document the chosen approach in a one-line code comment at the top of `scripts/sync-genesis.mjs` when written: `// MDX rendering: <Markdown> block | inlined-content` — pick whichever survives.

- [ ] **Step 4: Run storybook locally and visually verify**

Run from `app/elohim-library/`:

```bash
pnpm run storybook
```

Open http://localhost:6006 (or the dev-server port shown). Navigate the sidebar to `__validation__/Test Import`. Expected: heading, paragraph, and bullet list render correctly.

Stop the server (`Ctrl-C`).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-library/projects/lamad-ui/src/lib/components/__validation__/
git commit -m "$(cat <<'EOF'
spec(graphos): validate MDX content-import rendering pattern

Throwaway test page proving <Meta> + raw markdown ?raw import +
<Markdown> block renders correctly in Storybook 10 / addon-docs.
Removed in Task 2 once the graphos project bootstraps successfully.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Bootstrap the graphos library project

Create the `graphos` Angular project alongside `lamad-ui`. Mirror the existing project structure (ng-package.json, tsconfig.lib.json, src/public-api.ts) since the workspace's other libs follow that pattern. Register in angular.json. Verify storybook still builds.

**Files:**
- Create: `app/elohim-library/projects/graphos/ng-package.json`
- Create: `app/elohim-library/projects/graphos/tsconfig.lib.json`
- Create: `app/elohim-library/projects/graphos/src/public-api.ts`
- Create: `app/elohim-library/projects/graphos/src/imported/.gitignore`
- Modify: `app/elohim-library/angular.json` (add `graphos` to `projects`)
- Delete: `app/elohim-library/projects/lamad-ui/src/lib/components/__validation__/` (cleanup from Task 1)

- [ ] **Step 1: Create graphos project skeleton**

```bash
mkdir -p app/elohim-library/projects/graphos/src
```

Write `app/elohim-library/projects/graphos/ng-package.json`:

```json
{
  "$schema": "../../node_modules/ng-packagr/ng-package.schema.json",
  "dest": "../../dist/graphos",
  "lib": {
    "entryFile": "src/public-api.ts"
  }
}
```

Write `app/elohim-library/projects/graphos/tsconfig.lib.json`:

```json
{
  "extends": "../../tsconfig.json",
  "compilerOptions": {
    "outDir": "../../out-tsc/lib",
    "declaration": true,
    "declarationMap": true,
    "inlineSources": true,
    "types": []
  },
  "exclude": [
    "**/*.spec.ts"
  ]
}
```

Write `app/elohim-library/projects/graphos/src/public-api.ts`:

```typescript
// graphos — elohim-protocol-native design surface
// This library is consumed by Storybook only; no runtime exports yet.
```

- [ ] **Step 2: Set up imported/ gitignore**

```bash
mkdir -p app/elohim-library/projects/graphos/src/imported
```

Write `app/elohim-library/projects/graphos/src/imported/.gitignore`:

```
*
!.gitignore
```

- [ ] **Step 3: Register graphos in angular.json**

Open `app/elohim-library/angular.json`. Find the `"projects"` key. Add a new entry alongside `elohim-ui-playground` (and any existing library projects). Use the existing `lamad-ui` entry as a template — search for `"lamad-ui"` and copy that block.

The minimum graphos entry (storybook-only, not a published library, so we register only the bare minimum to satisfy ng workspace):

```json
"graphos": {
  "projectType": "library",
  "root": "projects/graphos",
  "sourceRoot": "projects/graphos/src",
  "prefix": "graphos",
  "architect": {
    "build": {
      "builder": "@angular-devkit/build-angular:ng-packagr",
      "options": {
        "project": "projects/graphos/ng-package.json",
        "tsConfig": "projects/graphos/tsconfig.lib.json"
      }
    }
  }
}
```

If `lamad-ui` is registered with additional architect targets (e.g., `test`, `lint`), copy those over too with paths swapped to `graphos`. If `lamad-ui` is NOT in `angular.json` (only `elohim-ui-playground` is), then this minimal entry is enough.

- [ ] **Step 4: Remove the validation files from Task 1**

```bash
rm -rf app/elohim-library/projects/lamad-ui/src/lib/components/__validation__/
```

- [ ] **Step 5: Verify storybook still builds**

From `app/elohim-library/`:

```bash
pnpm run build-storybook
```

Expected: build succeeds, no errors about graphos. The new project has no stories yet so it adds nothing to the bundle. Search verifies the validation page is gone:

```bash
grep "Test Import" app/elohim-library/dist/storybook/index.json
```

Expected: no match (validation page removed).

- [ ] **Step 6: Commit**

```bash
git add app/elohim-library/projects/graphos/ app/elohim-library/angular.json
git rm -r app/elohim-library/projects/lamad-ui/src/lib/components/__validation__/
git commit -m "$(cat <<'EOF'
feat(graphos): bootstrap library project for design surface

Empty graphos project alongside lamad-ui — ng-package.json,
tsconfig.lib.json, public-api.ts (empty), imported/.gitignore.
Registered in angular.json. Validation files from Task 1 removed.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Rebrand the Storybook chrome to "graphos"

Cosmetic change in `theme.ts`. Cheapest signal that the rename is real.

**Files:**
- Modify: `app/elohim-library/.storybook/theme.ts:7`

- [ ] **Step 1: Update brand title**

Open `app/elohim-library/.storybook/theme.ts`. Find the `brandTitle` line (line 7):

```typescript
  brandTitle: 'Lamad UI',
```

Change to:

```typescript
  brandTitle: 'graphos',
```

- [ ] **Step 2: Verify the change**

From `app/elohim-library/`:

```bash
pnpm run build-storybook 2>&1 | tail -20
grep -o 'graphos' app/elohim-library/dist/storybook/sb-manager/runtime.js | head -3
```

Expected: build succeeds; grep finds `graphos` in the manager bundle.

- [ ] **Step 3: Commit**

```bash
git add app/elohim-library/.storybook/theme.ts
git commit -m "$(cat <<'EOF'
feat(graphos): rebrand Storybook chrome from "Lamad UI" to "graphos"

Cosmetic-only update to theme.ts. Signals the conceptual rename;
deploy target stays at storybook.elohim.host per spec.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Implement sync-genesis.mjs core (single-file copy + validation)

The script is the single source of truth for the genesis-to-storybook mapping. This task lands the core: MAPPINGS table for single-file sources, file existence validation, and copy logic. Glob handling and gherkin transform come in later tasks.

**Files:**
- Create: `app/elohim-library/scripts/sync-genesis.mjs`
- Create: `app/elohim-library/scripts/sync-genesis.test.mjs`

- [ ] **Step 1: Write the unit test for path resolution and validation**

Write `app/elohim-library/scripts/sync-genesis.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, mkdirSync, rmSync, existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runSync, validateMappings, MAPPINGS } from './sync-genesis.mjs';

function setupFixtureRepo() {
  const root = mkdtempSync(join(tmpdir(), 'graphos-sync-'));
  const genesis = join(root, 'genesis');
  const out = join(root, 'graphos-imported');
  mkdirSync(genesis, { recursive: true });
  mkdirSync(out, { recursive: true });
  return { root, genesis, out };
}

test('validateMappings returns missing entries when sources do not exist', () => {
  const { root, genesis } = setupFixtureRepo();
  const mappings = [
    { from: 'docs/missing.md', to: 'narrative/why/manifesto.md', title: 'I. Why / Manifesto' },
  ];
  const missing = validateMappings(mappings, genesis);
  assert.equal(missing.length, 1);
  assert.match(missing[0].error, /missing/i);
  rmSync(root, { recursive: true, force: true });
});

test('validateMappings passes when source exists', () => {
  const { root, genesis } = setupFixtureRepo();
  mkdirSync(join(genesis, 'docs/content'), { recursive: true });
  writeFileSync(join(genesis, 'docs/content/manifesto.md'), '# Manifesto\n');
  const mappings = [
    { from: 'docs/content/manifesto.md', to: 'narrative/why/manifesto.md', title: 'I. Why / Manifesto' },
  ];
  const missing = validateMappings(mappings, genesis);
  assert.equal(missing.length, 0);
  rmSync(root, { recursive: true, force: true });
});

test('runSync copies single-file mapping to imported/', () => {
  const { root, genesis, out } = setupFixtureRepo();
  mkdirSync(join(genesis, 'docs/content'), { recursive: true });
  writeFileSync(join(genesis, 'docs/content/manifesto.md'), '# Hello\n');
  const mappings = [
    { from: 'docs/content/manifesto.md', to: 'narrative/why/manifesto.md', title: 'I. Why / Manifesto' },
  ];
  runSync(mappings, genesis, out);
  const expected = join(out, 'narrative/why/manifesto.md');
  assert.equal(existsSync(expected), true);
  assert.equal(readFileSync(expected, 'utf-8'), '# Hello\n');
  rmSync(root, { recursive: true, force: true });
});

test('MAPPINGS constant includes the manifesto entry', () => {
  const entry = MAPPINGS.find(m => m.from === 'docs/content/elohim-protocol/manifesto.md');
  assert.ok(entry, 'manifesto mapping must exist');
  assert.equal(entry.title, 'I. Why / Manifesto');
});
```

- [ ] **Step 2: Run the test to verify it fails (no script yet)**

```bash
cd app/elohim-library && node --test scripts/sync-genesis.test.mjs
```

Expected: FAIL with "Cannot find module './sync-genesis.mjs'" or similar.

- [ ] **Step 3: Write sync-genesis.mjs core**

Write `app/elohim-library/scripts/sync-genesis.mjs`:

```javascript
#!/usr/bin/env node
// MDX rendering: <Markdown> block (from @storybook/addon-docs/blocks) — confirmed in Task 1
//
// sync-genesis.mjs — single source of truth for genesis-to-graphos mapping.
//
// Copies mapped genesis files into projects/graphos/src/imported/ so that
// MDX wrappers can import them as ?raw at build time. Validates every
// mapping resolves; fails loudly otherwise.
//
// Glob mappings and gherkin transforms land in later tasks.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Repo root: app/elohim-library/scripts/sync-genesis.mjs → ../../../
const REPO_ROOT = resolve(__dirname, '..', '..', '..');
const GENESIS_DIR = join(REPO_ROOT, 'genesis');
const OUT_DIR = resolve(__dirname, '..', 'projects', 'graphos', 'src', 'imported');

export const MAPPINGS = [
  // I. Narrative Flow / Why
  { from: 'docs/content/elohim-protocol/manifesto.md',
    to: 'narrative/why/manifesto.md',
    title: 'I. Why / Manifesto' },
  { from: 'docs/content/elohim-protocol/constitution.md',
    to: 'narrative/why/constitution.md',
    title: 'I. Why / Constitution' },
  { from: 'docs/content/elohim-protocol/global-orchestra.md',
    to: 'narrative/why/vision.md',
    title: 'I. Why / Vision' },
  // I. Narrative Flow / What
  { from: 'graphos/elohim-protocol-design-spec.md',
    to: 'narrative/what/brand.md',
    title: 'I. What / Brand' },
  // I. Narrative Flow / How
  { from: 'docs/content/elohim-protocol/protocol-specification.md',
    to: 'narrative/how/protocol-specification.md',
    title: 'I. How / Protocol Specification' },
  { from: 'docs/content/elohim-protocol/governance-layers-architecture.md',
    to: 'narrative/how/governance-layers.md',
    title: 'I. How / Governance Layers' },
  { from: 'docs/content/elohim-protocol/epr-developer-guide.md',
    to: 'narrative/how/epr-developer-guide.md',
    title: 'I. How / EPR Developer Guide' },
  { from: 'docs/content/elohim-protocol/hardware-spec.md',
    to: 'narrative/how/hardware-spec.md',
    title: 'I. How / Hardware Spec' },
  // II. Foundations
  { from: 'graphos/vocabulary.md',
    to: 'foundations/vocabulary-register.md',
    title: 'II. Foundations / Vocabulary Register' },
  // III. Domains — single-file Reference Design (where genesis content exists)
  { from: 'docs/content/elohim-protocol/lamad.md',
    to: 'domains/learning/reference.md',
    title: 'III. Domains / Learning (Lamad) / Reference Design' },
];

export function validateMappings(mappings, genesisDir) {
  const missing = [];
  for (const m of mappings) {
    if (m.from) {
      const full = join(genesisDir, m.from);
      if (!existsSync(full)) {
        missing.push({ mapping: m, error: `Source file missing: ${full}` });
      }
    }
    // glob mappings handled in a later task
  }
  return missing;
}

export function runSync(mappings, genesisDir, outDir) {
  for (const m of mappings) {
    if (m.from && m.to) {
      const src = join(genesisDir, m.from);
      const dst = join(outDir, m.to);
      mkdirSync(dirname(dst), { recursive: true });
      const content = readFileSync(src, 'utf-8');
      writeFileSync(dst, content);
    }
  }
}

// CLI entrypoint
if (import.meta.url === `file://${process.argv[1]}`) {
  const missing = validateMappings(MAPPINGS, GENESIS_DIR);
  if (missing.length > 0) {
    console.error('sync-genesis: missing source files:');
    for (const m of missing) {
      console.error(`  - ${m.error} (would render as: ${m.mapping.title})`);
    }
    process.exit(1);
  }
  runSync(MAPPINGS, GENESIS_DIR, OUT_DIR);
  // Silent on success per spec.
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd app/elohim-library && node --test scripts/sync-genesis.test.mjs
```

Expected: 4 tests pass.

- [ ] **Step 5: Run sync-genesis.mjs against the real repo and verify output**

```bash
cd app/elohim-library && node scripts/sync-genesis.mjs
```

Expected: exits 0 silently. Verify output:

```bash
ls app/elohim-library/projects/graphos/src/imported/narrative/why/
ls app/elohim-library/projects/graphos/src/imported/foundations/
ls app/elohim-library/projects/graphos/src/imported/domains/learning/
```

Expected:
- `narrative/why/` contains `manifesto.md`, `constitution.md`, `vision.md`
- `foundations/` contains `vocabulary-register.md`
- `domains/learning/` contains `reference.md`

If any source file is genuinely missing in genesis (e.g., `epr-developer-guide.md` has been renamed since spec was written), the script exits non-zero with a clear list. Investigate the genesis path; either the mapping is wrong or the source file needs to be added to genesis. Update `MAPPINGS` accordingly and rerun.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-library/scripts/sync-genesis.mjs app/elohim-library/scripts/sync-genesis.test.mjs
git commit -m "$(cat <<'EOF'
feat(graphos): sync-genesis.mjs core — single-file copy + validation

MAPPINGS table for I/II/III-single-file sources. validateMappings()
checks every entry resolves; runSync() copies into imported/.
Unit-tested. Glob handling + gherkin transform land in next tasks.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Wire sync-genesis into pnpm scripts

Make every `pnpm run storybook` and `pnpm run build-storybook` invocation run sync-genesis first. Use named pre-hooks rather than chained `&&` so the existing scripts stay readable.

**Files:**
- Modify: `app/elohim-library/package.json:5-13`

- [ ] **Step 1: Add pre-hooks to package.json**

Open `app/elohim-library/package.json`. The current `scripts` block (lines 5-13) contains:

```json
"scripts": {
  "ng": "ng",
  "start": "ng serve",
  "build": "ng build",
  "watch": "ng build --watch --configuration development",
  "test": "vitest run --config vite.config.ts --passWithNoTests",
  "test:service": "cd projects/elohim-service && pnpm exec jest",
  "lint": "eslint projects/elohim-service/src projects/lamad-ui/src projects/html5-app-plugin/src",
  "lint:fix": "eslint projects/elohim-service/src projects/lamad-ui/src projects/html5-app-plugin/src --fix",
  "storybook": "ng run elohim-ui-playground:storybook",
  "build-storybook": "ng run elohim-ui-playground:build-storybook"
},
```

Change to:

```json
"scripts": {
  "ng": "ng",
  "start": "ng serve",
  "build": "ng build",
  "watch": "ng build --watch --configuration development",
  "test": "vitest run --config vite.config.ts --passWithNoTests",
  "test:service": "cd projects/elohim-service && pnpm exec jest",
  "test:sync-genesis": "node --test scripts/sync-genesis.test.mjs",
  "lint": "eslint projects/elohim-service/src projects/lamad-ui/src projects/html5-app-plugin/src",
  "lint:fix": "eslint projects/elohim-service/src projects/lamad-ui/src projects/html5-app-plugin/src --fix",
  "sync-genesis": "node scripts/sync-genesis.mjs",
  "prestorybook": "pnpm run sync-genesis",
  "storybook": "ng run elohim-ui-playground:storybook",
  "prebuild-storybook": "pnpm run sync-genesis",
  "build-storybook": "ng run elohim-ui-playground:build-storybook"
},
```

Note pnpm naming: pnpm honors the `pre<script>` convention. `prestorybook` runs before `storybook`; `prebuild-storybook` runs before `build-storybook`.

- [ ] **Step 2: Verify the pre-hook fires on build**

```bash
rm -rf app/elohim-library/projects/graphos/src/imported/narrative
cd app/elohim-library && pnpm run build-storybook 2>&1 | head -20
ls app/elohim-library/projects/graphos/src/imported/narrative/why/
```

Expected: build succeeds; the `narrative/` directory is regenerated even though we deleted it before running.

- [ ] **Step 3: Verify test:sync-genesis script works**

```bash
cd app/elohim-library && pnpm run test:sync-genesis
```

Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-library/package.json
git commit -m "$(cat <<'EOF'
feat(graphos): wire sync-genesis into prestorybook/prebuild-storybook hooks

Every storybook/build-storybook invocation now syncs genesis content
into imported/ first. Standalone `pnpm run sync-genesis` and
`pnpm run test:sync-genesis` scripts available for CI and dev.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Static MDX wrappers — Sections I (Narrative) + II (Foundations)

Create the 12 static MDX wrappers for single-file-source pages. Each wrapper is the same 5-line template — only the title and import path differ. After writing, verify each appears as a story in storybook.

**Files (all under `app/elohim-library/projects/graphos/src/`):**
- Create: `narrative/why/__docs__/manifesto.mdx`
- Create: `narrative/why/__docs__/constitution.mdx`
- Create: `narrative/why/__docs__/vision.mdx`
- Create: `narrative/what/__docs__/brand.mdx`
- Create: `narrative/how/__docs__/protocol-specification.mdx`
- Create: `narrative/how/__docs__/governance-layers.mdx`
- Create: `narrative/how/__docs__/epr-developer-guide.mdx`
- Create: `narrative/how/__docs__/hardware-spec.mdx`
- Create: `foundations/__docs__/vocabulary-register.mdx`
- Create: `foundations/__docs__/epr-elements.mdx` (placeholder)
- Create: `foundations/__docs__/rea-primitives.mdx` (placeholder)
- Create: `foundations/__docs__/brand-atoms.mdx` (placeholder)
- Create: `foundations/__docs__/component-atoms.mdx` (placeholder)

- [ ] **Step 1: Create directory structure**

```bash
cd app/elohim-library/projects/graphos/src
mkdir -p narrative/why/__docs__ narrative/what/__docs__ narrative/how/__docs__ foundations/__docs__
```

- [ ] **Step 2: Write the Section I wrapper template (using manifesto as exemplar)**

Write `narrative/why/__docs__/manifesto.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/narrative/why/manifesto.md?raw';

<Meta title="I. Why / Manifesto" />

<Markdown>{content}</Markdown>
```

Write `narrative/why/__docs__/constitution.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/narrative/why/constitution.md?raw';

<Meta title="I. Why / Constitution" />

<Markdown>{content}</Markdown>
```

Write `narrative/why/__docs__/vision.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/narrative/why/vision.md?raw';

<Meta title="I. Why / Vision" />

<Markdown>{content}</Markdown>
```

Write `narrative/what/__docs__/brand.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/narrative/what/brand.md?raw';

<Meta title="I. What / Brand" />

<Markdown>{content}</Markdown>
```

Write `narrative/how/__docs__/protocol-specification.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/narrative/how/protocol-specification.md?raw';

<Meta title="I. How / Protocol Specification" />

<Markdown>{content}</Markdown>
```

Write `narrative/how/__docs__/governance-layers.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/narrative/how/governance-layers.md?raw';

<Meta title="I. How / Governance Layers" />

<Markdown>{content}</Markdown>
```

Write `narrative/how/__docs__/epr-developer-guide.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/narrative/how/epr-developer-guide.md?raw';

<Meta title="I. How / EPR Developer Guide" />

<Markdown>{content}</Markdown>
```

Write `narrative/how/__docs__/hardware-spec.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/narrative/how/hardware-spec.md?raw';

<Meta title="I. How / Hardware Spec" />

<Markdown>{content}</Markdown>
```

- [ ] **Step 3: Write the Section II Foundations wrapper for the real source**

Write `foundations/__docs__/vocabulary-register.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../imported/foundations/vocabulary-register.md?raw';

<Meta title="II. Foundations / Vocabulary Register" />

<Markdown>{content}</Markdown>
```

- [ ] **Step 4: Write the four Foundations placeholders**

Each placeholder has the same shape: a `<Meta>` declaration plus a one-line "pending" message and a source pointer. No content import.

Write `foundations/__docs__/epr-elements.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="II. Foundations / EPR Elements" />

# EPR Elements

*Pending — to be authored as a separate spec. Source materials live in `elohim/epr/` and `elohim/sdk/epr-ts/`.*

## What this slot will contain

The Elohim Protocol's foundational elements: content-coupling, reach gates, signal kinds, and the protocol primitives every domain composes from. Documented as Foundations because all five domains (Identity, Learning, Community, Economy, Doorway) depend on them.

## Status

Slot reserved in graphos sprint 1 (narrative scaffold). Population deferred to a later EPR-Elements documentation spec.
```

Write `foundations/__docs__/rea-primitives.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="II. Foundations / REA Primitives" />

# REA Primitives

*Pending — to be authored as a separate spec. Source materials in genesis content under `economic_coordination/` and the `rea-economics` skill.*

## What this slot will contain

Resource–Event–Agent ontology as instantiated in the protocol: `Agent`, `Resource`, `Event`, `Commitment`, `EconomicEvent`. The shared vocabulary every Shefa flow speaks, also reachable from Lamad (recognition events) and Qahal (commitment ledgers).

## Status

Slot reserved in graphos sprint 1 (narrative scaffold). Population deferred to a later REA-Primitives documentation spec.
```

Write `foundations/__docs__/brand-atoms.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="II. Foundations / Brand Atoms" />

# Brand Atoms

*Pending — to be authored as a separate spec. Source materials in `genesis/graphos/elohim-protocol-design-spec.md` and `genesis/graphos/fonts/`.*

## What this slot will contain

The visual atoms of graphos: constellation mark, color palette (Vineyard, Hearthstone, Linen, Starlight, etc.), typography, motion vocabulary. Rendered as live Storybook stories where designers can grab tokens and see live components.

## Status

Slot reserved in graphos sprint 1 (narrative scaffold). Population deferred to a later Brand-Atoms documentation spec.
```

Write `foundations/__docs__/component-atoms.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="II. Foundations / Component Atoms" />

# Component Atoms

*Pending — sprint 2 populates as components migrate from `app/elohim-app/` and `doorway/doorway-app/`.*

## What this slot will contain

Cross-domain atomic components: buttons, badges, layout primitives, form controls. The shared component substrate every domain's organisms compose from.

## Status

Slot reserved in graphos sprint 1 (narrative scaffold). Population is sprint 2's primary deliverable — see component migration plan.
```

- [ ] **Step 5: Build storybook and verify all 13 pages exist**

```bash
cd app/elohim-library && pnpm run build-storybook 2>&1 | tail -20
```

Expected: build succeeds. Verify pages:

```bash
for title in "Manifesto" "Constitution" "Vision" "Brand" "Protocol Specification" "Governance Layers" "EPR Developer Guide" "Hardware Spec" "Vocabulary Register" "EPR Elements" "REA Primitives" "Brand Atoms" "Component Atoms"; do
  if grep -q "$title" app/elohim-library/dist/storybook/index.json; then
    echo "FOUND: $title"
  else
    echo "MISSING: $title"
  fi
done
```

Expected: all 13 lines say FOUND.

- [ ] **Step 6: Visually verify in dev server (sample 2-3 pages)**

```bash
cd app/elohim-library && pnpm run storybook
```

Open http://localhost:6006. Navigate sidebar:
- `I. Why / Manifesto` — should show the manifesto's heading and prose, fully rendered.
- `II. Foundations / Vocabulary Register` — should show the quilt/pantry/stock/draw entries.
- `II. Foundations / EPR Elements` — should show the placeholder content with "Pending" italics.

Stop server (Ctrl-C).

- [ ] **Step 7: Commit**

```bash
git add app/elohim-library/projects/graphos/src/narrative/ app/elohim-library/projects/graphos/src/foundations/
git commit -m "$(cat <<'EOF'
feat(graphos): static MDX wrappers for Sections I and II

13 static wrappers — 8 narrative pages (Why/What/How) + 5 foundations
(Vocabulary Register + 4 placeholders). All single-file sources;
content imported via ?raw and rendered through <Markdown> block.
Placeholders carry "pending" markers and source pointers for the
eventual content sprints.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Add gherkin pre-render + glob mapping support to sync-genesis.mjs

Extend the script to handle `fromGlob` mappings: walk the glob, transform each `.feature` file to fenced markdown, and emit a generated MDX wrapper alongside the imported markdown. This is the heart of the dynamic-content path.

**Files:**
- Modify: `app/elohim-library/scripts/sync-genesis.mjs`
- Modify: `app/elohim-library/scripts/sync-genesis.test.mjs`
- Modify: `app/elohim-library/.gitignore` (or root `.gitignore`)

- [ ] **Step 1: Decide where generated MDX wrappers live and gitignore them**

Generated wrappers go under `__docs__/_generated/` within each domain/reference directory. Add a gitignore rule.

Open `app/elohim-library/.gitignore` (create if missing — check first with `ls app/elohim-library/.gitignore`):

If it doesn't exist, create it with:

```
projects/*/src/**/__docs__/_generated/
```

If it exists, append that line.

Note: an alternative is the root `.gitignore`, but library-local keeps the rule near the code it governs.

- [ ] **Step 2: Write tests for gherkin transform and glob expansion**

In `app/elohim-library/scripts/sync-genesis.test.mjs`, edit the existing import line at the top to include the new exports:

```javascript
import { runSync, validateMappings, MAPPINGS, gherkinToMarkdown, expandGlob, runSyncWithGlobs } from './sync-genesis.mjs';
```

Then append the new tests at the bottom of the file:

```javascript
test('gherkinToMarkdown wraps feature content in a code fence with gherkin language', () => {
  const featureContent = `Feature: Auth\n  Scenario: Login\n    When user logs in\n    Then they are authenticated\n`;
  const md = gherkinToMarkdown(featureContent, 'auth.feature');
  assert.match(md, /```gherkin/);
  assert.match(md, /```\n*$/);
  assert.match(md, /Feature: Auth/);
});

test('expandGlob finds .feature files matching the pattern', () => {
  const { root, genesis } = setupFixtureRepo();
  mkdirSync(join(genesis, 'a2o/features/auth'), { recursive: true });
  writeFileSync(join(genesis, 'a2o/features/auth/login.feature'), 'Feature: Login\n');
  writeFileSync(join(genesis, 'a2o/features/auth/recovery.feature'), 'Feature: Recovery\n');
  writeFileSync(join(genesis, 'a2o/features/auth/notes.txt'), 'ignored\n');
  const matches = expandGlob('a2o/features/auth/*.feature', genesis);
  assert.equal(matches.length, 2);
  assert.ok(matches.some(p => p.endsWith('login.feature')));
  assert.ok(matches.some(p => p.endsWith('recovery.feature')));
  rmSync(root, { recursive: true, force: true });
});

test('runSyncWithGlobs writes both an imported .md AND a generated .mdx wrapper per match', () => {
  const { root, genesis, out } = setupFixtureRepo();
  mkdirSync(join(genesis, 'a2o/features/auth'), { recursive: true });
  writeFileSync(join(genesis, 'a2o/features/auth/login.feature'), 'Feature: Login\n  Scenario: x\n');
  const wrappersDir = join(out, '..', 'graphos-wrappers');
  mkdirSync(wrappersDir, { recursive: true });
  const mappings = [
    { fromGlob: 'a2o/features/auth/*.feature',
      toDir: 'domains/identity/stories/',
      titleFn: (name) => `III. Domains / Identity (Imagodei) / Stories / ${name}` },
  ];
  runSyncWithGlobs(mappings, genesis, out, wrappersDir);
  const importedMd = join(out, 'domains/identity/stories/login.md');
  const generatedMdx = join(wrappersDir, 'domains/identity/__docs__/_generated/login.mdx');
  assert.equal(existsSync(importedMd), true, 'imported markdown should exist');
  assert.equal(existsSync(generatedMdx), true, 'generated MDX wrapper should exist');
  const mdxContent = readFileSync(generatedMdx, 'utf-8');
  assert.match(mdxContent, /III\. Domains \/ Identity \(Imagodei\) \/ Stories \/ Login/);
  assert.match(mdxContent, /<Markdown>\{content\}<\/Markdown>/);
  rmSync(root, { recursive: true, force: true });
});
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd app/elohim-library && pnpm run test:sync-genesis
```

Expected: 3 new tests fail with "is not a function" or import errors.

- [ ] **Step 4: Implement gherkinToMarkdown, expandGlob, runSyncWithGlobs**

Edit `app/elohim-library/scripts/sync-genesis.mjs`. Add these exports BEFORE the CLI entrypoint block:

```javascript
import { readdirSync, statSync } from 'node:fs';
import { basename, extname } from 'node:path';

export function gherkinToMarkdown(featureContent, fileName) {
  // Wrap the raw .feature in a fenced code block with `gherkin` language.
  // Storybook's Markdown block handles syntax highlighting via prismjs.
  const titleMatch = featureContent.match(/^Feature:\s*(.+)$/m);
  const heading = titleMatch ? `# ${titleMatch[1].trim()}` : `# ${fileName}`;
  return `${heading}\n\n_Source: \`${fileName}\`_\n\n\`\`\`gherkin\n${featureContent.trimEnd()}\n\`\`\`\n`;
}

export function expandGlob(pattern, baseDir) {
  // Minimal glob: supports `<segments>/*.<ext>` only — no `**`, no character classes.
  // Add complexity only if a future mapping requires it.
  const parts = pattern.split('/');
  const fileGlob = parts.pop();
  const dir = join(baseDir, parts.join('/'));
  if (!existsSync(dir) || !statSync(dir).isDirectory()) return [];
  const ext = fileGlob.replace(/^\*/, '');
  return readdirSync(dir)
    .filter(name => name.endsWith(ext) && !name.startsWith('.'))
    .map(name => join(dir, name));
}

function toTitleCase(slug) {
  return slug
    .replace(/[-_]/g, ' ')
    .replace(/\b\w/g, c => c.toUpperCase());
}

export function runSyncWithGlobs(mappings, genesisDir, outDir, wrappersBase) {
  for (const m of mappings) {
    if (!m.fromGlob) continue;
    const matches = expandGlob(m.fromGlob, genesisDir);
    for (const sourcePath of matches) {
      const fileName = basename(sourcePath);
      const slug = fileName.replace(/\.feature$/, '');
      const niceName = toTitleCase(slug);
      const featureContent = readFileSync(sourcePath, 'utf-8');
      const md = gherkinToMarkdown(featureContent, fileName);

      // 1. Write the imported markdown content
      const mdDest = join(outDir, m.toDir, `${slug}.md`);
      mkdirSync(dirname(mdDest), { recursive: true });
      writeFileSync(mdDest, md);

      // 2. Write the generated MDX wrapper.
      // Wrapper lives under wrappersBase/<toDir-with-_generated-injected>/<slug>.mdx
      // toDir example: 'domains/identity/stories/' → 'domains/identity/__docs__/_generated/'
      const sectionPath = m.toDir.replace(/\/[^/]+\/?$/, '/__docs__/_generated/');
      const mdxDest = join(wrappersBase, sectionPath, `${slug}.mdx`);
      mkdirSync(dirname(mdxDest), { recursive: true });
      const title = m.titleFn(niceName);
      // Compute the relative import path from the wrapper to the imported md.
      // Wrapper is at: <wrappersBase>/<sectionPath>/<slug>.mdx
      // Markdown is at: <outDir>/<toDir>/<slug>.md
      // Both share <wrappersBase> == <outDir>'s parent (graphos/src), so we
      // build a relative path from sectionPath to toDir.
      const importRelPath = relativeFromTo(sectionPath, m.toDir, slug);
      const mdxContent = `import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '${importRelPath}?raw';

<Meta title="${title}" />

<Markdown>{content}</Markdown>
`;
      writeFileSync(mdxDest, mdxContent);
    }
  }
}

function relativeFromTo(fromDir, toDir, slug) {
  // Both are relative to the same root (graphos/src). Compute the relative
  // path between them, then append the file.
  const fromParts = fromDir.split('/').filter(Boolean);
  const toParts = toDir.split('/').filter(Boolean);
  // Replace the wrappersBase prefix conceptually (it's parent of imported/).
  // Wrapper imports from imported/, so we go up from fromParts to graphos/src,
  // then descend into imported/<toParts>/<slug>.md
  const upDirs = fromParts.length;
  const ups = '../'.repeat(upDirs);
  return `${ups}imported/${toParts.join('/')}/${slug}.md`;
}
```

Then update the CLI entrypoint to invoke `runSyncWithGlobs` after `runSync`. Find the CLI block at the bottom of the file:

```javascript
// CLI entrypoint
if (import.meta.url === `file://${process.argv[1]}`) {
  const missing = validateMappings(MAPPINGS, GENESIS_DIR);
  if (missing.length > 0) {
    console.error('sync-genesis: missing source files:');
    for (const m of missing) {
      console.error(`  - ${m.error} (would render as: ${m.mapping.title})`);
    }
    process.exit(1);
  }
  runSync(MAPPINGS, GENESIS_DIR, OUT_DIR);
  // Silent on success per spec.
}
```

Replace with:

```javascript
// CLI entrypoint
const WRAPPERS_BASE = resolve(__dirname, '..', 'projects', 'graphos', 'src');

if (import.meta.url === `file://${process.argv[1]}`) {
  const missing = validateMappings(MAPPINGS, GENESIS_DIR);
  if (missing.length > 0) {
    console.error('sync-genesis: missing source files:');
    for (const m of missing) {
      console.error(`  - ${m.error} (would render as: ${m.mapping.title})`);
    }
    process.exit(1);
  }
  runSync(MAPPINGS, GENESIS_DIR, OUT_DIR);
  runSyncWithGlobs(MAPPINGS, GENESIS_DIR, OUT_DIR, WRAPPERS_BASE);
  // Silent on success per spec.
}
```

(Glob mappings will be added to MAPPINGS in Task 8 — for now the CLI runs against an empty glob set and is a no-op.)

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd app/elohim-library && pnpm run test:sync-genesis
```

Expected: all 7 tests pass.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-library/scripts/sync-genesis.mjs app/elohim-library/scripts/sync-genesis.test.mjs app/elohim-library/.gitignore
git commit -m "$(cat <<'EOF'
feat(graphos): gherkin pre-render + glob mapping in sync-genesis

gherkinToMarkdown wraps .feature content in a fenced code block with
gherkin language. expandGlob handles the simple `<dir>/*.<ext>` form
needed by sprint 1. runSyncWithGlobs writes both the imported markdown
and the generated MDX wrapper per match. Generated wrappers go under
__docs__/_generated/ and are gitignored.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Add glob mappings to MAPPINGS table — Section III Stories + Section IV Reference

Now the engine handles globs, populate the MAPPINGS table with the Stories and Reference glob entries from the spec. Verify storybook discovers the generated wrappers.

**Files:**
- Modify: `app/elohim-library/scripts/sync-genesis.mjs` (extend MAPPINGS)

- [ ] **Step 1: Append glob mappings to the MAPPINGS array**

Open `app/elohim-library/scripts/sync-genesis.mjs`. Find the MAPPINGS array. Append these entries before the closing `]`:

```javascript
  // III. Domains — globs (each glob produces one wrapper per match)
  { fromGlob: 'a2o/features/auth/*.feature',
    toDir: 'domains/identity/stories/',
    titleFn: (name) => `III. Domains / Identity (Imagodei) / Stories / ${name}` },
  { fromGlob: 'a2o/features/lamad/*.feature',
    toDir: 'domains/learning/stories/',
    titleFn: (name) => `III. Domains / Learning (Lamad) / Stories / ${name}` },
  { fromGlob: 'a2o/features/content/*.feature',
    toDir: 'domains/learning/stories/',
    titleFn: (name) => `III. Domains / Learning (Lamad) / Stories / ${name}` },
  { fromGlob: 'a2o/features/qahal/*.feature',
    toDir: 'domains/community/stories/',
    titleFn: (name) => `III. Domains / Community (Qahal) / Stories / ${name}` },
  { fromGlob: 'a2o/features/shefa/*.feature',
    toDir: 'domains/economy/stories/',
    titleFn: (name) => `III. Domains / Economy (Shefa) / Stories / ${name}` },
  { fromGlob: 'a2o/features/delivery/*.feature',
    toDir: 'domains/doorway/stories/',
    titleFn: (name) => `III. Domains / Doorway / Stories / ${name}` },
  { fromGlob: 'a2o/features/browser/*.feature',
    toDir: 'domains/doorway/stories/',
    titleFn: (name) => `III. Domains / Doorway / Stories / ${name}` },
  // IV. Reference — globs
  { fromGlob: 'a2o/features/federation/*.feature',
    toDir: 'reference/federation/',
    titleFn: (name) => `IV. Reference / Federation / ${name}` },
  { fromGlob: 'a2o/features/resilience/*.feature',
    toDir: 'reference/resilience/',
    titleFn: (name) => `IV. Reference / Resilience / ${name}` },
  { fromGlob: 'a2o/features/deployment/*.feature',
    toDir: 'reference/deployment/',
    titleFn: (name) => `IV. Reference / Deployment / ${name}` },
  { fromGlob: 'a2o/features/elohim/*.feature',
    toDir: 'reference/cross-cutting/',
    titleFn: (name) => `IV. Reference / Cross-cutting Stories / ${name}` },
```

- [ ] **Step 2: Run sync-genesis and verify generated output**

```bash
cd app/elohim-library && pnpm run sync-genesis
```

Expected: exits 0 silently.

```bash
ls app/elohim-library/projects/graphos/src/imported/domains/identity/stories/
ls app/elohim-library/projects/graphos/src/domains/identity/__docs__/_generated/
ls app/elohim-library/projects/graphos/src/reference/federation/__docs__/_generated/ 2>/dev/null
```

Expected:
- `imported/domains/identity/stories/` contains one `.md` per `.feature` in `genesis/a2o/features/auth/`
- `domains/identity/__docs__/_generated/` contains one `.mdx` per `.feature`
- `reference/federation/__docs__/_generated/` exists if `genesis/a2o/features/federation/` has any `.feature` files; doesn't exist otherwise

- [ ] **Step 3: Build storybook and verify stories appear**

```bash
cd app/elohim-library && pnpm run build-storybook 2>&1 | tail -20
grep -o 'III. Domains / Identity (Imagodei) / Stories / [^"]*' app/elohim-library/dist/storybook/index.json | sort -u | head -10
grep -o 'IV. Reference / [^"]*' app/elohim-library/dist/storybook/index.json | sort -u | head -10
```

Expected:
- multiple `III. Domains / Identity (Imagodei) / Stories / ...` matches (one per auth feature file)
- multiple `IV. Reference / ...` matches

- [ ] **Step 4: Visually spot-check one story page**

```bash
cd app/elohim-library && pnpm run storybook
```

Open http://localhost:6006. Navigate to `III. Domains / Identity (Imagodei) / Stories / Auth Lifecycle` (or whatever feature file exists). Expected: renders as a markdown page with a fenced gherkin block showing Feature, Scenario, Given/When/Then. Code highlighting may be plain (Storybook's Markdown block uses prism by default and may or may not highlight gherkin specifically — acceptable for sprint 1).

Stop server (Ctrl-C).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-library/scripts/sync-genesis.mjs
git commit -m "$(cat <<'EOF'
feat(graphos): glob mappings for Section III Stories + Section IV Reference

11 glob entries auto-render every .feature in
genesis/a2o/features/{auth,lamad,content,qahal,shefa,delivery,browser,
federation,resilience,deployment,elohim}/ as a Storybook page under
the appropriate IA slot. Generated MDX wrappers + imported markdown
both gitignored.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Domain landing pages + Reference Design + Components placeholders

Each of the 5 domains gets three static pages: a landing `index.mdx`, a `reference.mdx`, and a `components.mdx`. Learning's `reference.mdx` imports the real `lamad.md` content; the other 4 are placeholders. Components is always a placeholder.

**Files (all under `app/elohim-library/projects/graphos/src/domains/`):**
- Create: `identity/__docs__/index.mdx`
- Create: `identity/__docs__/reference.mdx` (placeholder)
- Create: `identity/__docs__/components.mdx` (placeholder)
- Create: `learning/__docs__/index.mdx`
- Create: `learning/__docs__/reference.mdx` (real source)
- Create: `learning/__docs__/components.mdx` (placeholder)
- Create: `community/__docs__/index.mdx`
- Create: `community/__docs__/reference.mdx` (placeholder)
- Create: `community/__docs__/components.mdx` (placeholder)
- Create: `economy/__docs__/index.mdx`
- Create: `economy/__docs__/reference.mdx` (placeholder)
- Create: `economy/__docs__/components.mdx` (placeholder)
- Create: `doorway/__docs__/index.mdx`
- Create: `doorway/__docs__/reference.mdx` (placeholder)
- Create: `doorway/__docs__/components.mdx` (placeholder)

- [ ] **Step 1: Create domain directories**

```bash
cd app/elohim-library/projects/graphos/src/domains
for d in identity learning community economy doorway; do
  mkdir -p "$d/__docs__"
done
```

- [ ] **Step 2: Identity landing + placeholders**

Write `identity/__docs__/index.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Identity (Imagodei)" />

# Identity (Imagodei)

The protocol's **identity** reference implementation — presence, profile, recovery, capabilities, and the trust contracts that bind agents to their commitments.

## Stories

Generated from `genesis/a2o/features/auth/`. See the **Stories** sub-tree in the sidebar for individual scenarios.

## Reference Design

See `Reference Design` in the sidebar (placeholder — single-file source pending).

## Components

Component documentation arrives in sprint 2 (component migration from `app/elohim-app/src/app/imagodei/components/`).
```

Write `identity/__docs__/reference.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Identity (Imagodei) / Reference Design" />

# Identity Reference Design

*Pending — no single-file source in `genesis/docs/content/elohim-protocol/` for the identity domain yet. Source materials when consolidated will likely live alongside `lamad.md` as `imagodei.md` (or be folded into a broader "domain reference" pattern).*

## What this slot will contain

The opinionated Imagodei design — presence vs. profile vs. account-management surfaces, recovery archetypes (intimate circle / qahal governance / global elohim witness), capability grants, the stewardship lifecycle (cradle-to-grave). Renders the protocol's identity stance, not just the wire shape.

## Status

Slot reserved in graphos sprint 1 (narrative scaffold). When `genesis/docs/content/elohim-protocol/imagodei.md` (or equivalent) is authored, add a single-file mapping in `sync-genesis.mjs` and replace this placeholder with the wrapper that imports it.
```

Write `identity/__docs__/components.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Identity (Imagodei) / Components" />

# Identity Components

*Component documentation arrives in sprint 2 (component migration from `app/elohim-app/src/app/imagodei/components/`).*

## What this slot will contain

Component stories for the imagodei pillar: account-switcher, agency-badge, appeal-wizard, capabilities-dashboard, login, profile, recovery-interview, recovery-request, register, stewardship-dashboard, and others. Each component will land here with a `.stories.ts` showing default/edge cases plus an MDX page documenting the design intent.

## Status

Reserved by sprint 1. Sprint 2 (component migration) populates this slot.
```

- [ ] **Step 3: Learning landing + real reference + placeholder components**

Write `learning/__docs__/index.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Learning (Lamad)" />

# Learning (Lamad)

The protocol's **learning** reference implementation — content, paths, assessments, mastery, recognition, and the practice flows that turn information into formation.

## Stories

Generated from `genesis/a2o/features/lamad/` and `genesis/a2o/features/content/`. See the **Stories** sub-tree in the sidebar for individual scenarios.

## Reference Design

See `Reference Design` in the sidebar — imports `genesis/docs/content/elohim-protocol/lamad.md`.

## Components

Component documentation arrives in sprint 2 (component migration from `app/elohim-app/src/app/lamad/components/`).
```

Write `learning/__docs__/reference.mdx`:

```mdx
import { Meta, Markdown } from '@storybook/addon-docs/blocks';
import content from '../../../imported/domains/learning/reference.md?raw';

<Meta title="III. Domains / Learning (Lamad) / Reference Design" />

<Markdown>{content}</Markdown>
```

Path note: `__docs__/` is three levels deep from `src/` for domain wrappers (`src/domains/<domain>/__docs__/`), so the import uses three `../`. Compare with single-section wrappers like `foundations/__docs__/` which use two `../`.

Write `learning/__docs__/components.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Learning (Lamad) / Components" />

# Learning Components

*Component documentation arrives in sprint 2 (component migration from `app/elohim-app/src/app/lamad/components/`).*

## What this slot will contain

Component stories for the lamad pillar: concept-card, content-viewer, content-editor-page, focused-view-toggle, graph-explorer, learner-dashboard, lesson-view, meaning-map, path-navigator, path-overview, related-concepts-panel, search, and others. Each migrates from the elohim-app tree and gains a story documenting design intent + interaction states.

## Status

Reserved by sprint 1. Sprint 2 populates this slot.
```

- [ ] **Step 4: Community landing + placeholders**

Write `community/__docs__/index.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Community (Qahal)" />

# Community (Qahal)

The protocol's **community** reference implementation — governance, consent, affinity, challenge/proposal/response cycles, sensemaking flows, and the institutional substrate that lets a group act as one.

## Stories

Generated from `genesis/a2o/features/qahal/`. See the **Stories** sub-tree in the sidebar for individual scenarios.

## Reference Design

See `Reference Design` in the sidebar (placeholder — directory-sourced content pending consolidation).

## Components

Component documentation arrives in sprint 2 (component migration from `app/elohim-app/src/app/qahal/components/`).
```

Write `community/__docs__/reference.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Community (Qahal) / Reference Design" />

# Community Reference Design

*Pending — current source materials are directory-sourced (`genesis/docs/content/elohim-protocol/governance/`, `governance_layers/`). Sprint 1's mapping table only handles single-file sources. When the directory contents are consolidated into a single `qahal.md` (or a curated list of single files), add mapping entries to `sync-genesis.mjs` and replace this placeholder with content wrappers.*

## What this slot will contain

The opinionated Qahal design — governance layers, consent flows, affinity circles, challenge/proposal/response cycles, the institutional substrate that lets a group act as one. Anchored by the constitution and protocol-specification narrative pieces in Section I.

## Status

Slot reserved in graphos sprint 1 (narrative scaffold). Source consolidation needed before population.
```

Write `community/__docs__/components.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Community (Qahal) / Components" />

# Community Components

*Component documentation arrives in sprint 2 (component migration from `app/elohim-app/src/app/qahal/components/`).*

## What this slot will contain

Component stories for the qahal pillar: challenge-detail, challenge-list, community-directory, community-home, contribute-statement, face-card, feedback-aggregate, feedback-mechanism-gateway, file-appeal, file-challenge, governance-disposition, graduated-feedback, opinion-cluster, proposal-vote, psephos-ballot-wrapper, reaction-bar, sensemaking-page, and others.

## Status

Reserved by sprint 1. Sprint 2 populates this slot.
```

- [ ] **Step 5: Economy landing + placeholders**

Write `economy/__docs__/index.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Economy (Shefa)" />

# Economy (Shefa)

The protocol's **economy** reference implementation — stewardship contracts, banking bridges, resource flows, mutual credit, REA-shaped commitments and economic events, and the placement signals that turn coordination into care.

## Stories

Generated from `genesis/a2o/features/shefa/`. See the **Stories** sub-tree in the sidebar for individual scenarios.

## Reference Design

See `Reference Design` in the sidebar (placeholder — directory-sourced content pending consolidation).

## Components

Component documentation arrives in sprint 2 (component migration from `app/elohim-app/src/app/shefa/components/`).
```

Write `economy/__docs__/reference.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Economy (Shefa) / Reference Design" />

# Economy Reference Design

*Pending — current source materials are directory-sourced (`genesis/docs/content/elohim-protocol/economic_coordination/`). Sprint 1's mapping table only handles single-file sources. When the directory contents are consolidated into a single `shefa.md` (or a curated list of single files), add mapping entries to `sync-genesis.mjs`.*

## What this slot will contain

The opinionated Shefa design — stewardship contracts as DePIN policy, REA primitives in concrete flow, mutual credit (Unyt) bridges, banking integrations, placement signals (gaps/breaches/recoveries) as economic inputs to subsidy and recruitment.

## Status

Slot reserved in graphos sprint 1 (narrative scaffold). Source consolidation needed before population.
```

Write `economy/__docs__/components.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Economy (Shefa) / Components" />

# Economy Components

*Component documentation arrives in sprint 2 (component migration from `app/elohim-app/src/app/shefa/components/`).*

## What this slot will contain

Component stories for the shefa pillar: compute-needs, custodian-view, device-stewardship, journal-page, offline-node-alert, resource-explorer, shefa-dashboard, signals-card, storage-distribution, transaction-import, and others.

## Status

Reserved by sprint 1. Sprint 2 populates this slot.
```

- [ ] **Step 6: Doorway landing + placeholders**

Write `doorway/__docs__/index.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Doorway" />

# Doorway

The protocol's **gateway projection** — the Web2-legible surface that lets browsers, federations, and external systems reach into the peer-native substrate without speaking libp2p directly. Doorway routes are manifest-driven; doorway is a registry-driven proxy.

## Stories

Generated from `genesis/a2o/features/delivery/` and `genesis/a2o/features/browser/`. See the **Stories** sub-tree in the sidebar for individual scenarios.

## Reference Design

See `Reference Design` in the sidebar (placeholder — pending consolidation).

## Components

Component documentation arrives in sprint 2 (component migration from `doorway/doorway-app/src/app/components/`).
```

Write `doorway/__docs__/reference.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Doorway / Reference Design" />

# Doorway Reference Design

*Pending — Doorway's design intent is currently spread across multiple specs and the protocol-specification narrative. When consolidated into a single `doorway.md` source, add a mapping in `sync-genesis.mjs`.*

## What this slot will contain

The opinionated Doorway design — manifest-driven routing, AT Protocol / ActivityPub federation flavors as projections (not new entry types), CDN/DNS concerns, OAuth-pattern account graduation, the inside-out peer-registration model.

## Status

Slot reserved in graphos sprint 1 (narrative scaffold).
```

Write `doorway/__docs__/components.mdx`:

```mdx
import { Meta } from '@storybook/addon-docs/blocks';

<Meta title="III. Domains / Doorway / Components" />

# Doorway Components

*Component documentation arrives in sprint 2 (component migration from `doorway/doorway-app/src/app/components/`).*

## What this slot will contain

Component stories for the doorway-app: dashboard, dashboard tabs, doorway-browser, login, register, account, policy-console, and others. Operator-facing surfaces, distinct from the elohim-app's human-facing UIs.

## Status

Reserved by sprint 1. Sprint 2 populates this slot.
```

- [ ] **Step 7: Build storybook and verify all 15 domain pages exist**

```bash
cd app/elohim-library && pnpm run build-storybook 2>&1 | tail -10
for slot in "Identity (Imagodei)" "Identity (Imagodei) / Reference" "Identity (Imagodei) / Components" \
            "Learning (Lamad)" "Learning (Lamad) / Reference" "Learning (Lamad) / Components" \
            "Community (Qahal)" "Community (Qahal) / Reference" "Community (Qahal) / Components" \
            "Economy (Shefa)" "Economy (Shefa) / Reference" "Economy (Shefa) / Components" \
            "Doorway" "Doorway / Reference" "Doorway / Components"; do
  if grep -q "$slot" app/elohim-library/dist/storybook/index.json; then
    echo "FOUND: $slot"
  else
    echo "MISSING: $slot"
  fi
done
```

Expected: all 15 lines say FOUND.

- [ ] **Step 8: Spot-check Learning Reference Design renders the real lamad.md content**

```bash
cd app/elohim-library && pnpm run storybook
```

Open http://localhost:6006. Navigate to `III. Domains / Learning (Lamad) / Reference Design`. Expected: renders the actual content of `genesis/docs/content/elohim-protocol/lamad.md` (not a placeholder).

Stop server (Ctrl-C).

- [ ] **Step 9: Commit**

```bash
git add app/elohim-library/projects/graphos/src/domains/
git commit -m "$(cat <<'EOF'
feat(graphos): domain landing pages + Reference + Components placeholders

15 static MDX pages for the five domain top-levels (Identity/Imagodei,
Learning/Lamad, Community/Qahal, Economy/Shefa, Doorway). Each domain
gets a landing index, a reference.mdx (Learning's imports lamad.md;
others are placeholders pending source consolidation), and a
components.mdx placeholder ("arrives in sprint 2"). All authored
prose limited to one-line captions + slot-pending markers.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Update build manifest and orchestrator triggers

Storybook must rebuild when genesis content changes. Two files codify the trigger set: `app/elohim-library/build-manifest.json` (used by graph-walker) and `genesis/orchestrator/orchestrator-strategy.mjs` (used by the orchestrator). Both need extending.

**Files:**
- Modify: `app/elohim-library/build-manifest.json:8-17`
- Modify: `genesis/orchestrator/orchestrator-strategy.mjs:81-97`

- [ ] **Step 1: Extend build-manifest.json inputs**

Open `app/elohim-library/build-manifest.json`. The `steps.build-storybook.inputs.sources` array (lines 8-17) currently lists:

```json
"sources": [
  "app/elohim-library/projects/**",
  "app/elohim-library/.storybook/**",
  "app/elohim-library/package.json",
  "app/elohim-library/tsconfig.json",
  "app/elohim-library/tsconfig.storybook.json",
  "app/elohim-library/angular.json"
],
```

Replace with:

```json
"sources": [
  "app/elohim-library/projects/**",
  "app/elohim-library/.storybook/**",
  "app/elohim-library/scripts/sync-genesis.mjs",
  "app/elohim-library/scripts/sync-genesis.test.mjs",
  "app/elohim-library/package.json",
  "app/elohim-library/tsconfig.json",
  "app/elohim-library/tsconfig.storybook.json",
  "app/elohim-library/angular.json",
  "genesis/docs/content/elohim-protocol/**",
  "genesis/graphos/**",
  "genesis/a2o/features/**"
],
```

- [ ] **Step 2: Extend orchestrator-strategy.mjs changePatterns**

Open `genesis/orchestrator/orchestrator-strategy.mjs`. Find the `'elohim-storybook':` block (line 81). The current `changePatterns` array contains:

```javascript
changePatterns: [
  'app/elohim-library/projects/**',
  'app/elohim-library/.storybook/**',
  'app/elohim-library/package.json',
  'app/elohim-library/tsconfig.storybook.json',
  'app/elohim-library/angular.json',
  'app/elohim-library/Jenkinsfile',
  'app/elohim-library/images/**',
  'genesis/orchestrator/manifests/elohim-storybook/**',
],
```

Replace with:

```javascript
changePatterns: [
  'app/elohim-library/projects/**',
  'app/elohim-library/.storybook/**',
  'app/elohim-library/scripts/**',
  'app/elohim-library/package.json',
  'app/elohim-library/tsconfig.storybook.json',
  'app/elohim-library/angular.json',
  'app/elohim-library/Jenkinsfile',
  'app/elohim-library/images/**',
  'app/elohim-library/build-manifest.json',
  'genesis/orchestrator/manifests/elohim-storybook/**',
  // graphos source content (per design-surface-narrative-scaffold spec)
  'genesis/docs/content/elohim-protocol/**',
  'genesis/graphos/**',
  'genesis/a2o/features/**',
],
```

- [ ] **Step 3: Run orchestrator-strategy tests if they exist**

```bash
cd /projects/elohim/genesis/orchestrator && node --test orchestrator-strategy.test.mjs 2>&1 | tail -20
```

Expected: passes (no behavior change beyond pattern list expansion). If a test asserts the exact `changePatterns` length or content for elohim-storybook, update it to match the new patterns.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-library/build-manifest.json genesis/orchestrator/orchestrator-strategy.mjs
git commit -m "$(cat <<'EOF'
feat(graphos): broaden elohim-storybook triggers to genesis content

build-manifest.json adds sync-genesis.mjs scripts + genesis content
paths to inputs.sources. orchestrator-strategy.mjs mirrors the same
expansion in changePatterns. A genesis edit (manifesto, vocabulary,
a2o feature) now rebuilds the storybook. Trigger fanout cost is
acceptable per spec; revisit if it becomes painful.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: End-to-end verification against success criteria

Run the full pipeline from a clean state and verify each spec success criterion is met.

**Files:** none (verification only)

- [ ] **Step 1: Clean rebuild from scratch**

```bash
cd app/elohim-library
rm -rf dist/storybook node_modules/.cache/storybook projects/graphos/src/imported/* projects/graphos/src/**/__docs__/_generated
pnpm install --frozen-lockfile
pnpm run build-storybook 2>&1 | tail -30
```

Expected: build succeeds end-to-end with no missing-source errors. Bundle exists at `dist/storybook/index.html`.

- [ ] **Step 2: Verify success criterion 1 — four-section IA populated**

Search the bundle index for representative pages from each section:

```bash
INDEX=app/elohim-library/dist/storybook/index.json
echo "Section I (Narrative Flow):"
grep -o 'I\. Why / Manifesto\|I\. What / Brand\|I\. How / Protocol Specification' "$INDEX" | sort -u
echo "Section II (Foundations):"
grep -o 'II\. Foundations / Vocabulary Register\|II\. Foundations / EPR Elements' "$INDEX" | sort -u
echo "Section III (Domains):"
grep -o 'III\. Domains / [^/]*' "$INDEX" | sort -u
echo "Section IV (Reference):"
grep -o 'IV\. Reference / [^/]*' "$INDEX" | sort -u
```

Expected: each `echo` shows multiple matches.

- [ ] **Step 3: Verify success criterion 2 — genesis edit triggers a rebuild**

Touch a genesis file (no real change), re-run, and verify the rebuild surfaces the touch. Use a known-mapped file:

```bash
touch -d '+1 second' genesis/docs/content/elohim-protocol/manifesto.md
cd app/elohim-library && pnpm run build-storybook 2>&1 | tail -5
md5sum app/elohim-library/projects/graphos/src/imported/narrative/why/manifesto.md genesis/docs/content/elohim-protocol/manifesto.md
```

Expected: both md5 sums match (sync re-copied the file). Note: the build-trigger broadening (Task 10) is verified by orchestrator/Jenkins on the next push, not locally; this step verifies the prebuild hook fires correctly at minimum.

- [ ] **Step 4: Verify success criterion 3 — domain landings show their three sub-slots**

```bash
INDEX=app/elohim-library/dist/storybook/index.json
for d in "Identity (Imagodei)" "Learning (Lamad)" "Community (Qahal)" "Economy (Shefa)" "Doorway"; do
  echo "=== $d ==="
  grep -o "III\. Domains / $d\\([^|]*\\)" "$INDEX" | sort -u | head -10
done
```

Expected: for each domain, see the landing (`III. Domains / Identity (Imagodei)`), Reference Design, Components, and (one or more) Stories entries.

- [ ] **Step 5: Verify success criterion 4 — Foundations placeholders present with markers**

```bash
INDEX=app/elohim-library/dist/storybook/index.json
for slot in "Vocabulary Register" "EPR Elements" "REA Primitives" "Brand Atoms" "Component Atoms"; do
  if grep -q "II\\. Foundations / $slot" "$INDEX"; then
    echo "FOUND: $slot"
  else
    echo "MISSING: $slot"
  fi
done
```

Expected: all 5 say FOUND.

- [ ] **Step 6: Verify success criterion 5 — sync-genesis fails loudly on missing source**

Temporarily break a mapping to confirm validation fires:

```bash
sed -i.bak "s|docs/content/elohim-protocol/manifesto.md|docs/content/elohim-protocol/MISSING.md|" app/elohim-library/scripts/sync-genesis.mjs
cd app/elohim-library && pnpm run sync-genesis; echo "Exit: $?"
mv app/elohim-library/scripts/sync-genesis.mjs.bak app/elohim-library/scripts/sync-genesis.mjs
```

Expected: non-zero exit, stderr clearly names the missing path and the IA slot it would render as.

- [ ] **Step 7: Verify success criterion 6 — only one place authors prose**

Sanity-check that the only authored prose lives in static MDX wrappers and the spec-permitted domain-landing captions. Static prose is bounded; auto-generated content dominates.

```bash
# Count generated MDX wrappers (gitignored, dynamic count)
find app/elohim-library/projects/graphos/src -path '*/_generated/*.mdx' | wc -l
# Count static MDX wrappers (committed)
find app/elohim-library/projects/graphos/src -name '*.mdx' -not -path '*/_generated/*' | wc -l
```

Expected: generated count is sizeable (one per `.feature` file in the mapped a2o dirs); static count is roughly 28 (13 from Sections I+II + 15 from Section III).

Confirm no design-narrative prose was hand-written by spot-checking a few static wrappers:

```bash
wc -l app/elohim-library/projects/graphos/src/narrative/why/__docs__/manifesto.mdx
wc -l app/elohim-library/projects/graphos/src/foundations/__docs__/vocabulary-register.mdx
```

Expected: each 5-7 lines (Meta + import + Markdown block, nothing else).

- [ ] **Step 8: Final commit (verification log only — no code changes)**

If any of Steps 2–7 surfaced a real defect, fix it (in-place edit + commit + re-verify) before proceeding. If all six criteria pass, no commit is needed for this task. Mark the task complete in the plan tracker.

If a defect is fixed in this task, commit with:

```bash
git add <fixed-files>
git commit -m "$(cat <<'EOF'
fix(graphos): <specific defect surfaced by end-to-end verification>

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-review checklist (for the implementer)

Before declaring done:

- [ ] All 11 task heads marked complete; spec success criteria 1–6 verified in Task 11
- [ ] No hand-authored design narrative beyond IA titles, slot-pending captions, and domain-landing one-liners
- [ ] `pnpm run test:sync-genesis` passes (7 tests)
- [ ] `pnpm run build-storybook` succeeds from a clean checkout
- [ ] Genesis edit → storybook rebuild verified locally (Task 11 Step 3)
- [ ] Brand title in chrome reads "graphos"
- [ ] No regression in existing `lamad-ui` stories — all five (hexagon-grid, governance-diagram, observer-diagram, value-scanner-diagram, resilience-indicator) still render

If any check fails, fix in place, recommit, and re-run the affected step before declaring done.
