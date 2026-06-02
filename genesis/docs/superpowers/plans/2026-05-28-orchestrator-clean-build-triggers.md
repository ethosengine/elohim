---
status: Draft
related:
  - 2026-05-28-sprint1-zd-substrate-correct-deploy.md   # sibling Sprint 1 Z.D deploy plan this build-trigger work supports
---

# Clean Build Triggers — Webhook-Gate Fix + Full Legacy Deprecation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `[build:*]` commit tags work on every orchestrator trigger (not just webhook), then complete the April-13 deprecation plan's Task 4 — remove the legacy `PIPELINES` algorithm entirely so build-manifests are the single source of truth.

**Architecture:** Two layers. (1) Tag-parsing lift: move `[build:*]` and `[skip ci]` parsing out of the `BUILD_TRIGGER == 'WEBHOOK'` conditional so timer/manual/replay builds honor HEAD's commit-message tags. (2) Legacy removal: move pipeline metadata (`jenkinsPath`, `manualOnly`, `triggersGenesis`, `cascades`, `dependsOn`, `deploymentCheck`) from the in-Jenkinsfile `PIPELINES` map into each project's `build-manifest.json`; delete `analyzePipelineRequirements()`, `propagateDependencies()`, `orderByDependencies()`, the comparison matrix, and the `orchestrator-strategy.mjs` mirror; rewire all external consumers (preview.mjs, count-pipeline-failures.sh, registry-cluster-drift.mjs, pipeline-trajectory.mjs, .husky/pre-push) to read pipeline metadata via a new `pipeline-registry.mjs` that loads it from manifests.

**Tech Stack:** Groovy (Jenkinsfile, build-graph.groovy), JavaScript ESM (manifest-utils.mjs, new pipeline-registry.mjs, preview.mjs, scripts/*.mjs), JSON (build-manifest.json), Bash (count-pipeline-failures.sh), Vitest.

**Reference plan:** `genesis/plans/2026-04-13-deprecate-legacy-pipelines-algorithm-plan.md` (Tasks 1-3 already landed; Task 4 partially landed — comparison still echoed and `analyzePipelineRequirements()` still called).

---

## Context

### What's already done (verify before starting)

- `sophia/build-manifest.json` exists ✓
- Build graph is primary algorithm (`applyBuildGraphRouting` picks from `graphResult.pipelineSteps`, then layers `FORCE_BUILD_PIPELINES` + genesis-auto-include) ✓
- All 8 PIPELINES entries have build-manifest.json files ✓
- `runBuildGraph` (not `Shadow`) exists at Jenkinsfile L1405 ✓

### What broke tonight (orchestrator #1077 → #1078)

- `[build:app]` was pushed in commit a412a788, expecting `elohim` pipeline to dispatch
- Webhook build #1077 was `NOT_BUILT` (aborted by milestone)
- Timer-triggered build #1078 picked up the same HEAD, ran, but **skipped tag parsing** because the tag-parsing block at Jenkinsfile L1611-1664 is gated on `env.BUILD_TRIGGER == 'WEBHOOK'`
- Result: `elohim-app` never rebuilt; measure stuck at 5/6

### Key files and their roles

| File | Role | Action |
|------|------|--------|
| `genesis/orchestrator/Jenkinsfile` | Orchestrator pipeline | Lift tag-gate (Task 1); delete legacy helpers (Tasks 6-7) |
| `genesis/orchestrator/build-graph.groovy` | Manifest-driven graph walker | Delete comparison matrix (Task 7) |
| `genesis/orchestrator/manifest-utils.mjs` | Manifest discovery + parsing | Extend with metadata helpers (Task 4) |
| `genesis/orchestrator/orchestrator-strategy.mjs` | JS mirror of `PIPELINES` | Replace with `pipeline-registry.mjs` (Task 4); delete (Task 10) |
| `genesis/orchestrator/orchestrator-strategy.test.mjs` | Tests for the JS mirror | Rewrite as manifest-driven tests (Task 10) |
| `genesis/orchestrator/preview.mjs` | `just ci-preview` CLI | Rewire to pipeline-registry (Task 8) |
| `genesis/orchestrator/scripts/registry-cluster-drift.mjs` | Health probe | Rewire (Task 8) |
| `genesis/orchestrator/scripts/pipeline-trajectory.mjs` | Failure stats | Rewire (Task 8) |
| `genesis/orchestrator/scripts/count-pipeline-failures.sh` | Bash → pipeline-list.json | Regen artifact (Task 9) |
| `genesis/orchestrator/scripts/generate-pipeline-list.mjs` | Generator for pipeline-list.json | Rewire to read manifests (Task 9) |
| `genesis/orchestrator/pipeline-list.json` | Bash-consumable artifact | Regenerated (Task 9) |
| `.husky/pre-push` | Pre-push hook | Already uses graph-walker; verify no PIPELINES dependency (Task 11) |
| `CLAUDE.md` (root) | Project docs | Update CI/CD section (Task 11) |

---

## Task 1: Lift tag-parsing out of WEBHOOK gate

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile:1610-1664`

**Why:** HEAD's commit-message tags are meaningful regardless of how the build was triggered. The current gate silently drops tags on timer/manual/replay builds.

- [ ] **Step 1: Read the current gated block**

Open `genesis/orchestrator/Jenkinsfile` lines 1610-1664. The block is:

```groovy
if (env.BUILD_TRIGGER == 'WEBHOOK') {
    def commitMsg = sh(script: 'git log -1 --format=%B', returnStdout: true).trim()
    if (commitMsg =~ /(?i)\[(skip ci|ci skip|no ci)\]/) {
        echo "⏭️  [skip ci] detected in commit message — skipping all pipelines"
        env.SKIP_CI = 'true'
    }
    // ... [build:*] tag parsing ...
    if (commitMsg =~ /(?i)\[deploy[-_ ]only\]/) {
        echo "🚀 [deploy-only] detected ..."
        env.DEPLOY_ONLY_FROM_TAG = 'true'
    }
}
```

- [ ] **Step 2: Restructure — move tag reading out, keep [deploy-only] gated**

Replace lines 1610-1664 with:

```groovy
// Read HEAD commit message once. Used for [skip ci], [build:*], and
// (webhook-only) [deploy-only] parsing.
def commitMsg = sh(script: 'git log -1 --format=%B', returnStdout: true).trim()

// [skip ci] — meaningful on every trigger type. Operator who tagged a commit
// with [skip ci] means "do not run CI on this HEAD regardless of how this
// build was kicked off" (timer, replay, webhook).
if (commitMsg =~ /(?i)\[(skip ci|ci skip|no ci)\]/) {
    echo "⏭️  [skip ci] detected in commit message — skipping all pipelines"
    env.SKIP_CI = 'true'
}

// [build:*] tags — meaningful on every trigger type. The tag lives in
// git history; the orchestrator MUST honor it whenever it reads HEAD,
// not only on the first webhook delivery (which may be NOT_BUILT due
// to milestone() supersede — see plan 2026-05-28).
def buildTags = []
def buildTagAliases = [
    'edge': 'elohim-edge',
    'dna': 'elohim-holochain',
    'app': 'elohim',
    'genesis': 'elohim-genesis',
    'sophia': 'elohim-sophia',
    'steward': 'elohim-steward'
]
def tagMatcher = (commitMsg =~ /(?i)\[build:([a-z,-]+)\]/)
tagMatcher.each { match ->
    match[1].split(',').each { tag ->
        tag = tag.trim().toLowerCase()
        if (tag == 'all') {
            buildTags.addAll(PIPELINES.keySet().findAll { !PIPELINES[it].manualOnly })
        } else if (buildTagAliases[tag]) {
            buildTags.add(buildTagAliases[tag])
        } else {
            echo "⚠️  Unknown [build:${tag}] — valid: ${buildTagAliases.keySet().join(', ')}, all"
        }
    }
}
if (buildTags) {
    env.FORCE_BUILD_PIPELINES = buildTags.unique().join(',')
    echo "🔧 [build:*] tags detected — force-including: ${env.FORCE_BUILD_PIPELINES}"
}

// [deploy-only] — DELIBERATELY webhook-gated. This tag tells the orchestrator
// to bypass changeset analysis and redeploy existing Harbor tags. It is only
// meaningful for fresh pushes that intend a deployment; timer/replay reusing
// the same HEAD must not silently retrigger a deploy.
if (env.BUILD_TRIGGER == 'WEBHOOK') {
    if (commitMsg =~ /(?i)\[deploy[-_ ]only\]/) {
        echo "🚀 [deploy-only] detected in commit message — DEPLOY_ONLY mode (skip builds, redeploy from env-file tags)"
        env.DEPLOY_ONLY_FROM_TAG = 'true'
    }
}
```

- [ ] **Step 3: Verify the milestone(2) call still fires unconditionally**

Confirm `milestone(ordinal: 2, label: 'Checkout Complete')` at line ~1667 is OUTSIDE the (now-deleted) `if (BUILD_TRIGGER == 'WEBHOOK')` block. It must still run for every trigger type to prevent runaway old builds.

- [ ] **Step 4: Add a CPS-scope sanity test**

Open `genesis/orchestrator/jenkinsfile-cps-scope.test.mjs`. Find the assertion that scopes the tag-parsing block. Update it to verify the block now lives outside the WEBHOOK conditional.

If no such assertion exists, add this test at the end of the file:

```javascript
test('[build:*] tag parsing is NOT gated on BUILD_TRIGGER == WEBHOOK', () => {
  const jf = readFileSync(
    resolve(__dirname, 'Jenkinsfile'),
    'utf8'
  );
  // Find the tag-aliases declaration; walk back to find its enclosing
  // conditional; assert it is NOT the WEBHOOK gate.
  const tagAliasIdx = jf.indexOf("def buildTagAliases");
  assert(tagAliasIdx > 0, 'buildTagAliases declaration not found');
  const slice = jf.slice(Math.max(0, tagAliasIdx - 800), tagAliasIdx);
  assert(
    !/if \(env\.BUILD_TRIGGER == 'WEBHOOK'\) \{[^}]*$/s.test(slice),
    '[build:*] tag parsing must not be inside the WEBHOOK conditional'
  );
});

test('[deploy-only] parsing IS still gated on WEBHOOK', () => {
  const jf = readFileSync(
    resolve(__dirname, 'Jenkinsfile'),
    'utf8'
  );
  const deployIdx = jf.indexOf("DEPLOY_ONLY_FROM_TAG = 'true'");
  assert(deployIdx > 0, 'DEPLOY_ONLY_FROM_TAG assignment not found');
  const slice = jf.slice(Math.max(0, deployIdx - 400), deployIdx);
  assert(
    /if \(env\.BUILD_TRIGGER == 'WEBHOOK'\) \{/s.test(slice),
    '[deploy-only] must remain webhook-gated'
  );
});
```

- [ ] **Step 5: Run the CPS-scope tests**

Run: `cd genesis/orchestrator && pnpm exec vitest run jenkinsfile-cps-scope.test.mjs`

Expected: All tests pass, including the two new assertions.

- [ ] **Step 6: Commit**

```bash
git add genesis/orchestrator/Jenkinsfile genesis/orchestrator/jenkinsfile-cps-scope.test.mjs
git commit -m "$(cat <<'EOF'
fix(orchestrator): lift [build:*] tag parsing out of WEBHOOK gate

[build:*] commit tags now honored on every trigger type (webhook, timer,
manual, replay) — HEAD's commit message is the source of truth.
[deploy-only] remains webhook-gated by design.

Discovered during 2026-05-28 shift when orchestrator #1077 (webhook,
NOT_BUILT) was superseded by #1078 (timer) for commit a412a788,
silently dropping the [build:app] tag and stalling the cross-pillar
sprint at 5/6 pipelines green.
EOF
)"
```

---

## Task 2: Define manifest schema extensions

**Files:**
- Modify: `genesis/orchestrator/manifest.schema.json`

**Why:** Before moving metadata into manifests, the schema must accept the new top-level fields. Pipeline-level metadata (`jenkinsPath`, `manualOnly`, `triggersGenesis`, `cascades`, `dependsOn`) needs to live alongside `pipeline`, `steps`, `gate`, `deployment`.

- [ ] **Step 1: Read the current schema**

Run: `cat /projects/elohim/genesis/orchestrator/manifest.schema.json`

Identify the top-level `properties` object and its `required` list.

- [ ] **Step 2: Add new optional pipeline-metadata properties**

Edit `manifest.schema.json` to add these properties under top-level `properties`:

```json
"jenkinsPath": {
  "type": "string",
  "description": "Path to the pipeline's Jenkinsfile (relative to repo root). Required for dispatchable pipelines; absent for graph-only pipelines that are never directly dispatched by the orchestrator."
},
"manualOnly": {
  "type": "boolean",
  "default": false,
  "description": "If true, this pipeline is never auto-dispatched on changeset; it can only be triggered manually (or via [build:*] tag)."
},
"triggersGenesis": {
  "type": "boolean",
  "default": false,
  "description": "If true and this pipeline builds, elohim-genesis is auto-included on dev branches."
},
"cascades": {
  "type": "boolean",
  "default": true,
  "description": "If true, dependents of this pipeline auto-build when it builds. Set false for leaf pipelines that produce artifacts consumed only by themselves."
},
"dependsOn": {
  "type": "array",
  "items": { "type": "string" },
  "default": [],
  "description": "Pipeline names this pipeline depends on for execution ordering."
}
```

Do NOT add any of these to `required` — they default to safe values, and graph-only pipelines (no Jenkins job) legitimately have none of them.

- [ ] **Step 3: Run schema validator on existing manifests**

Run: `cd genesis/orchestrator && node validate-manifests.mjs`

Expected: PASS (existing manifests don't have the new fields yet, but the fields are optional).

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/manifest.schema.json
git commit -m "feat(orchestrator): extend build-manifest schema with pipeline-level metadata fields

Adds optional jenkinsPath, manualOnly, triggersGenesis, cascades, and
dependsOn properties at the top level of build-manifest.json. These
will replace the in-Jenkinsfile PIPELINES map in subsequent commits."
```

---

## Task 3: Migrate pipeline metadata into manifests

**Files:**
- Modify: `app/elohim-app/build-manifest.json` (pipeline: `elohim`)
- Modify: `app/elohim-library/build-manifest.json` (pipeline: `elohim-storybook`)
- Modify: `doorway/doorway-app/build-manifest.json` (pipeline: `elohim-doorway-app`)
- Modify: `elohim/elohim-compute/build-manifest.json` (pipeline: `elohim-compute`)
- Modify: `elohim/epr/build-manifest.json` (pipeline: `elohim-epr`)
- Modify: `elohim/holochain/build-manifest.json` (pipeline: `elohim-edge`)
- Modify: `elohim/holochain/dna/build-manifest.json` (pipeline: `elohim-holochain`)
- Modify: `genesis/build-manifest.json` (pipeline: `elohim-genesis`)
- Modify: `genesis/orchestrator/build-manifest.json` (pipeline: `elohim-orchestrator`)
- Modify: `sophia/build-manifest.json` (pipeline: `elohim-sophia`)
- Modify: `steward/device/build-manifest.json` (pipeline: `elohim-steward`)

**Why:** Each manifest absorbs the per-pipeline fields currently declared in `Jenkinsfile.PIPELINES` and `orchestrator-strategy.mjs.PIPELINES`. After this task, the two PIPELINES maps are redundant; they get deleted in Task 7.

Use this reference table — derived from `genesis/orchestrator/Jenkinsfile:39` (PIPELINES) and L1462-L1700 (deployment health endpoints). Pipelines marked "(graph-only)" have no Jenkins job — they need no `jenkinsPath`.

| Pipeline | jenkinsPath | manualOnly | triggersGenesis | cascades | dependsOn |
|---|---|---|---|---|---|
| elohim-holochain | `elohim/holochain/dna/Jenkinsfile` | false | true | true | [] |
| elohim-edge | `elohim/holochain/Jenkinsfile` | false | true | (default true) | ["elohim-holochain"] |
| elohim | `Jenkinsfile` | false | true | (default true) | ["elohim-sophia"] |
| elohim-genesis | `genesis/Jenkinsfile` | false | false | (default true) | ["elohim-edge", "elohim"] |
| elohim-steward | `steward/device/Jenkinsfile` | true | false | (default true) | ["elohim-holochain"] |
| elohim-sophia | `sophia.Jenkinsfile` | false | false | true | [] |
| elohim-epr | `elohim/epr/Jenkinsfile` | false | false | false | [] |
| elohim-storybook | `app/elohim-library/Jenkinsfile` | false | false | false | [] |
| elohim-doorway-app | (graph-only) | — | — | — | — |
| elohim-compute | (graph-only) | — | — | — | — |
| elohim-orchestrator | `genesis/orchestrator/Jenkinsfile` | false | false | false | [] |

- [ ] **Step 1: Update elohim-holochain manifest**

Edit `elohim/holochain/dna/build-manifest.json`. Add these top-level fields (alongside `manifestVersion`, `pipeline`, `steps`, etc.):

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-holochain",
  "jenkinsPath": "elohim/holochain/dna/Jenkinsfile",
  "manualOnly": false,
  "triggersGenesis": true,
  "cascades": true,
  "dependsOn": [],
  ...existing fields...
}
```

- [ ] **Step 2: Update elohim-edge manifest**

Edit `elohim/holochain/build-manifest.json`. Add:

```json
"jenkinsPath": "elohim/holochain/Jenkinsfile",
"manualOnly": false,
"triggersGenesis": true,
"dependsOn": ["elohim-holochain"]
```

(`cascades` omitted — default `true` is correct.)

- [ ] **Step 3: Update elohim manifest**

Edit `app/elohim-app/build-manifest.json`. Add:

```json
"jenkinsPath": "Jenkinsfile",
"manualOnly": false,
"triggersGenesis": true,
"dependsOn": ["elohim-sophia"]
```

- [ ] **Step 4: Update elohim-genesis manifest**

Edit `genesis/build-manifest.json`. Add:

```json
"jenkinsPath": "genesis/Jenkinsfile",
"manualOnly": false,
"triggersGenesis": false,
"dependsOn": ["elohim-edge", "elohim"]
```

- [ ] **Step 5: Update elohim-steward manifest**

Edit `steward/device/build-manifest.json`. Add:

```json
"jenkinsPath": "steward/device/Jenkinsfile",
"manualOnly": true,
"triggersGenesis": false,
"dependsOn": ["elohim-holochain"]
```

- [ ] **Step 6: Update elohim-sophia manifest**

Edit `sophia/build-manifest.json`. Add:

```json
"jenkinsPath": "sophia.Jenkinsfile",
"manualOnly": false,
"triggersGenesis": false,
"cascades": true,
"dependsOn": []
```

- [ ] **Step 7: Update elohim-epr manifest**

Edit `elohim/epr/build-manifest.json`. Add:

```json
"jenkinsPath": "elohim/epr/Jenkinsfile",
"manualOnly": false,
"triggersGenesis": false,
"cascades": false,
"dependsOn": []
```

- [ ] **Step 8: Update elohim-storybook manifest**

Edit `app/elohim-library/build-manifest.json`. Add:

```json
"jenkinsPath": "app/elohim-library/Jenkinsfile",
"manualOnly": false,
"triggersGenesis": false,
"cascades": false,
"dependsOn": []
```

- [ ] **Step 9: Update elohim-orchestrator manifest**

Edit `genesis/orchestrator/build-manifest.json`. Add:

```json
"jenkinsPath": "genesis/orchestrator/Jenkinsfile",
"manualOnly": false,
"triggersGenesis": false,
"cascades": false,
"dependsOn": []
```

- [ ] **Step 10: Leave graph-only manifests alone**

`doorway/doorway-app/build-manifest.json` (elohim-doorway-app) and `elohim/elohim-compute/build-manifest.json` (elohim-compute) have no Jenkins job. They get no `jenkinsPath` — the registry treats absence-of-jenkinsPath as "graph-only, never dispatched."

- [ ] **Step 11: Validate all manifests**

Run: `cd genesis/orchestrator && node validate-manifests.mjs`

Expected: PASS for all manifests. Schema validates the new fields' types.

- [ ] **Step 12: Run schema codegen (if applicable)**

If `pnpm run schema:codegen:ts` regenerates a TypeScript view of build-manifest.json anywhere, run it now and stage the result. Most likely no-op — these manifests are not in the DNA schema set.

- [ ] **Step 13: Commit**

```bash
git add app/elohim-app/build-manifest.json app/elohim-library/build-manifest.json elohim/epr/build-manifest.json elohim/holochain/build-manifest.json elohim/holochain/dna/build-manifest.json genesis/build-manifest.json genesis/orchestrator/build-manifest.json sophia/build-manifest.json steward/device/build-manifest.json
git commit -m "feat(orchestrator): move pipeline metadata into build-manifest.json files

Each manifest now carries jenkinsPath, manualOnly, triggersGenesis,
cascades, and dependsOn — the fields that previously lived only in
Jenkinsfile.PIPELINES and orchestrator-strategy.mjs.PIPELINES.

Sets up Task 7's removal of the two redundant PIPELINES maps."
```

---

## Task 4: Build the new pipeline-registry helper

**Files:**
- Create: `genesis/orchestrator/pipeline-registry.mjs`
- Create: `genesis/orchestrator/pipeline-registry.test.mjs`

**Why:** External consumers (preview.mjs, scripts/*, drift tests) currently import `PIPELINES` from `orchestrator-strategy.mjs`. They need a manifest-backed replacement before the strategy module can be deleted.

- [ ] **Step 1: Write the failing test**

Create `genesis/orchestrator/pipeline-registry.test.mjs`:

```javascript
import { test, describe } from 'node:test';
import { strict as assert } from 'node:assert';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import {
  loadPipelineRegistry,
  nonManualPipelines,
  dispatchablePipelines,
  pipelinesThatTriggerGenesis,
  pipelineDependencyMap,
} from './pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../..');

describe('pipeline-registry', () => {
  const registry = loadPipelineRegistry(ROOT);

  test('loads every build-manifest.json with a pipeline field', () => {
    assert.ok(registry.size >= 8, `expected ≥8 pipelines, got ${registry.size}`);
    for (const known of ['elohim', 'elohim-edge', 'elohim-holochain', 'elohim-genesis', 'elohim-sophia']) {
      assert.ok(registry.has(known), `missing pipeline: ${known}`);
    }
  });

  test('nonManualPipelines excludes manualOnly entries', () => {
    const names = nonManualPipelines(registry);
    assert.ok(!names.includes('elohim-steward'), 'elohim-steward should be excluded');
    assert.ok(names.includes('elohim'), 'elohim should be included');
  });

  test('dispatchablePipelines returns only entries with jenkinsPath', () => {
    const names = dispatchablePipelines(registry);
    assert.ok(!names.includes('elohim-doorway-app'), 'graph-only pipeline should be excluded');
    assert.ok(!names.includes('elohim-compute'), 'graph-only pipeline should be excluded');
    assert.ok(names.includes('elohim'), 'elohim has jenkinsPath');
  });

  test('pipelinesThatTriggerGenesis returns the marked set', () => {
    const names = pipelinesThatTriggerGenesis(registry);
    assert.deepStrictEqual(
      [...names].sort(),
      ['elohim', 'elohim-edge', 'elohim-holochain'].sort()
    );
  });

  test('pipelineDependencyMap produces a name → deps map', () => {
    const deps = pipelineDependencyMap(registry);
    assert.deepStrictEqual(deps.get('elohim-edge'), ['elohim-holochain']);
    assert.deepStrictEqual(deps.get('elohim-genesis'), ['elohim-edge', 'elohim']);
    assert.deepStrictEqual(deps.get('elohim-sophia'), []);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd genesis/orchestrator && pnpm exec vitest run pipeline-registry.test.mjs`

Expected: FAIL with `Cannot find module './pipeline-registry.mjs'`.

- [ ] **Step 3: Write the minimal implementation**

Create `genesis/orchestrator/pipeline-registry.mjs`:

```javascript
/**
 * Pipeline registry — single source of truth for pipeline-level metadata.
 *
 * Loads every build-manifest.json in the workspace and exposes the
 * pipeline-level fields (jenkinsPath, manualOnly, triggersGenesis,
 * cascades, dependsOn) that previously lived in Jenkinsfile.PIPELINES
 * and orchestrator-strategy.mjs.PIPELINES.
 *
 * Replaces orchestrator-strategy.mjs as of plan
 * 2026-05-28-orchestrator-clean-build-triggers.
 */

import { loadManifests } from './manifest-utils.mjs';

/**
 * @returns {Map<string, {pipeline: string, jenkinsPath?: string,
 *   manualOnly: boolean, triggersGenesis: boolean, cascades: boolean,
 *   dependsOn: string[], manifestPath: string}>}
 */
export function loadPipelineRegistry(rootDir) {
  const manifests = loadManifests(rootDir);
  const registry = new Map();

  for (const { path, content } of manifests) {
    if (!content.pipeline) continue;
    if (registry.has(content.pipeline)) {
      throw new Error(
        `Duplicate pipeline name '${content.pipeline}' in ${path} and ${registry.get(content.pipeline).manifestPath}`
      );
    }
    registry.set(content.pipeline, {
      pipeline: content.pipeline,
      jenkinsPath: content.jenkinsPath,
      manualOnly: content.manualOnly === true,
      triggersGenesis: content.triggersGenesis === true,
      cascades: content.cascades === undefined ? true : content.cascades === true,
      dependsOn: Array.isArray(content.dependsOn) ? content.dependsOn : [],
      manifestPath: path,
    });
  }

  return registry;
}

export function nonManualPipelines(registry) {
  return [...registry.values()]
    .filter(p => !p.manualOnly)
    .map(p => p.pipeline);
}

export function dispatchablePipelines(registry) {
  return [...registry.values()]
    .filter(p => typeof p.jenkinsPath === 'string' && p.jenkinsPath.length > 0)
    .map(p => p.pipeline);
}

export function pipelinesThatTriggerGenesis(registry) {
  return [...registry.values()]
    .filter(p => p.triggersGenesis)
    .map(p => p.pipeline);
}

export function pipelineDependencyMap(registry) {
  const map = new Map();
  for (const p of registry.values()) {
    map.set(p.pipeline, p.dependsOn);
  }
  return map;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd genesis/orchestrator && pnpm exec vitest run pipeline-registry.test.mjs`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/pipeline-registry.mjs genesis/orchestrator/pipeline-registry.test.mjs
git commit -m "feat(orchestrator): add pipeline-registry.mjs — manifest-backed pipeline metadata

Replaces the in-memory PIPELINES map in orchestrator-strategy.mjs.
External consumers (preview.mjs, scripts/*, drift tests) migrate to
this module in subsequent commits."
```

---

## Task 5: Surface pipeline metadata in build-graph.groovy

**Files:**
- Modify: `genesis/orchestrator/build-graph.groovy:60-110` (composeGraph)
- Modify: `genesis/orchestrator/build-graph.groovy:763-834` (walkBuildGraph)

**Why:** The orchestrator's Jenkinsfile reads `PIPELINES[name].jenkinsPath`, `.manualOnly`, `.triggersGenesis`, `.dependsOn` from the in-file map. Once that map is deleted (Task 7), the Jenkinsfile must read those fields from the build-graph result. `build-graph.groovy` already loads manifests — it just needs to carry the new fields through.

- [ ] **Step 1: Extend composeGraph to carry pipeline metadata**

In `build-graph.groovy`, the `composeGraph(List manifests)` function populates `graph.pipelines[pipeline] = manifest`. The whole manifest is stored, so `jenkinsPath`, `manualOnly`, etc. are already accessible via `graph.pipelines['elohim'].jenkinsPath`. Verify this by reading the function. No code change expected.

- [ ] **Step 2: Add a registry accessor in walkBuildGraph's result**

Find `walkBuildGraph(...)` and its return statement. Add a `pipelineRegistry` field that exposes the metadata as a simple Map<String, Map>:

```groovy
def pipelineRegistry = [:]
graph.pipelines.each { name, manifest ->
    pipelineRegistry[name] = [
        jenkinsPath: manifest.jenkinsPath,
        manualOnly: manifest.manualOnly == true,
        triggersGenesis: manifest.triggersGenesis == true,
        cascades: manifest.cascades == null ? true : (manifest.cascades == true),
        dependsOn: manifest.dependsOn ?: [],
    ]
}

return [
    graph: graph,
    staleMap: staleMap,
    pipelineSteps: pipelineSteps,
    buildProcessHashes: buildProcessHashes,
    previousState: previousState,
    pipelineRegistry: pipelineRegistry,  // NEW
]
```

- [ ] **Step 3: Verify a Groovy linter or test exists**

Run: `cd genesis/orchestrator && pnpm exec vitest run jenkinsfile-cps-scope.test.mjs`

Expected: PASS. The CPS-scope tests do not deeply parse Groovy semantics, but they verify the file still compiles as Jenkins-pipeline Groovy.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/build-graph.groovy
git commit -m "feat(orchestrator): surface pipelineRegistry from walkBuildGraph result

Exposes per-pipeline metadata (jenkinsPath, manualOnly, triggersGenesis,
cascades, dependsOn) from the parsed manifests. The Jenkinsfile will
consume this in Task 6 to replace its in-file PIPELINES map."
```

---

## Task 6: Make Jenkinsfile read metadata from manifests

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile:39-150` (PIPELINES map)
- Modify: `genesis/orchestrator/Jenkinsfile:176-200` (getHealthEndpoints)
- Modify: `genesis/orchestrator/Jenkinsfile:790-870` (triggerPipeline)
- Modify: `genesis/orchestrator/Jenkinsfile:893-1000` (autoModeAnalyze)
- Modify: `genesis/orchestrator/Jenkinsfile:1009-1042` (applyBuildGraphRouting)
- Modify: `genesis/orchestrator/Jenkinsfile:1462-1480` (MODE choices include 'rebuild-doorway-app' etc.)

**Why:** Every read of `PIPELINES[name].X` becomes a read of `env.PIPELINE_REGISTRY_JSON` (parsed once after `runBuildGraph` lands). Once Task 7 deletes the map, every reference must already be migrated.

- [ ] **Step 1: Stash the pipelineRegistry from the build graph result**

In `runBuildGraph(...)` at Jenkinsfile L1405, after `def result = buildGraph.walkBuildGraph(changedFiles)`, add:

```groovy
// Stash registry as JSON env var so it's available to all later stages.
// Read from build-manifest.json files at composeGraph time; replaces the
// in-Jenkinsfile PIPELINES map.
if (result.pipelineRegistry) {
    env.PIPELINE_REGISTRY_JSON = writeJSON(returnText: true, json: result.pipelineRegistry)
}
```

- [ ] **Step 2: Add a getPipelineMetadata helper near the top of Jenkinsfile**

After the existing `@Field def PIPELINES = [...]` block (line 39), add:

```groovy
/**
 * Look up pipeline metadata from the manifest-derived registry.
 * Available after runBuildGraph() runs (Determine Build Plan stage).
 * For Pre-flight + early stages that need metadata, falls back to
 * parsing manifests directly.
 */
def getPipelineMetadata(String name) {
    if (env.PIPELINE_REGISTRY_JSON) {
        def registry = readJSON(text: env.PIPELINE_REGISTRY_JSON)
        return registry[name] ?: [:]
    }
    // Pre-build-graph fallback: parse manifests directly.
    def manifestPath = sh(
        script: "find . -name 'build-manifest.json' -not -path '*/node_modules/*' | xargs grep -l '\"pipeline\": \"${name}\"' | head -1",
        returnStdout: true
    ).trim()
    if (!manifestPath) return [:]
    def m = readJSON(file: manifestPath)
    return [
        jenkinsPath: m.jenkinsPath,
        manualOnly: m.manualOnly == true,
        triggersGenesis: m.triggersGenesis == true,
        cascades: m.cascades == null ? true : (m.cascades == true),
        dependsOn: m.dependsOn ?: [],
    ]
}
```

- [ ] **Step 3: Migrate getHealthEndpoints**

At line 176, replace the PIPELINES.each iteration with a manifest iteration. The current code reads `config.deploymentCheck`; verify which manifests carry `deployment.targets.alpha.healthCheck` and which need a new field. Update to:

```groovy
def getHealthEndpoints() {
    def endpoints = [:]
    // Read deployment health checks from build-manifest.json files.
    def manifestPaths = sh(
        script: "find . -name 'build-manifest.json' -not -path '*/node_modules/*'",
        returnStdout: true
    ).trim().split('\n').findAll { it }
    manifestPaths.each { path ->
        def m = readJSON(file: path)
        def alphaCheck = m.deployment?.targets?.alpha?.healthCheck
        if (m.pipeline && alphaCheck) {
            endpoints[m.pipeline] = alphaCheck
        }
    }
    return endpoints
}
```

- [ ] **Step 4: Migrate triggerPipeline at L794**

The current code reads `def config = PIPELINES[name]` then uses `config.jenkinsPath`. Change to:

```groovy
def config = getPipelineMetadata(name)
if (!config.jenkinsPath) {
    error "Cannot dispatch pipeline '${name}': no jenkinsPath in its build-manifest.json"
}
```

- [ ] **Step 5: Migrate autoModeAnalyze**

In `autoModeAnalyze()` at line 893, all references to `PIPELINES[name]` become `getPipelineMetadata(name)`. Notably:
- Line 931: `if (!pipelines.contains(name) && PIPELINES[name])` → `if (!pipelines.contains(name) && getPipelineMetadata(name).jenkinsPath)`
- Line 944: `pipelines.find { PIPELINES[it]?.triggersGenesis }` → `pipelines.find { getPipelineMetadata(it).triggersGenesis }`

Delete the call to `analyzePipelineRequirements()` at line 907. The pipelines list now comes entirely from the build graph result (set by applyBuildGraphRouting). Replace lines 907-926 with:

```groovy
// Pipelines list now comes from the build graph (applied in applyBuildGraphRouting).
// autoModeAnalyze's job here is reduced to: load baselines (advisory),
// surface the changed-file count for the decision-matrix log, and emit
// the per-area breakdown.
echo "📁 Changed Files (${changedFiles.size()} total):"
def filesByArea = [:]
changedFiles.each { file ->
    def area = file.split('/')[0]
    if (!filesByArea[area]) filesByArea[area] = []
    filesByArea[area].add(file)
}
filesByArea.each { area, files ->
    echo "   ${area}/ (${files.size()} files)"
    files.take(3).each { echo "      └─ ${it}" }
    if (files.size() > 3) echo "      └─ ... and ${files.size() - 3} more"
}

def pipelines = []  // Empty initial set; applyBuildGraphRouting populates from graph.
```

Remove the `analysis` variable entirely from the return — downstream code reads metadata via `getPipelineMetadata()` now. Update `return [pipelines: pipelines, analysis: analysis, ...]` to `return [pipelines: pipelines, changedFiles: changedFiles]`.

- [ ] **Step 6: Migrate applyBuildGraphRouting**

In `applyBuildGraphRouting()` at line 1009, change the signature to drop `analysis` (no longer computed):

```groovy
def applyBuildGraphRouting(String mode, changedFiles, fallbackPipelines) {
    def graphPipelines
    if (mode in ['auto', 'status']) {
        echo '\n=== Build Graph (Primary) ==='
        def graphResult = runBuildGraph(changedFiles)  // dropped analysis arg
        graphPipelines = graphResult.pipelineSteps.keySet().toList()

        // Filter to dispatchable pipelines (have jenkinsPath in their manifest).
        graphPipelines.removeAll { name ->
            !getPipelineMetadata(name).jenkinsPath
        }

        // Apply [build:*] commit-message overrides.
        if (env.FORCE_BUILD_PIPELINES) {
            env.FORCE_BUILD_PIPELINES.split(',').each { name ->
                def meta = getPipelineMetadata(name)
                if (!graphPipelines.contains(name) && meta.jenkinsPath) {
                    graphPipelines.add(name)
                    echo "🔧 [build:*] force-include applied to graph result: ${name}"
                }
            }
        }

        // Genesis auto-include on dev branches.
        if (!graphPipelines.contains('elohim-genesis') && !params.SKIP_GENESIS) {
            def isDevBranch = env.BRANCH_NAME == 'dev' || env.BRANCH_NAME ==~ /dev-.+|feat-.+|claude\/.+/
            def triggeringPipeline = graphPipelines.find { getPipelineMetadata(it).triggersGenesis }
            if (triggeringPipeline && isDevBranch) {
                graphPipelines.add('elohim-genesis')
            }
        }

        // Apply manualOnly filter (in case [build:*] tried to add a manual-only pipeline).
        graphPipelines.removeAll { name -> getPipelineMetadata(name).manualOnly }
    } else {
        graphPipelines = fallbackPipelines
    }
    return graphPipelines
}
```

Update the caller in Determine Build Plan to drop the `analysis` arg.

- [ ] **Step 7: Migrate manualModeAnalyze**

At line 963, `manualModeAnalyze(String mode)` builds an `analysis` Map by iterating `PIPELINES`. The whole `analysis` Map is dead weight — autoModeAnalyze no longer needs it, neither does applyBuildGraphRouting. Simplify to:

```groovy
def manualModeAnalyze(String mode) {
    echo "📋 Using manual build mode: ${mode}"
    def pipelines = []

    switch (mode) {
        case 'rebuild-all':
            pipelines = ['elohim-holochain', 'elohim-edge', 'elohim']
            if (!params.SKIP_GENESIS) pipelines.add('elohim-genesis')
            break
        case 'rebuild-edge':
            pipelines = ['elohim-edge']
            if (!params.SKIP_GENESIS) pipelines.add('elohim-genesis')
            break
        case 'rebuild-app':
            pipelines = ['elohim']
            if (!params.SKIP_GENESIS) pipelines.add('elohim-genesis')
            break
        case 'genesis-only':
            pipelines = ['elohim-genesis']
            break
    }

    return [pipelines: pipelines]
}
```

- [ ] **Step 8: Run CPS-scope test + manifest validation**

Run: `cd genesis/orchestrator && pnpm exec vitest run jenkinsfile-cps-scope.test.mjs && node validate-manifests.mjs`

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add genesis/orchestrator/Jenkinsfile
git commit -m "refactor(orchestrator): read pipeline metadata from manifests via getPipelineMetadata()

Replaces in-Jenkinsfile PIPELINES[name].X reads with manifest lookups.
The in-file PIPELINES map is still present but unused except by helper
fallbacks; Task 7 deletes it."
```

---

## Task 7: Delete legacy algorithm code

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile` — remove ~400 lines
- Modify: `genesis/orchestrator/build-graph.groovy:571-668` (comparison matrix)

**Why:** With Task 6 landed, nothing in the Jenkinsfile reads from the in-file `PIPELINES` map or calls the legacy functions. They're dead code.

- [ ] **Step 1: Delete PIPELINES map and legacy functions from Jenkinsfile**

Delete these blocks in `genesis/orchestrator/Jenkinsfile`:

1. `@Field def PIPELINES = [...]` (lines 39-150 — entire map literal)
2. `def analyzeChangeset(...)` (lines 383-440)
3. `def analyzePipelineRequirements(...)` (lines 444-580)
4. `def orderByDependencies(...)` (lines 586-608)
5. `def groupByDependencyLevel(...)` (lines 615-632) — REPLACE with a version that reads dependsOn from `getPipelineMetadata`:

```groovy
def groupByDependencyLevel(pipelineList) {
    def nonGenesis = pipelineList.findAll { it != 'elohim-genesis' }
    def levels = []
    def placed = [] as Set

    while (placed.size() < nonGenesis.size()) {
        def currentLevel = nonGenesis.findAll { name ->
            if (placed.contains(name)) return false
            def deps = getPipelineMetadata(name).dependsOn ?: []
            deps.every { dep -> !nonGenesis.contains(dep) || placed.contains(dep) }
        }
        if (currentLevel.isEmpty()) break
        levels.add(currentLevel)
        placed.addAll(currentLevel)
    }

    return levels
}
```

6. `def propagateDependencies(...)` (lines 640-664) — REPLACE with manifest-backed version:

```groovy
def propagateDependencies(pipelines) {
    def added = true
    while (added) {
        added = false
        // Iterate every pipeline in the registry.
        def allNames = []
        if (env.PIPELINE_REGISTRY_JSON) {
            def registry = readJSON(text: env.PIPELINE_REGISTRY_JSON)
            allNames = registry.keySet().toList()
        }
        allNames.each { name ->
            if (pipelines.contains(name)) return
            def meta = getPipelineMetadata(name)
            if (meta.manualOnly) return

            def deps = meta.dependsOn ?: []
            def buildingDep = deps.find { dep ->
                if (!pipelines.contains(dep)) return false
                def depMeta = getPipelineMetadata(dep)
                return depMeta.cascades == null ? true : depMeta.cascades
            }
            if (buildingDep) {
                pipelines.add(name)
                added = true
            }
        }
    }
    return pipelines
}
```

Note that `propagateDependencies` no longer takes `analysis` — the caller signature changes. Update the lone caller (applyBuildGraphRouting or wherever it lives now) to drop the second arg.

- [ ] **Step 2: Delete the comparison matrix from build-graph.groovy**

In `build-graph.groovy`, delete:
- `getKnownDivergences()` (lines ~571-600)
- `formatComparisonMatrix()` (lines ~610-668)

In `runBuildGraph()` (Jenkinsfile L1405-L1429), delete the `comparison = buildGraph.formatComparisonMatrix(...)` lines and the divergence warning echo. The function simplifies to:

```groovy
def runBuildGraph(List changedFiles) {
    def buildGraph = load('genesis/orchestrator/build-graph.groovy')
    def result = buildGraph.walkBuildGraph(changedFiles)

    echo buildGraph.formatPerFileMatrix(result.graph, result.staleMap, changedFiles)

    def previousCommit = result.previousState?.lastSuccessfulCommit ?: null
    buildGraph.saveBuildState(result.graph, result.staleMap, result.buildProcessHashes, previousCommit, result.previousState)

    if (result.pipelineRegistry) {
        env.PIPELINE_REGISTRY_JSON = writeJSON(returnText: true, json: result.pipelineRegistry)
    }

    return result
}
```

(Note dropped second arg — `pipelinesAnalysis` is gone.)

- [ ] **Step 3: Simplify loadPipelineBaselines to global-only**

The per-pipeline baseline cache existed to support `analyzePipelineRequirements`'s per-pipeline diff. With that function gone, only `__global__` is needed for `analyzeChangeset` — but `analyzeChangeset` is also gone. Find every caller of `loadPipelineBaselines()`; if none remain, delete the function entirely. Same for `archivePipelineBaselines` per-pipeline keys.

Actually verify: `analyzeChangeset` produces the changedFiles list. Something still needs to do that. The build-graph already takes `changedFiles` as input — find who computes it now. If `analyzeChangeset` is still the producer, keep it but rip out the per-pipeline cache:

```groovy
def analyzeChangeset(storedGlobalBaseline = null) {
    def baseCommit = params.FORCE_COMMIT?.trim() ?: storedGlobalBaseline ?: env.GIT_PREVIOUS_SUCCESSFUL_COMMIT ?: env.GIT_PREVIOUS_COMMIT
    if (!baseCommit) {
        echo "No baseline commit available — treating as cold start (no changed files)"
        return []
    }
    def changedFiles = sh(
        script: "git diff --name-only ${baseCommit}..HEAD",
        returnStdout: true
    ).trim().split('\n').findAll { it }
    return changedFiles
}
```

And `loadPipelineBaselines()` either disappears or shrinks to a one-line reader of the global baseline.

- [ ] **Step 4: Update params.MODE help text**

At Jenkinsfile L1472 (`choice` declaration), the description references rebuild-edge etc. No change needed — these still work via `manualModeAnalyze`.

- [ ] **Step 5: Run CPS-scope test**

Run: `cd genesis/orchestrator && pnpm exec vitest run jenkinsfile-cps-scope.test.mjs`

Expected: PASS. Specifically verify that the `[build:*]` tag parsing test from Task 1 still passes.

- [ ] **Step 6: Commit**

```bash
git add genesis/orchestrator/Jenkinsfile genesis/orchestrator/build-graph.groovy
git commit -m "refactor(orchestrator): delete legacy PIPELINES algorithm

Removes ~400 lines: the in-file PIPELINES map, analyzePipelineRequirements,
analyzeChangeset's per-pipeline cache, the comparison matrix, and the
known-divergences allowlist. The manifest-driven build graph is now the
sole source of truth for change detection."
```

---

## Task 8: Migrate JS consumers off orchestrator-strategy.mjs

**Files:**
- Modify: `genesis/orchestrator/preview.mjs`
- Modify: `genesis/orchestrator/scripts/registry-cluster-drift.mjs`
- Modify: `genesis/orchestrator/scripts/pipeline-trajectory.mjs`
- Modify: `genesis/orchestrator/pipeline-results.mjs` (re-exports nonManualPipelines)

**Why:** Each consumer currently does `import { PIPELINES, nonManualPipelines } from './orchestrator-strategy.mjs'`. They must switch to `pipeline-registry.mjs` before Task 10 deletes strategy.

- [ ] **Step 1: Migrate preview.mjs**

Open `genesis/orchestrator/preview.mjs`. Replace:

```javascript
import { simulate, PIPELINES } from './orchestrator-strategy.mjs';
```

with:

```javascript
import { loadPipelineRegistry, nonManualPipelines } from './pipeline-registry.mjs';
import { resolve } from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../..');
const registry = loadPipelineRegistry(ROOT);
```

`preview.mjs` uses `simulate()` to model a push. Re-implement `simulate` here OR move it into a new `simulate.mjs` that uses the registry. Minimal viable inline replacement:

```javascript
import { walkGraph } from './graph-walker.mjs';
import { loadManifests } from './manifest-utils.mjs';
import { parseSkipCi, parseCommitTags } from './commit-tag-parser.mjs';
// ^ extract parseSkipCi/parseCommitTags from orchestrator-strategy.mjs
//   into a new shared module (Task 8 Step 2 below)

function simulate({ changedFiles = [], commitMsg = '' } = {}) {
  if (parseSkipCi(commitMsg)) {
    return { pipelines: [], skipped: true };
  }
  const manifests = loadManifests(ROOT);
  const graphResult = walkGraph(manifests, changedFiles);
  let pipelines = [...new Set(graphResult.projects.map(p => {
    // Map gate-project back to its parent pipeline by looking up the manifest.
    const owning = manifests.find(m => m.content.gate?.projects?.[p.name]);
    return owning?.content.pipeline;
  }).filter(Boolean))];

  // Apply [build:*] overrides.
  const forced = parseCommitTags(commitMsg, registry);
  for (const name of forced) {
    if (!pipelines.includes(name) && registry.has(name)) {
      pipelines.push(name);
    }
  }

  // Filter manual-only.
  pipelines = pipelines.filter(p => !registry.get(p)?.manualOnly);

  return { pipelines, skipped: false };
}

const allNames = nonManualPipelines(registry).sort();
```

- [ ] **Step 2: Extract commit-tag parser to a shared module**

Create `genesis/orchestrator/commit-tag-parser.mjs`:

```javascript
/**
 * Commit-message tag parsing. Extracted from orchestrator-strategy.mjs
 * so it can survive that module's deletion.
 */

import { nonManualPipelines } from './pipeline-registry.mjs';

const BUILD_TAG_ALIASES = {
  edge: 'elohim-edge',
  dna: 'elohim-holochain',
  app: 'elohim',
  genesis: 'elohim-genesis',
  sophia: 'elohim-sophia',
  steward: 'elohim-steward',
};

export function parseCommitTags(commitMsg, registry) {
  const buildTags = [];
  const tagRegex = /\[build:([a-z,-]+)\]/gi;
  let match;
  while ((match = tagRegex.exec(commitMsg)) !== null) {
    for (const tag of match[1].split(',')) {
      const t = tag.trim().toLowerCase();
      if (t === 'all') {
        buildTags.push(...nonManualPipelines(registry));
      } else if (BUILD_TAG_ALIASES[t]) {
        buildTags.push(BUILD_TAG_ALIASES[t]);
      }
    }
  }
  return [...new Set(buildTags)];
}

export function parseSkipCi(commitMsg) {
  return /\[(skip ci|ci skip|no ci)\]/i.test(commitMsg);
}

export { BUILD_TAG_ALIASES };
```

Create matching test `genesis/orchestrator/commit-tag-parser.test.mjs`:

```javascript
import { test, describe } from 'node:test';
import { strict as assert } from 'node:assert';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseCommitTags, parseSkipCi } from './commit-tag-parser.mjs';
import { loadPipelineRegistry } from './pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../..');
const registry = loadPipelineRegistry(ROOT);

describe('parseCommitTags', () => {
  test('[build:app] returns elohim', () => {
    assert.deepStrictEqual(parseCommitTags('ci: retrigger [build:app]', registry), ['elohim']);
  });

  test('[build:edge,app] returns both', () => {
    assert.deepStrictEqual(
      parseCommitTags('[build:edge,app] fix things', registry).sort(),
      ['elohim', 'elohim-edge'].sort()
    );
  });

  test('[build:all] returns all non-manual pipelines', () => {
    const result = parseCommitTags('[build:all]', registry);
    assert.ok(!result.includes('elohim-steward'), 'should exclude steward (manualOnly)');
    assert.ok(result.includes('elohim'), 'should include elohim');
    assert.ok(result.length >= 6, `expected ≥6 pipelines, got ${result.length}`);
  });

  test('unknown tag silently dropped', () => {
    assert.deepStrictEqual(parseCommitTags('[build:nonsense] foo', registry), []);
  });

  test('no tag returns empty', () => {
    assert.deepStrictEqual(parseCommitTags('regular commit', registry), []);
  });
});

describe('parseSkipCi', () => {
  test('[skip ci] returns true', () => {
    assert.strictEqual(parseSkipCi('chore: docs [skip ci]'), true);
  });

  test('[ci skip] returns true', () => {
    assert.strictEqual(parseSkipCi('chore: docs [ci skip]'), true);
  });

  test('no tag returns false', () => {
    assert.strictEqual(parseSkipCi('regular commit'), false);
  });
});
```

Run: `cd genesis/orchestrator && pnpm exec vitest run commit-tag-parser.test.mjs`

Expected: PASS.

- [ ] **Step 3: Migrate scripts/registry-cluster-drift.mjs**

Open the file, find the `import { PIPELINES, nonManualPipelines } from '../orchestrator-strategy.mjs'` line. Replace with:

```javascript
import { loadPipelineRegistry, nonManualPipelines } from '../pipeline-registry.mjs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../../..');
const registry = loadPipelineRegistry(ROOT);

// Replace `Object.keys(PIPELINES)` with `[...registry.keys()]`.
// Replace `PIPELINES[name].manualOnly` with `registry.get(name).manualOnly`.
// etc.
```

Run: `cd genesis/orchestrator && node scripts/registry-cluster-drift.mjs --dry-run` (or whichever invocation is harmless) to verify no crash.

- [ ] **Step 4: Migrate scripts/pipeline-trajectory.mjs**

Same migration pattern as Step 3.

- [ ] **Step 5: Migrate pipeline-results.mjs**

Open `genesis/orchestrator/pipeline-results.mjs`. If it re-exports `nonManualPipelines` from orchestrator-strategy.mjs, change the source to pipeline-registry.mjs.

- [ ] **Step 6: Run all orchestrator tests**

Run: `cd genesis/orchestrator && pnpm exec vitest run`

Expected: most tests pass. `orchestrator-strategy.test.mjs` may still fail — Task 10 rewrites it.

- [ ] **Step 7: Commit**

```bash
git add genesis/orchestrator/preview.mjs genesis/orchestrator/commit-tag-parser.mjs genesis/orchestrator/commit-tag-parser.test.mjs genesis/orchestrator/scripts/registry-cluster-drift.mjs genesis/orchestrator/scripts/pipeline-trajectory.mjs genesis/orchestrator/pipeline-results.mjs
git commit -m "refactor(orchestrator): migrate JS consumers to pipeline-registry

preview.mjs, registry-cluster-drift, pipeline-trajectory, and
pipeline-results now read pipeline metadata from build-manifest.json
files via pipeline-registry.mjs. Commit-tag parsing extracted to
commit-tag-parser.mjs so it survives orchestrator-strategy.mjs deletion."
```

---

## Task 9: Regenerate pipeline-list.json from manifests

**Files:**
- Modify: `genesis/orchestrator/scripts/generate-pipeline-list.mjs`
- Modify: `genesis/orchestrator/pipeline-list.json` (regenerated)
- Modify: `genesis/orchestrator/scripts/count-pipeline-failures.sh` (verify schema unchanged)

**Why:** The Bash-consumable artifact must keep the same shape so `count-pipeline-failures.sh` doesn't break. Only the *source* changes (manifests instead of strategy.mjs).

- [ ] **Step 1: Update generate-pipeline-list.mjs**

Open `genesis/orchestrator/scripts/generate-pipeline-list.mjs`. Replace the import + iteration:

```javascript
import { loadPipelineRegistry } from '../pipeline-registry.mjs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { writeFileSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../../..');
const OUT = resolve(__dirname, '..', 'pipeline-list.json');

const registry = loadPipelineRegistry(ROOT);
const pipelines = [...registry.values()]
  .filter(p => p.jenkinsPath)  // only dispatchable pipelines
  .map(p => ({
    name: p.pipeline,
    manualOnly: p.manualOnly,
    triggersGenesis: p.triggersGenesis,
    cascades: p.cascades,
  }));

writeFileSync(OUT, JSON.stringify({
  generatedFrom: 'build-manifest.json files via pipeline-registry.mjs',
  pipelines,
}, null, 2) + '\n');

console.log(`Wrote ${pipelines.length} pipelines to ${OUT}`);
```

- [ ] **Step 2: Regenerate the artifact**

Run: `cd genesis/orchestrator && node scripts/generate-pipeline-list.mjs`

- [ ] **Step 3: Diff the result**

Run: `git diff genesis/orchestrator/pipeline-list.json`

Expected diff: only the `generatedFrom` string changes. The `pipelines` array must be identical (same names, same flags). If anything else changes, the manifests in Task 3 have a typo — fix and re-run.

- [ ] **Step 4: Verify count-pipeline-failures.sh still works**

Run: `bash genesis/orchestrator/scripts/count-pipeline-failures.sh dev` (or whatever the standard invocation is).

Expected: output looks the same as before this plan.

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/scripts/generate-pipeline-list.mjs genesis/orchestrator/pipeline-list.json
git commit -m "refactor(orchestrator): regenerate pipeline-list.json from manifests

generate-pipeline-list.mjs now reads from build-manifest.json via
pipeline-registry.mjs. Output schema unchanged — count-pipeline-failures.sh
and any other Bash consumers continue to work."
```

---

## Task 10: Delete orchestrator-strategy.mjs and rewrite its tests

**Files:**
- Delete: `genesis/orchestrator/orchestrator-strategy.mjs`
- Modify: `genesis/orchestrator/orchestrator-strategy.test.mjs` (rewrite as manifest-driven tests)
- Modify: `genesis/orchestrator/ci-ignore.mjs` (verify the re-exports from strategy don't break)
- Modify: `genesis/orchestrator/package.json` (verify no test glob hardcodes the deleted file)

**Why:** With Task 8 done, no production code imports `orchestrator-strategy.mjs`. Its tests duplicate what `commit-tag-parser.test.mjs` (Task 8) and `pipeline-registry.test.mjs` (Task 4) cover. The drift-detection tests need a new anchor.

- [ ] **Step 1: Verify nothing still imports orchestrator-strategy.mjs**

Run: `grep -rn "orchestrator-strategy" /projects/elohim/ 2>/dev/null | grep -v node_modules | grep -v .claude/worktrees`

Expected: only the deletion-targeted files (orchestrator-strategy.mjs, orchestrator-strategy.test.mjs) and stale references in archived docs. If any production code still imports it, fix that first.

- [ ] **Step 2: Rewrite orchestrator-strategy.test.mjs as orchestrator-integration.test.mjs**

Decide: rename the file to reflect new scope, or replace contents in place. Recommended rename — old name implies legacy.

Move keeper test groups to the renamed file:
- `parseCiIgnore` tests → keep, dependency is on ci-ignore.mjs (still alive)
- `matchesCiIgnore` tests → keep
- `CI_IGNORE_PATTERNS` tests → keep
- `pipeline-list.json drift` tests → adapt to read from registry + assert pipeline-list.json matches

Delete:
- `changeset routing` tests → covered by graph-walker.test.mjs
- `cascade propagation` tests → covered by build-graph.groovy CPS-scope tests + propagateDependencies in Jenkinsfile (no JS port left to test directly)
- `commit message tags` tests → covered by commit-tag-parser.test.mjs
- `dependency ordering` tests → covered by graph-walker's topoSort tests
- `real-world scenarios` tests → covered by graph-walker + manifest tests
- `drift detection: mirror vs live Jenkinsfile` tests → no mirror exists, so no drift to detect

Add a new drift test that verifies pipeline-list.json matches the registry:

```javascript
describe('pipeline-list.json drift', () => {
  test('pipeline-list.json matches what generate-pipeline-list.mjs would produce', () => {
    const registry = loadPipelineRegistry(ROOT);
    const expected = [...registry.values()]
      .filter(p => p.jenkinsPath)
      .map(p => ({
        name: p.pipeline,
        manualOnly: p.manualOnly,
        triggersGenesis: p.triggersGenesis,
        cascades: p.cascades,
      }));
    const actual = JSON.parse(readFileSync(
      resolve(__dirname, 'pipeline-list.json'), 'utf8'
    )).pipelines;
    assert.deepStrictEqual(
      actual.sort((a, b) => a.name.localeCompare(b.name)),
      expected.sort((a, b) => a.name.localeCompare(b.name)),
      'pipeline-list.json is stale — run node scripts/generate-pipeline-list.mjs'
    );
  });
});
```

- [ ] **Step 3: Update ci-ignore.mjs if needed**

`orchestrator-strategy.mjs` re-exports `parseCiIgnore, matchesCiIgnore, CI_IGNORE_PATTERNS` from `ci-ignore.mjs`. After deletion, callers that did `import { matchesCiIgnore } from './orchestrator-strategy.mjs'` must switch to `./ci-ignore.mjs`. Grep:

```bash
grep -rn "from.*orchestrator-strategy" /projects/elohim/ 2>/dev/null | grep -v node_modules | grep -v .claude/worktrees
```

Update each to import from `./ci-ignore.mjs` directly.

- [ ] **Step 4: Delete the strategy module**

Run: `git rm genesis/orchestrator/orchestrator-strategy.mjs genesis/orchestrator/orchestrator-strategy.test.mjs`

- [ ] **Step 5: Run all orchestrator tests**

Run: `cd genesis/orchestrator && pnpm exec vitest run`

Expected: all tests pass. No test should reference orchestrator-strategy.mjs.

- [ ] **Step 6: Commit**

```bash
git add genesis/orchestrator/ci-ignore.mjs
git rm genesis/orchestrator/orchestrator-strategy.mjs genesis/orchestrator/orchestrator-strategy.test.mjs
# Add the rewritten/moved test file:
git add genesis/orchestrator/orchestrator-integration.test.mjs
git commit -m "refactor(orchestrator): delete orchestrator-strategy.mjs and its mirror tests

The JS mirror of the Jenkinsfile PIPELINES map is dead code — all
consumers now read from pipeline-registry.mjs (manifest-backed) or
commit-tag-parser.mjs. Drift detection re-anchored to the
pipeline-list.json artifact."
```

---

## Task 11: Update operator docs

**Files:**
- Modify: `CLAUDE.md` (root, CI/CD section)
- Modify: `genesis/orchestrator/README.md`
- Modify: `.claude/memory/project_orchestrator_build_tag_syntax.md`
- Modify: `.husky/pre-push` (verify it never imported strategy.mjs)

- [ ] **Step 1: Audit .husky/pre-push**

Run: `grep -n "orchestrator-strategy\|PIPELINES" /projects/elohim/.husky/pre-push`

Expected: empty or only comments. If it imports anything from strategy.mjs, migrate to pipeline-registry.mjs.

- [ ] **Step 2: Update root CLAUDE.md**

In `/projects/elohim/CLAUDE.md`, find the CI/CD section. Replace any reference to "PIPELINES map" with "build-manifest.json registry". Update the pipeline table to reference the manifest path (e.g., `elohim/holochain/build-manifest.json`) instead of change patterns.

- [ ] **Step 3: Update orchestrator README**

In `/projects/elohim/genesis/orchestrator/README.md`, replace the example `PIPELINES = [...]` block with an example `build-manifest.json` excerpt showing `pipeline`, `jenkinsPath`, `manualOnly`, `triggersGenesis`, `cascades`, `dependsOn`.

- [ ] **Step 4: Update memory entry**

In `.claude/memory/project_orchestrator_build_tag_syntax.md`, the "2026-05-28 caveat — webhook-gate silent-drop" section is now historical. Either:
- (a) Mark it as RESOLVED in this plan and add a pointer to this plan's commit SHAs
- (b) Delete the caveat section, leave the tag-syntax table

Recommended (a) — historical context is valuable.

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md genesis/orchestrator/README.md .claude/memory/project_orchestrator_build_tag_syntax.md
git commit -m "docs: update CI/CD references after PIPELINES → manifests migration"
```

---

## Task 12: Smoke-test the orchestrator end-to-end

**Files:** None (operational verification)

**Why:** Before declaring this plan complete, the orchestrator must dispatch correctly for representative scenarios.

- [ ] **Step 1: Push a no-op commit with [build:app] tag from a non-webhook trigger context**

The simplest test is to push from the operator's normal flow and confirm `elohim` dispatches. If timer-triggered re-pickup is possible to simulate (the original bug from #1078), great.

```bash
git commit --allow-empty -m "test: smoke-test [build:app] tag-routing post-migration"
git push origin sprint/cross-pillar-cleanup
```

Watch the next orchestrator build's log for:
- `🔧 [build:*] tags detected — force-including: elohim` — confirms Task 1
- `🔧 [build:*] force-include applied to graph result: elohim` — confirms Task 1's wiring still works
- NO `⚠️  X DIVERGENCE(S)` warnings — confirms Task 7

- [ ] **Step 2: Push a real path change matching one pipeline's manifest sources**

Touch `app/elohim-app/src/app/app.component.ts` with a trivial comment edit. Push. Confirm `elohim` dispatches via changeset (not via tag) — proves the registry-backed metadata flows correctly.

- [ ] **Step 3: Push a Jenkinsfile-only change**

Touch `genesis/orchestrator/Jenkinsfile` with a comment edit. Push. Confirm the dispatch decision uses the per-pipeline jenkinsfile rule (only the owning pipeline triggers — this case it would be `elohim-orchestrator`).

- [ ] **Step 4: Push a [build:all] tag**

`git commit --allow-empty -m "test: [build:all]" && git push`

Confirm every non-manual pipeline dispatches and `elohim-steward` does NOT.

- [ ] **Step 5: Document any issues found**

If any smoke test reveals a bug, fix it in a follow-up commit on this branch. Update the plan's risk table accordingly.

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Manifest field typo in Task 3 → pipeline silently undispatched | Task 9 Step 3 diffs the regenerated pipeline-list.json against current; any typo shows as field drift |
| Graph-only pipelines (elohim-doorway-app, elohim-compute) get accidentally dispatched | Task 6 Step 6 filters `dispatchablePipelines` by jenkinsPath presence |
| `getPipelineMetadata` shell-find performance hit | Only called in fallback path (pre-build-graph); main path reads from `env.PIPELINE_REGISTRY_JSON` (cached) |
| External consumer outside this plan still imports orchestrator-strategy.mjs | Task 10 Step 1 grep enforces this; CI fails the next build if anything references the deleted file |
| Webhook gate test (Task 1 Step 4) regresses if someone re-wraps tag parsing | Test asserts the conditional context — strict enough to catch the re-wrap |
| Smoke test in Task 12 reveals build-graph bug | This plan does not change build-graph semantics, only delete its comparison output; if a bug shows, it predates this plan and should be fixed in a separate commit |
| `[deploy-only]` mode accidentally extended to non-webhook triggers | Task 1 Step 4 explicitly tests the deploy-only gate remains webhook-only |
| `analyzeChangeset` deletion breaks `loadPipelineBaselines` callers | Task 7 Step 3 handles the dependency chain; verify by running `vitest run` after each commit |

---

## Self-Review

**Spec coverage:**
- ✅ Webhook-gate fix → Task 1
- ✅ Move pipeline metadata into manifests → Tasks 2, 3
- ✅ New pipeline-registry helper → Task 4
- ✅ Surface metadata from build-graph result → Task 5
- ✅ Migrate Jenkinsfile to use manifest metadata → Task 6
- ✅ Delete PIPELINES map + legacy functions → Task 7
- ✅ Delete comparison matrix → Task 7 Step 2
- ✅ Migrate JS consumers → Task 8
- ✅ Regenerate pipeline-list.json → Task 9
- ✅ Delete orchestrator-strategy.mjs → Task 10
- ✅ Update docs → Task 11
- ✅ Smoke test → Task 12

**Type consistency:** `getPipelineMetadata`, `loadPipelineRegistry`, `nonManualPipelines`, `dispatchablePipelines`, `pipelinesThatTriggerGenesis`, `pipelineDependencyMap` — names consistent across all tasks. Registry is always `Map<string, {pipeline, jenkinsPath?, manualOnly, triggersGenesis, cascades, dependsOn, manifestPath}>`.

**Placeholder scan:** Every code step has actual code; every test step has actual assertions; every commit step has actual messages.

---
