---
id: ci-cd-resolution-gap-close
status: Draft
related:
  - ../superpowers/plans/2026-05-28-orchestrator-clean-build-triggers.md   # sibling CI/orchestrator pipeline work
---

# CI/CD Resolution-Gap Close — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the resolution gap between Jenkins/orchestrator and local build-graph/husky so developers learn whether their work is good in minutes (not hours), eliminate cascade-storm waste (1-hour integrations repeated 4× silently), and treat CI as an extension of development rather than a deferred verdict.

**Architecture:** Foundation-first. Phase 0 lands two shared modules (pipeline registry + result classification) that every other phase consumes. Phases 1-7 can then ship in any order — each phase produces a measurable win on its own. Existing instrumentation (`pipeline-trajectory.mjs`) is the measurement substrate; each phase is judged by the trajectory tool's pattern detector before and after.

**Tech Stack:** Node.js (ES modules), Bash (POSIX `sh` for husky, with a phased move to `bash`), Groovy (Jenkinsfile CPS), `just` (recipe runner), `node:test` framework, `vitest` for TypeScript projects, Jenkins REST API (anonymous read).

**Working memory references:**
- `_jenkins_mcp_anonymous_mode` — Jenkins reads work without auth; sending auth triggers OIDC redirect
- `_quilt_as_native_s3_surface` + `_holochain_build_pvcs_in_jenkins_ns` — caching substrate
- `_pre_dispatch_hard_fail_post_dispatch_unstable` — orchestrator Jenkinsfile convention
- `_cascade_halt_masks_failures` — green-driving surfaces buried failures one layer at a time
- `_orchestrator_abort_baseline_rollback` — aborting orchestrator forces full-chain rebuild

**Success criteria (measured after each phase):**
- Phase 0: zero divergence between three pipeline lists; `node --test` passes for both new modules
- Phase 1: a storage-only push dispatches storage rebuild only (1 image, ~15 min) instead of full edge + genesis cascade (6 images + seeding, ~85 min)
- Phase 2: `supersede-waste` pattern drops to 0/10 in trajectory tool output across two weeks
- Phase 3: pre-push hook is ≤200 lines, no `eval`, identical change-detection to orchestrator
- Phase 4: trajectory tool runs in <2s, surfaces registry-vs-cluster drift, alert-tunable
- Phase 5: storage Docker rebuild cache-hit reduces wall-time by ≥10 min
- Phase 6: DNA sweettest stage drops ≥20 min via sharding
- Phase 7: genesis pipeline drops ≥3 min via stage parallelization

---

## File Structure

**New files (created by this plan):**

| Path | Responsibility |
|---|---|
| `genesis/orchestrator/pipeline-results.mjs` | Canonical SUCCESS/UNSTABLE/FAILURE/ABORTED classifier + waste-detection helpers |
| `genesis/orchestrator/pipeline-results.test.mjs` | Test mirror for `pipeline-results.mjs` |
| `genesis/orchestrator/pipeline-list.json` | Generated artifact: pipeline names + manualOnly flags, consumed by shell tools |
| `genesis/orchestrator/scripts/generate-pipeline-list.mjs` | Generator that writes `pipeline-list.json` from `orchestrator-strategy.mjs` |
| `genesis/orchestrator/scripts/jenkins-client.mjs` | Shared Jenkins API wrapper — anonymous, retried, instrumented |
| `genesis/orchestrator/scripts/registry-cluster-drift.mjs` | Detects pipelines in registry without a Jenkins job and vice versa |

**Existing files modified:**

| Path | Modification |
|---|---|
| `genesis/orchestrator/orchestrator-strategy.mjs` | Add `nonManualPipelines()` helper + re-export from `pipeline-results.mjs` |
| `genesis/orchestrator/scripts/pipeline-trajectory.mjs` | Consume `pipeline-results.mjs` + `jenkins-client.mjs`; add index, tunable constants, watch mode |
| `genesis/orchestrator/scripts/count-pipeline-failures.sh` | Read from `pipeline-list.json`; use shared anonymous-curl from `jenkins-client.sh` companion |
| `genesis/orchestrator/reconcile-build-graph.mjs` | Import `SUCCESSFUL_RESULTS` from `pipeline-results.mjs` |
| `genesis/orchestrator/orchestrator-strategy.test.mjs` | Drift assertion: `count-pipeline-failures.sh`'s JSON consumption equals registry |
| `genesis/orchestrator/Jenkinsfile` | Storage-only routing branch; genesis auto-include tightening; `shortSha` Groovy helper |
| `genesis/orchestrator/manifests/elohim-edge-storage/build-manifest.json` | New manifest scoping a storage-only sub-pipeline (Phase 1 alternative B) |
| `.husky/pre-push` | Eliminate `eval`; collapse duplicated case arms; subshell `run_gate`; bash + pipefail; storybook story-content paths in fallback |
| `elohim/holochain/Jenkinsfile` | `abortPrevious: true`; remove `--no-cache`; restructure Doorway cargo layer |
| `elohim/holochain/dna/Jenkinsfile` | `abortPrevious: true`; split sweettest into 2 parallel shards |
| `genesis/Jenkinsfile` | Parallelize install/schema/validate vs target-health stages |

---

## Phase 0 — Foundation: shared modules

This phase unblocks every other phase. Do not skip.

### Task 0.1: Add `nonManualPipelines()` helper to orchestrator-strategy.mjs

**Files:**
- Modify: `genesis/orchestrator/orchestrator-strategy.mjs` (append after `PIPELINES` export)

- [ ] **Step 1: Write the failing test**

Append to `genesis/orchestrator/orchestrator-strategy.test.mjs` (find the `describe('PIPELINES')` block or add at top-level):

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PIPELINES, nonManualPipelines } from './orchestrator-strategy.mjs';

test('nonManualPipelines: excludes manualOnly entries', () => {
  const result = nonManualPipelines();
  assert.ok(Array.isArray(result), 'returns an array');
  assert.ok(!result.includes('elohim-steward'), 'elohim-steward (manualOnly) excluded');
  assert.ok(result.includes('elohim-holochain'), 'elohim-holochain included');
  assert.ok(result.includes('elohim-epr'), 'elohim-epr included');
});

test('nonManualPipelines: returns names matching PIPELINES keys', () => {
  const result = nonManualPipelines();
  for (const name of result) {
    assert.ok(PIPELINES[name], `${name} is a known pipeline`);
    assert.ok(!PIPELINES[name].manualOnly, `${name} is not manualOnly`);
  }
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
node --test genesis/orchestrator/orchestrator-strategy.test.mjs 2>&1 | grep -A2 nonManualPipelines
```

Expected: FAIL with `SyntaxError: The requested module './orchestrator-strategy.mjs' does not provide an export named 'nonManualPipelines'`

- [ ] **Step 3: Implement the helper**

Append to `genesis/orchestrator/orchestrator-strategy.mjs` immediately after the `PIPELINES` export (before the `parseCiIgnore` re-export):

```javascript
/**
 * Returns the names of all pipelines that the orchestrator may auto-dispatch
 * (i.e., excluding manualOnly entries like elohim-steward). Single source of
 * truth for tools that need to enumerate the orchestrator's dispatchable set
 * — count-pipeline-failures.sh, pipeline-trajectory.mjs, registry-cluster-drift.mjs.
 */
export function nonManualPipelines() {
  return Object.keys(PIPELINES).filter(name => !PIPELINES[name].manualOnly);
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
node --test genesis/orchestrator/orchestrator-strategy.test.mjs 2>&1 | tail -10
```

Expected: PASS, including the two `nonManualPipelines` tests.

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/orchestrator-strategy.mjs genesis/orchestrator/orchestrator-strategy.test.mjs
git commit -m "feat(orchestrator): add nonManualPipelines() helper — single source of truth for dispatchable pipeline set"
```

---

### Task 0.2: Create `pipeline-results.mjs` classification module

**Files:**
- Create: `genesis/orchestrator/pipeline-results.mjs`
- Create: `genesis/orchestrator/pipeline-results.test.mjs`

- [ ] **Step 1: Write the failing test**

Create `genesis/orchestrator/pipeline-results.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  SUCCESSFUL_RESULTS,
  TERMINAL_FAILURE_RESULTS,
  classifyResult,
  isWasted,
  isSuccess,
  isFailure,
} from './pipeline-results.mjs';

test('SUCCESSFUL_RESULTS: SUCCESS + UNSTABLE only', () => {
  assert.deepEqual([...SUCCESSFUL_RESULTS].sort(), ['SUCCESS', 'UNSTABLE']);
});

test('TERMINAL_FAILURE_RESULTS: FAILURE only (ABORTED is waste, not failure)', () => {
  assert.deepEqual([...TERMINAL_FAILURE_RESULTS], ['FAILURE']);
});

test('classifyResult: maps to one of {success, failure, wasted, pending, skipped}', () => {
  assert.equal(classifyResult('SUCCESS'), 'success');
  assert.equal(classifyResult('UNSTABLE'), 'success');
  assert.equal(classifyResult('FAILURE'), 'failure');
  assert.equal(classifyResult('ABORTED'), 'wasted');
  assert.equal(classifyResult('NOT_BUILT'), 'skipped');
  assert.equal(classifyResult(null), 'pending');
  assert.equal(classifyResult(undefined), 'pending');
});

test('isSuccess / isFailure / isWasted: convenience predicates', () => {
  assert.ok(isSuccess('SUCCESS'));
  assert.ok(isSuccess('UNSTABLE'));
  assert.ok(!isSuccess('FAILURE'));
  assert.ok(isFailure('FAILURE'));
  assert.ok(!isFailure('ABORTED'));
  assert.ok(isWasted('ABORTED'));
  assert.ok(!isWasted('NOT_BUILT'));
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
node --test genesis/orchestrator/pipeline-results.test.mjs 2>&1 | head -10
```

Expected: FAIL with `Cannot find module './pipeline-results.mjs'`

- [ ] **Step 3: Implement the module**

Create `genesis/orchestrator/pipeline-results.mjs`:

```javascript
/**
 * Pipeline result classification — single source of truth for what counts
 * as success, failure, or waste across all CI/CD tooling.
 *
 * Why three buckets:
 *   - success: SUCCESS + UNSTABLE — the user's work passed enough gates to be
 *     useful. UNSTABLE means non-blocking issues; we don't pretend it's clean
 *     but we also don't pretend it's broken.
 *   - failure: FAILURE only. ABORTED is NOT a failure of the work; it's a
 *     failure of CI orchestration (we asked it to stop) and should be counted
 *     as waste so concurrency tuning can act on it.
 *   - wasted: ABORTED — the build never got a chance to verdict on the work.
 *     Persistent waste signals supersede-thrash or operator-aborts.
 *
 * Used by:
 *   - pipeline-trajectory.mjs (pattern detector)
 *   - reconcile-build-graph.mjs (success-set check)
 *   - count-pipeline-failures.sh (via pipeline-list.json side channel)
 */

export const SUCCESSFUL_RESULTS = new Set(['SUCCESS', 'UNSTABLE']);
export const TERMINAL_FAILURE_RESULTS = new Set(['FAILURE']);
export const WASTED_RESULTS = new Set(['ABORTED']);
export const SKIPPED_RESULTS = new Set(['NOT_BUILT']);

/**
 * @param {string|null|undefined} result Jenkins build result string
 * @returns {'success'|'failure'|'wasted'|'skipped'|'pending'}
 */
export function classifyResult(result) {
  if (result == null) return 'pending';
  if (SUCCESSFUL_RESULTS.has(result)) return 'success';
  if (TERMINAL_FAILURE_RESULTS.has(result)) return 'failure';
  if (WASTED_RESULTS.has(result)) return 'wasted';
  if (SKIPPED_RESULTS.has(result)) return 'skipped';
  return 'pending';
}

export function isSuccess(result) { return classifyResult(result) === 'success'; }
export function isFailure(result) { return classifyResult(result) === 'failure'; }
export function isWasted(result)  { return classifyResult(result) === 'wasted'; }
```

- [ ] **Step 4: Run test to verify it passes**

```bash
node --test genesis/orchestrator/pipeline-results.test.mjs 2>&1 | tail -5
```

Expected: PASS (all 4 tests).

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/pipeline-results.mjs genesis/orchestrator/pipeline-results.test.mjs
git commit -m "feat(orchestrator): pipeline-results.mjs — canonical result classifier (success/failure/wasted/skipped/pending)"
```

---

### Task 0.3: Migrate `pipeline-trajectory.mjs` to use `pipeline-results.mjs`

**Files:**
- Modify: `genesis/orchestrator/scripts/pipeline-trajectory.mjs:198-253` (trajectory + pattern code)

- [ ] **Step 1: Replace inline success-set checks with import**

Edit the import block at the top of `genesis/orchestrator/scripts/pipeline-trajectory.mjs`:

```javascript
import process from 'node:process';
import { PIPELINES as PIPELINE_REGISTRY, nonManualPipelines } from '../orchestrator-strategy.mjs';
import { isSuccess, isFailure, isWasted, classifyResult } from '../pipeline-results.mjs';
```

- [ ] **Step 2: Replace the inline success-rate computation**

Find:

```javascript
const completed = stream.filter(s => s.result != null);
const success = stream.filter(s => s.result === 'SUCCESS' || s.result === 'UNSTABLE').length;
```

Replace with:

```javascript
const completed = stream.filter(s => s.result != null);
const success = stream.filter(s => isSuccess(s.result)).length;
```

- [ ] **Step 3: Replace the persistent-failure detector**

Find:

```javascript
const failures = completed.filter(s => s.result === 'FAILURE').length;
```

Replace with:

```javascript
const failures = completed.filter(s => isFailure(s.result)).length;
```

- [ ] **Step 4: Replace the supersede-waste detector**

Find:

```javascript
const aborted = t.stream.filter(s => s.result === 'ABORTED').length;
```

Replace with:

```javascript
const aborted = t.stream.filter(s => isWasted(s.result)).length;
```

- [ ] **Step 5: Replace orchestrator-failure-streak detector**

Find:

```javascript
const orchSuccess = orchCompleted.filter(
  r => r.result === 'SUCCESS' || r.result === 'UNSTABLE',
).length;
```

Replace with:

```javascript
const orchSuccess = orchCompleted.filter(r => isSuccess(r.result)).length;
```

- [ ] **Step 6: Replace the default pipeline list using `nonManualPipelines()`**

Find:

```javascript
const DEFAULT_PIPELINES = Object.keys(PIPELINE_REGISTRY)
  .filter(k => !PIPELINE_REGISTRY[k].manualOnly)
  .join(',');
```

Replace with:

```javascript
const DEFAULT_PIPELINES = nonManualPipelines().join(',');
```

- [ ] **Step 7: Smoke test against live Jenkins**

```bash
JENKINS_URL=https://jenkins.ethosengine.com node genesis/orchestrator/scripts/pipeline-trajectory.mjs --builds 5
```

Expected: table renders identical content to pre-change; no new WARN lines beyond what was already there.

- [ ] **Step 8: Commit**

```bash
git add genesis/orchestrator/scripts/pipeline-trajectory.mjs
git commit -m "refactor(orchestrator): pipeline-trajectory.mjs consumes pipeline-results.mjs — drop inline result-set logic"
```

---

### Task 0.4: Migrate `reconcile-build-graph.mjs` to use `pipeline-results.mjs`

**Files:**
- Modify: `genesis/orchestrator/reconcile-build-graph.mjs:21-22`

- [ ] **Step 1: Read current state**

```bash
sed -n '15,30p' genesis/orchestrator/reconcile-build-graph.mjs
```

Confirm line 21-22 defines `SUCCESSFUL_RESULTS = new Set(['SUCCESS', 'UNSTABLE'])`.

- [ ] **Step 2: Replace inline set with import**

Find the local `SUCCESSFUL_RESULTS` definition. Replace it with an import at the top of the file:

```javascript
import { SUCCESSFUL_RESULTS } from './pipeline-results.mjs';
```

Delete the local `const SUCCESSFUL_RESULTS = new Set(['SUCCESS', 'UNSTABLE']);` line.

- [ ] **Step 3: Run reconcile-build-graph tests**

```bash
node --test genesis/orchestrator/reconcile-build-graph.test.mjs 2>&1 | tail -5
```

Expected: PASS (all existing tests).

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/reconcile-build-graph.mjs
git commit -m "refactor(orchestrator): reconcile-build-graph.mjs imports SUCCESSFUL_RESULTS from pipeline-results.mjs"
```

---

### Task 0.5: Generate `pipeline-list.json` artifact for shell consumers

**Files:**
- Create: `genesis/orchestrator/scripts/generate-pipeline-list.mjs`
- Modify: `genesis/orchestrator/justfile` (add `ci-pipeline-list` recipe)
- Modify: `.husky/pre-push` (run generator on relevant changes)

- [ ] **Step 1: Write the generator**

Create `genesis/orchestrator/scripts/generate-pipeline-list.mjs`:

```javascript
#!/usr/bin/env node
/**
 * Generates genesis/orchestrator/pipeline-list.json from PIPELINES in
 * orchestrator-strategy.mjs. Shell scripts (count-pipeline-failures.sh,
 * jenkins-client.sh) consume the JSON instead of hardcoding their own lists.
 *
 * Run by:
 *   - just ci-pipeline-list      (manual / pre-commit)
 *   - .husky/pre-push            (when orchestrator-strategy.mjs changes)
 *   - genesis/orchestrator/Jenkinsfile  (as a sanity check stage)
 */

import { writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PIPELINES } from '../orchestrator-strategy.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const out = resolve(__dirname, '..', 'pipeline-list.json');

const payload = {
  generatedFrom: 'orchestrator-strategy.mjs PIPELINES',
  generatedAt: new Date().toISOString().slice(0, 10),
  pipelines: Object.entries(PIPELINES).map(([name, cfg]) => ({
    name,
    manualOnly: !!cfg.manualOnly,
    triggersGenesis: !!cfg.triggersGenesis,
    cascades: cfg.cascades !== false,
  })),
};

writeFileSync(out, JSON.stringify(payload, null, 2) + '\n');
console.log(`wrote ${out} (${payload.pipelines.length} pipelines)`);
```

- [ ] **Step 2: Make it executable and run it**

```bash
chmod +x genesis/orchestrator/scripts/generate-pipeline-list.mjs
node genesis/orchestrator/scripts/generate-pipeline-list.mjs
cat genesis/orchestrator/pipeline-list.json
```

Expected: JSON file with 8 entries (holochain, edge, elohim, genesis, steward [manualOnly:true], sophia, epr, storybook).

- [ ] **Step 3: Add `just ci-pipeline-list` recipe**

Append to `genesis/orchestrator/justfile`:

```just
# Regenerate pipeline-list.json from orchestrator-strategy.mjs.
# Run after editing PIPELINES.
ci-pipeline-list:
  @node scripts/generate-pipeline-list.mjs
```

- [ ] **Step 4: Wire pre-push to enforce freshness**

In `.husky/pre-push`, find the `if [ "$USE_MANIFEST" = false ]; then` block and add a new project trigger BEFORE that block (around line 184):

```sh
# Pipeline-list freshness: if orchestrator-strategy.mjs changed, the
# generated pipeline-list.json must be regenerated and committed.
if echo "$CHANGED" | grep -q "^genesis/orchestrator/orchestrator-strategy.mjs$"; then
  PROJECTS="$PROJECTS pipeline-list-fresh"
fi
```

Then add the gate handler inside `run_gate()`'s schema-checks block (where `schema-validate`, `schema-codegen`, etc. live), as a new case arm:

```sh
      pipeline-list-fresh)
        echo "[$PROJECT_NAME] Verifying pipeline-list.json is fresh..."
        node genesis/orchestrator/scripts/generate-pipeline-list.mjs >/dev/null
        if ! git diff --quiet -- genesis/orchestrator/pipeline-list.json; then
          echo "ERROR: pipeline-list.json is stale relative to orchestrator-strategy.mjs"
          echo "  Run: just -d genesis/orchestrator ci-pipeline-list && git add genesis/orchestrator/pipeline-list.json"
          rc=1
        else
          rc=0
        fi
        ;;
```

Also add `pipeline-list-fresh` to the `if [ "$PROJECT_NAME" = "schema-validate" ...` long condition at line 293, and add the directory mapping `pipeline-list-fresh)  PROJECT_DIR="." ;;` in the project-dir case statement at line 637.

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/scripts/generate-pipeline-list.mjs \
        genesis/orchestrator/pipeline-list.json \
        genesis/orchestrator/justfile \
        .husky/pre-push
git commit -m "feat(orchestrator): pipeline-list.json + freshness gate — shell tools consume canonical list"
```

---

### Task 0.6: Migrate `count-pipeline-failures.sh` to read `pipeline-list.json`

**Files:**
- Modify: `genesis/orchestrator/scripts/count-pipeline-failures.sh:17-28`

- [ ] **Step 1: Read current state**

```bash
sed -n '1,50p' genesis/orchestrator/scripts/count-pipeline-failures.sh
```

- [ ] **Step 2: Replace the hardcoded array with a `jq` read**

Find:

```sh
# Source of truth: orchestrator-strategy.mjs PIPELINES.
PIPELINES=(
  "elohim-orchestrator"
  ...
)
```

Replace with:

```sh
# Read pipeline names from pipeline-list.json (generated from
# orchestrator-strategy.mjs by scripts/generate-pipeline-list.mjs).
# Include elohim-orchestrator explicitly because it's the orchestrator
# itself and not in the dispatchable downstream set.
REPO_ROOT="$(git rev-parse --show-toplevel)"
PIPELINE_LIST="$REPO_ROOT/genesis/orchestrator/pipeline-list.json"
if [ ! -f "$PIPELINE_LIST" ]; then
  echo "ERROR: $PIPELINE_LIST not found; run 'just -d genesis/orchestrator ci-pipeline-list'" >&2
  exit 2
fi
# shellcheck disable=SC2207
PIPELINES=( "elohim-orchestrator" $(jq -r '.pipelines[] | select(.manualOnly | not) | .name' "$PIPELINE_LIST") )
```

- [ ] **Step 3: Run the script and verify output unchanged**

```bash
bash genesis/orchestrator/scripts/count-pipeline-failures.sh 2>&1 | head -20
```

Expected: same count as before (the script's contract is unchanged; we only fixed the source).

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/scripts/count-pipeline-failures.sh
git commit -m "fix(orchestrator): count-pipeline-failures.sh reads pipeline-list.json — closes 3-way drift"
```

---

### Task 0.7: Drift assertion test

**Files:**
- Modify: `genesis/orchestrator/orchestrator-strategy.test.mjs`

- [ ] **Step 1: Add a drift-detection test**

Append to `genesis/orchestrator/orchestrator-strategy.test.mjs`:

```javascript
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = resolve(fileURLToPath(import.meta.url), '..');

test('pipeline-list.json is in sync with PIPELINES (regenerate via just ci-pipeline-list)', () => {
  const listPath = resolve(__dirname, 'pipeline-list.json');
  assert.ok(existsSync(listPath), 'pipeline-list.json must exist');
  const json = JSON.parse(readFileSync(listPath, 'utf8'));
  const jsonNames = new Set(json.pipelines.map(p => p.name));
  const registryNames = new Set(Object.keys(PIPELINES));
  assert.deepEqual(
    [...jsonNames].sort(),
    [...registryNames].sort(),
    'pipeline-list.json names differ from PIPELINES — regenerate with just ci-pipeline-list',
  );
});
```

- [ ] **Step 2: Run test**

```bash
node --test genesis/orchestrator/orchestrator-strategy.test.mjs 2>&1 | grep -E "(pass|fail|drift)"
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/orchestrator-strategy.test.mjs
git commit -m "test(orchestrator): drift assertion — pipeline-list.json must equal PIPELINES"
```

---

**Phase 0 complete.** Foundation modules + drift detection in place. All subsequent phases can ship in any order.

---

## Phase 1 — Cascade containment

Highest ROI on the 1hr×4 problem. Address the cascading from storage-only changes that triggered build #996 (121 min) and the genesis auto-include that adds ~30 min to every edge-only push.

### Task 1.1: Tighten genesis auto-include rule

**Background:** `genesis/orchestrator/Jenkinsfile`'s `applyBuildGraphRouting` auto-includes genesis on any push that triggers a `triggersGenesis: true` pipeline. This adds 25-35 min to edge-only and storage-only pushes that don't change genesis content. Fix: only auto-include genesis when (edge AND app rebuild together) OR (any `genesis/` path was touched), not on edge-alone.

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile` (the `applyBuildGraphRouting` function, ~line 928)

- [ ] **Step 1: Locate the auto-include logic**

```bash
grep -n "triggersGenesis" genesis/orchestrator/Jenkinsfile | head -20
```

Note the line numbers of the auto-include code path.

- [ ] **Step 2: Replace the trigger predicate**

Find the block (likely near lines 839-847 per code review):

```groovy
if (triggeringPipeline && isDevBranch) {
  // auto-include genesis
}
```

Replace with:

```groovy
// Auto-include genesis ONLY when:
//   a) a genesis/ path was touched in the changeset, OR
//   b) BOTH edge AND app are being rebuilt (full-deploy shape)
//
// This stops storage-only and edge-only pushes from paying 25-35 min
// for a genesis seeding cycle they don't need.
def genesisTouched = changedFiles.any { it.startsWith('genesis/') }
def fullDeployShape = dispatchedPipelines.contains('elohim-edge') &&
                      dispatchedPipelines.contains('elohim')
if (isDevBranch && (genesisTouched || fullDeployShape)) {
  // ... existing auto-include body ...
}
```

- [ ] **Step 3: Add a comment + log line so it's audible at runtime**

In the same block, prepend a log:

```groovy
echo "[orchestrator] genesis auto-include: touched=${genesisTouched} fullDeploy=${fullDeployShape}"
```

- [ ] **Step 4: Validate with `just ci-preview`**

```bash
# Simulate a storage-only changeset
just -d genesis/orchestrator ci-preview elohim/elohim-storage/src/views.rs
```

Expected output: `elohim-edge` dispatched; `elohim-genesis` NOT in dispatched set.

- [ ] **Step 5: Validate with a full-deploy changeset**

```bash
just -d genesis/orchestrator ci-preview elohim/elohim-storage/src/views.rs app/elohim-app/src/main.ts
```

Expected: `elohim-edge` AND `elohim` AND `elohim-genesis` all dispatched.

- [ ] **Step 6: Commit**

```bash
git add genesis/orchestrator/Jenkinsfile
git commit -m "fix(orchestrator): genesis auto-include only on (genesis-touched OR full-deploy shape)"
```

---

### Task 1.2: Split storage out of elohim-edge changePatterns (per-image selectivity)

**Background:** `elohim-edge.changePatterns` is a superset of storage's source paths. Touching `elohim/elohim-storage/` rebuilds all 6 container images (Doorway, Doorway App, Agent SDK, Storage, edgenode, happ-installer) when only Storage changes. The edge Jenkinsfile already has `shouldRunStep('cargo-build-storage')` — extend the pattern to skip Docker image builds whose sources are unchanged.

**Files:**
- Modify: `elohim/holochain/Jenkinsfile` (Build Storage / Build Doorway / Build Agent SDK stages)
- Modify: `genesis/orchestrator/Jenkinsfile` (pass `CHANGED_PATHS` env to edge)

- [ ] **Step 1: Identify per-image source roots**

```bash
grep -n "^FROM\|^COPY" elohim/holochain/*.Dockerfile elohim/elohim-storage/Dockerfile 2>/dev/null | head -40
```

Document which files each Dockerfile's `COPY` ingests. Record in this checklist:

| Image | Source root(s) |
|---|---|
| Storage | `elohim/elohim-storage/`, `elohim/sdk/storage-client-ts/` (subset) |
| Doorway | `doorway/doorway-service/`, `crates/` |
| Doorway App | `doorway/doorway-app/` |
| Agent SDK | `elohim/elohim-agent/elohim-agent-sdk/` |
| edgenode | `elohim/holochain/edgenode/`, plus DNA `.happ` artifact |
| happ-installer | `elohim/holochain/dna/elohim/workdir/` |

- [ ] **Step 2: Add per-image change detection in orchestrator**

In `genesis/orchestrator/Jenkinsfile`, when dispatching `elohim-edge`, pass the changeset as a job param:

```groovy
def edgeChangedPaths = changedFiles.join(',')
build job: 'elohim-edge/dev',
      parameters: [
        string(name: 'UPSTREAM_CHANGED_PATHS', value: edgeChangedPaths),
        // ... existing params ...
      ],
      wait: false
```

- [ ] **Step 3: In edge Jenkinsfile, derive a per-image build map**

Add at the top of `elohim/holochain/Jenkinsfile` (helper methods section):

```groovy
def imageNeedsBuild(String imageName, String changedPathsCsv) {
  if (!changedPathsCsv) return true  // unknown — build everything
  def changed = changedPathsCsv.split(',')
  def imagePathPrefixes = [
    'storage':         ['elohim/elohim-storage/', 'elohim/sdk/storage-client-ts/'],
    'doorway':         ['doorway/doorway-service/', 'crates/'],
    'doorway-app':     ['doorway/doorway-app/'],
    'agent-sdk':       ['elohim/elohim-agent/elohim-agent-sdk/'],
    'edgenode':        ['elohim/holochain/edgenode/', 'elohim/holochain/dna/'],
    'happ-installer':  ['elohim/holochain/dna/elohim/workdir/'],
  ]
  def prefixes = imagePathPrefixes[imageName] ?: []
  return changed.any { f -> prefixes.any { p -> f.startsWith(p) } }
}
```

- [ ] **Step 4: Gate each Build stage**

For each `stage('Build Storage')`, `stage('Build Doorway')`, etc., wrap the body:

```groovy
stage('Build Storage') {
  when { expression { imageNeedsBuild('storage', params.UPSTREAM_CHANGED_PATHS ?: '') } }
  steps {
    // existing body
  }
}
```

Add the same `when` guard to all 6 build stages.

- [ ] **Step 5: Validate against build #996's changeset**

```bash
# Simulate build #996's actual changeset (storage-only)
just -d genesis/orchestrator ci-preview elohim/elohim-storage/src/views.rs
```

Expected: trajectory reports show `elohim-edge` runs in <20 min (storage rebuild only) versus the historical 60+ min full-stack rebuild.

- [ ] **Step 6: Commit**

```bash
git add genesis/orchestrator/Jenkinsfile elohim/holochain/Jenkinsfile
git commit -m "feat(orchestrator,edge): per-image change-gating — storage-only push rebuilds storage image only"
```

---

### Task 1.3: Cascade-skip downstream pipelines when upstream baseline failed

**Background:** Per memory `_cascade_halt_masks_failures` — currently the orchestrator dispatches all downstream pipelines and lets each fail independently. If DNA failed on this baseline already, app rebuild is moot. Add a "skip-if-upstream-failed" check that consults `pipeline-baselines.json` artifacts before dispatching.

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile` (dispatch logic, near `applyBuildGraphRouting`)

- [ ] **Step 1: Identify upstream-failure lookup point**

```bash
grep -n "applyBuildGraphRouting\|build job:" genesis/orchestrator/Jenkinsfile | head -20
```

- [ ] **Step 2: Add a `lastBaselineResult(pipelineName, sha)` helper**

In the Jenkinsfile helper methods section, add:

```groovy
def lastBaselineResult(String pipelineName, String sha) {
  // Returns the Result of the most recent build of this pipeline whose
  // archived pipeline-baselines.json __global__ matches `sha`. null if
  // no such build exists.
  try {
    def job = Jenkins.instance.getItemByFullName("${pipelineName}/${env.BRANCH_NAME}")
    if (!job) return null
    for (run in job.builds.limit(20)) {
      def artifact = run.getArtifacts().find { it.relativePath == 'pipeline-baselines.json' }
      if (!artifact) continue
      def json = new groovy.json.JsonSlurper().parse(artifact.file)
      if (json['__global__'] == sha) return run.result?.toString()
    }
  } catch (e) {
    echo "[orchestrator] lastBaselineResult lookup failed for ${pipelineName}: ${e.message}"
  }
  return null
}
```

- [ ] **Step 3: Skip downstream dispatch when upstream failed on same baseline**

Wrap each `build job: '<downstream>/dev'` call:

```groovy
def upstreamResult = lastBaselineResult('elohim-holochain', baselineSha)
if (upstreamResult == 'FAILURE') {
  echo "[orchestrator] SKIP elohim-edge: elohim-holochain FAILED on baseline ${baselineSha.take(8)}"
  // Mark our own build UNSTABLE to surface the skip
  currentBuild.result = 'UNSTABLE'
} else {
  build job: 'elohim-edge/dev', ...
}
```

Apply this pattern to any pipeline whose `dependsOn` failed on the same baseline.

- [ ] **Step 4: Smoke-test in shadow run**

```bash
# Trigger an orchestrator run after a known-failing DNA on baseline X
# (operator-driven, no automation here — log results in journal)
```

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/Jenkinsfile
git commit -m "feat(orchestrator): cascade-skip downstream pipelines when upstream FAILED on same baseline"
```

---

## Phase 2 — Concurrency model

Eliminate the supersede-waste (30% ABORTED rate on holochain pipeline) by making downstream pipelines abort-on-newer, matching the orchestrator's existing semantics.

### Task 2.1: Add `abortPrevious: true` to downstream pipelines

**Files:**
- Modify: `elohim/holochain/Jenkinsfile` (around line 812)
- Modify: `elohim/holochain/dna/Jenkinsfile` (around line 175)
- Modify: `Jenkinsfile` (root — app pipeline)

- [ ] **Step 1: Locate `disableConcurrentBuilds` calls**

```bash
grep -rn "disableConcurrentBuilds" Jenkinsfile elohim/holochain/Jenkinsfile elohim/holochain/dna/Jenkinsfile genesis/Jenkinsfile
```

- [ ] **Step 2: Update each call**

For each downstream Jenkinsfile, change:

```groovy
disableConcurrentBuilds()
```

to:

```groovy
disableConcurrentBuilds(abortPrevious: true)
```

This mirrors the orchestrator's existing semantics — newer push aborts older queued/running build of the same job.

- [ ] **Step 3: Document the trade-off in a Jenkinsfile comment**

Above each modified line, add:

```groovy
// abortPrevious: true — when a new orchestrator dispatch arrives while a
// previous build of this pipeline is in-flight, abort the old one. Without
// this, the previous orchestrator's downstream queues up behind the in-flight
// build (60+ min wait) while the orchestrator itself has already been aborted.
// See memory: _orchestrator_abort_baseline_rollback.
disableConcurrentBuilds(abortPrevious: true)
```

- [ ] **Step 4: Validate post-merge with trajectory tool**

After this lands and 5-10 builds have occurred:

```bash
JENKINS_URL=https://jenkins.ethosengine.com node genesis/orchestrator/scripts/pipeline-trajectory.mjs --builds 15 | grep -A2 "supersede-waste"
```

Expected: `supersede-waste` pattern absent or `aborted/N` ratio dropped below the threshold.

- [ ] **Step 5: Commit**

```bash
git add Jenkinsfile elohim/holochain/Jenkinsfile elohim/holochain/dna/Jenkinsfile genesis/Jenkinsfile
git commit -m "fix(ci): abortPrevious: true on downstream pipelines — eliminate supersede-waste"
```

---

## Phase 3 — Pre-push and husky unification

Make the local pre-push hook a thin consumer of orchestrator logic so the local gate and CI agree on what changed. Eliminate the `eval` injection surface, the duplicated case arms, and the silent-empty MANIFEST_DIRS_RAW failure.

### Task 3.1: Replace `eval` with a structured graph-walker output

**Files:**
- Modify: `genesis/orchestrator/graph-walker.mjs` (add line-oriented output mode)
- Modify: `.husky/pre-push:166-183`

- [ ] **Step 1: Add a `--shell-lines` output mode to graph-walker**

In `genesis/orchestrator/graph-walker.mjs`, find the main output block. Add support for an `--shell-lines` flag that emits two TSV lines instead of JSON:

```javascript
if (process.argv.includes('--shell-lines')) {
  process.stdout.write('PROJECTS\t' + result.projects.map(p => p.name).join(' ') + '\n');
  process.stdout.write('DIRS\t' + result.projects.map(p => p.dir).join(' ') + '\n');
  process.exit(0);
}
```

- [ ] **Step 2: Replace the husky `eval` block**

In `.husky/pre-push`, replace the block from line 166 to ~183:

```sh
if command -v node >/dev/null 2>&1; then
  MANIFEST_LINES=$(echo "$CHANGED" | node genesis/orchestrator/graph-walker.mjs --shell-lines 2>/dev/null)
  if [ $? -eq 0 ] && [ -n "$MANIFEST_LINES" ]; then
    # Parse the two TSV lines without eval; whitespace-safe within the
    # well-known set of project names and directory paths.
    PROJECTS=$(echo "$MANIFEST_LINES" | awk -F'\t' '$1=="PROJECTS"{print $2}')
    MANIFEST_DIRS=$(echo "$MANIFEST_LINES" | awk -F'\t' '$1=="DIRS"{print $2}')
    if [ -n "$PROJECTS" ]; then
      USE_MANIFEST=true
    fi
  fi
fi
```

- [ ] **Step 3: Smoke test against a synthetic changeset**

```bash
# Verify graph-walker emits the new format
echo "elohim/elohim-storage/src/views.rs" | node genesis/orchestrator/graph-walker.mjs --shell-lines
```

Expected: two lines, `PROJECTS\t<names>` and `DIRS\t<dirs>`.

- [ ] **Step 4: Smoke-test pre-push with HUSKY=0 bypass-test mode**

```bash
# Run the change-detection block in isolation
bash -x -c 'CHANGED="elohim/elohim-storage/src/views.rs"; . ./.husky/pre-push 2>&1 | head -30' || true
```

(May fail at later gate steps — only verifying the change-detection portion runs without `eval` errors.)

- [ ] **Step 5: Commit**

```bash
git add genesis/orchestrator/graph-walker.mjs .husky/pre-push
git commit -m "fix(husky): replace eval-on-shell-string with --shell-lines TSV from graph-walker"
```

---

### Task 3.2: Subshell `run_gate` to fix CWD mutation

**Files:**
- Modify: `.husky/pre-push:628-666` (the loop calling `run_gate`)

- [ ] **Step 1: Wrap `run_gate` in a subshell**

Find the loop body:

```sh
run_gate "$PROJECT" "$PROJECT_DIR"
GATE_EXIT=$?
```

Replace with:

```sh
(run_gate "$PROJECT" "$PROJECT_DIR")
GATE_EXIT=$?
```

The parentheses run `run_gate` in a subshell, so any `cd` inside it cannot leak to subsequent iterations.

- [ ] **Step 2: Remove the now-unneeded `cd "$OLDPWD"`**

In `run_gate()`, delete the line:

```sh
cd "$OLDPWD" || true
```

It was a workaround for the missing subshell isolation.

- [ ] **Step 3: Smoke test by triggering two consecutive gates**

```bash
# Synthetic: stage a file in two projects and run the loop
git diff --name-only HEAD | head -1
# Then run a dry pre-push by setting GIT_PUSH_DRYRUN if available, or
# inspect manually by reading the FAILED/RESULTS output after a real push.
```

- [ ] **Step 4: Commit**

```bash
git add .husky/pre-push
git commit -m "fix(husky): subshell run_gate to isolate cwd mutation across project iterations"
```

---

### Task 3.3: Update storybook fallback grep to match registry globs

**Files:**
- Modify: `.husky/pre-push` (the fallback grep block, around lines 219-243)

- [ ] **Step 1: Add the missing globs**

Find:

```sh
if echo "$CHANGED" | grep -q "^app/elohim-library/"; then
  PROJECTS="$PROJECTS elohim-library"
fi
```

Add immediately after (a separate, dedicated check for storybook source globs):

```sh
# elohim-storybook: registry-defined paths include graphos source content
# (genesis/docs/content/elohim-protocol/**, genesis/graphos/**, genesis/a2o/features/**)
# — keep this grep aligned with orchestrator-strategy.mjs PIPELINES['elohim-storybook'].
if echo "$CHANGED" | grep -qE "^app/elohim-library/|^genesis/docs/content/elohim-protocol/|^genesis/graphos/|^genesis/a2o/features/"; then
  PROJECTS="$PROJECTS elohim-storybook"
fi
```

- [ ] **Step 2: Commit**

```bash
git add .husky/pre-push
git commit -m "fix(husky): storybook fallback grep includes graphos + a2o feature globs (registry parity)"
```

---

### Task 3.4: Phase out duplicated fallback case arms

**Files:**
- Modify: `.husky/pre-push` (the `if command -v just` branch vs the fallback case block)

**Strategy:** For projects that already have a `justfile gate` recipe, delete the fallback case arm — the `just gate` path is the canonical local gate. Only keep fallback arms for projects without a justfile.

- [ ] **Step 1: Audit which projects have a justfile gate**

```bash
for dir in app/elohim-app app/elohim-library doorway/doorway-service doorway/doorway-app sophia elohim/elohim-storage elohim/elohim-compute elohim/epr steward/node genesis/seeder genesis/a2o; do
  if [ -f "$dir/justfile" ] && grep -q "^gate" "$dir/justfile"; then
    echo "HAS_GATE: $dir"
  else
    echo "NO_GATE:  $dir"
  fi
done
```

Record which projects have `just gate` and which still need fallback.

- [ ] **Step 2: Delete fallback case arms for projects with a justfile gate**

For each `HAS_GATE` project, delete its case arm from the fallback `case "$PROJECT_NAME" in` block (lines ~376-545). The just-driven path at line 370 (`just gate`) handles it.

Example — if `doorway` has `just gate`:

```sh
# DELETE this case arm:
doorway)
  cargo fmt --check 2>&1 && \
  RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 && \
  RUSTFLAGS="" cargo test --lib --bins 2>&1
  rc=$?
  ;;
```

- [ ] **Step 3: Add a comment explaining the boundary**

Above the remaining fallback case block:

```sh
# Fallback case arms below are for projects without a justfile gate, or
# for synthesized projects (schema-validate, constants-sync) that are
# orchestrator-defined virtual gates rather than per-project. Anything
# with a justfile MUST be invoked via `just gate` — do not duplicate logic.
```

- [ ] **Step 4: Test by triggering a gate run for each migrated project**

(Manual — push to a branch with a representative changeset; verify gates run via `just gate`.)

- [ ] **Step 5: Commit**

```bash
git add .husky/pre-push
git commit -m "refactor(husky): delete fallback case arms for projects with justfile gate"
```

---

### Task 3.5: Convert pre-push to bash with `set -o pipefail`

**Files:**
- Modify: `.husky/pre-push:1` (shebang) + add `set -o pipefail` after HUSKY=0 guard

- [ ] **Step 1: Change shebang and add pipefail**

```sh
#!/usr/bin/env bash
# Pre-push hook: Runs project-specific checks before allowing push.
# Bypass: HUSKY=0 git push  (or: git push --no-verify)

[ "${HUSKY-}" = "0" ] && { echo "pre-push: HUSKY=0 — skipping all gates"; exit 0; }

# Treat pipe failures as the pipeline's exit code. -e is intentionally NOT
# set because many existing gate paths use `command || handle_failure`
# patterns; -u is unsafe given the optional toolchain vars (NVM_DIR etc.).
set -o pipefail
```

- [ ] **Step 2: Audit for pipe-failure regressions**

```bash
grep -n "|" .husky/pre-push | grep -v "||" | head -20
```

Spot-check any pipes that previously masked failures — most of the `echo "$CHANGED" | grep -qE "..."` patterns are intentional and unaffected by pipefail.

- [ ] **Step 3: Smoke-test by triggering a no-op push (then aborting)**

```bash
echo "" | bash .husky/pre-push 2>&1 | head -5
```

Expected: exits 0 silently (no changes detected).

- [ ] **Step 4: Commit**

```bash
git add .husky/pre-push
git commit -m "fix(husky): bash + set -o pipefail — surface pipe failures in change-detection"
```

---

## Phase 4 — Trajectory tool maturation

Make the trajectory tool the team's primary CI lens: fast enough to run every shift, surfacing registry-vs-cluster drift, with tunable thresholds.

### Task 4.1: Index-based upstream-build lookup (O(N+M) not O(N×M))

**Files:**
- Modify: `genesis/orchestrator/scripts/pipeline-trajectory.mjs` (around `pipelineBuildForOrchestrator`)

- [ ] **Step 1: Replace the scan with a pre-built index**

Find:

```javascript
function pipelineBuildForOrchestrator(orchestratorBuild, pipelineBuilds, orchestratorJob = 'elohim-orchestrator') {
  for (const pb of pipelineBuilds) {
    const causes = (pb.actions || []).flatMap(a => a.causes || []);
    for (const c of causes) {
      if (c.upstreamProject && c.upstreamProject.includes(orchestratorJob) && c.upstreamBuild === orchestratorBuild.number) {
        return pb;
      }
    }
  }
  return null;
}
```

Replace with:

```javascript
function buildUpstreamIndex(pipelineBuilds, orchestratorJob = 'elohim-orchestrator') {
  const index = new Map();  // upstreamBuildNumber -> pipelineBuild
  for (const pb of pipelineBuilds) {
    const causes = (pb.actions || []).flatMap(a => a.causes || []);
    for (const c of causes) {
      if (c.upstreamProject && c.upstreamProject.includes(orchestratorJob)) {
        index.set(c.upstreamBuild, pb);
      }
    }
  }
  return index;
}
```

- [ ] **Step 2: Update the call site in `main()`**

Find:

```javascript
const rows = orchBuilds.map(ob => {
  const downstream = {};
  for (const job of PIPELINES) {
    downstream[job] = pipelineBuildForOrchestrator(ob, pipelineBuildLists[job] || []);
  }
  ...
});
```

Replace with:

```javascript
// Build an index per pipeline once, then O(1) lookup per orchestrator build.
const pipelineIndexes = Object.fromEntries(
  PIPELINES.map(job => [job, buildUpstreamIndex(pipelineBuildLists[job] || [])]),
);
const rows = orchBuilds.map(ob => {
  const downstream = {};
  for (const job of PIPELINES) {
    downstream[job] = pipelineIndexes[job].get(ob.number) || null;
  }
  ...
});
```

- [ ] **Step 3: Smoke test**

```bash
time JENKINS_URL=https://jenkins.ethosengine.com node genesis/orchestrator/scripts/pipeline-trajectory.mjs --builds 20
```

Expected: total wall-time dominated by network I/O, not local processing.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/scripts/pipeline-trajectory.mjs
git commit -m "perf(trajectory): index-based upstream lookup (O(N+M) not O(N×M))"
```

---

### Task 4.2: Tunable thresholds as named constants

**Files:**
- Modify: `genesis/orchestrator/scripts/pipeline-trajectory.mjs` (top of file)

- [ ] **Step 1: Define the thresholds block**

Insert after the existing CLI-arg block:

```javascript
// ─── pattern thresholds ───────────────────────────────────────────────────
// Tuned for a 10-build window. Adjust if --builds is wildly different.
const PATTERN_THRESHOLDS = {
  // persistent-failure: at least this fraction of completed builds failed
  persistentFailureFraction: 0.5,
  // supersede-waste: require BOTH an absolute count AND a fraction of the window
  supersedeWasteAbsolute: 3,
  supersedeWasteFraction: 0.3,
  // orchestrator-failure-streak: zero successes in window when ≥ this many built
  orchestratorStreakMinCompleted: 3,
  // baseline-drift: distinct baselines in window
  baselineDriftMin: 5,
};
```

- [ ] **Step 2: Use the constants in pattern detection**

Replace each magic number in the pattern-detection block with the corresponding `PATTERN_THRESHOLDS.*` reference.

Example — supersede-waste:

```javascript
const aborted = t.stream.filter(s => isWasted(s.result)).length;
if (
  aborted >= PATTERN_THRESHOLDS.supersedeWasteAbsolute &&
  aborted / t.stream.length >= PATTERN_THRESHOLDS.supersedeWasteFraction
) {
  patterns.push({ ... });
}
```

- [ ] **Step 3: Verify thresholds via `--builds 5` (window too small for supersede-waste)**

```bash
JENKINS_URL=https://jenkins.ethosengine.com node genesis/orchestrator/scripts/pipeline-trajectory.mjs --builds 5 | grep -E "patterns|supersede"
```

Expected: supersede-waste does NOT fire on a 5-build window with 1-2 aborts (because the absolute threshold is 3).

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/scripts/pipeline-trajectory.mjs
git commit -m "fix(trajectory): named thresholds + supersede-waste requires both absolute and fraction"
```

---

### Task 4.3: Add registry-vs-cluster drift detector

**Files:**
- Create: `genesis/orchestrator/scripts/registry-cluster-drift.mjs`

**Background:** The live trajectory run revealed `elohim-epr/dev` returns 404 — the pipeline is in `orchestrator-strategy.mjs` but the Jenkins job doesn't exist. This kind of drift was previously invisible. The drift detector enumerates the registry and probes Jenkins for each.

- [ ] **Step 1: Create the drift detector**

Create `genesis/orchestrator/scripts/registry-cluster-drift.mjs`:

```javascript
#!/usr/bin/env node
/**
 * Registry vs Cluster Drift Detector
 *
 * For each pipeline in orchestrator-strategy.mjs PIPELINES, probe its
 * Jenkins job and report:
 *   - registry-only: declared in PIPELINES but no Jenkins job exists
 *   - cluster-only: Jenkins job exists but not in PIPELINES (orphan)
 *
 * Exit codes:
 *   0 — no drift
 *   1 — drift detected
 *   2 — JENKINS_URL not set or fetch error
 *
 * Run: JENKINS_URL=... node registry-cluster-drift.mjs [--branch dev]
 */

import process from 'node:process';
import { PIPELINES } from '../orchestrator-strategy.mjs';

const JENKINS_URL = process.env.JENKINS_URL;
if (!JENKINS_URL) { console.error('FATAL: JENKINS_URL must be set'); process.exit(2); }

const BRANCH = (() => {
  const i = process.argv.indexOf('--branch');
  return i >= 0 ? process.argv[i + 1] : 'dev';
})();

async function jobExists(name) {
  const res = await fetch(`${JENKINS_URL}/job/${name}/job/${BRANCH}/api/json?tree=name`);
  return res.status === 200;
}

async function listOrchestratorChildJobs() {
  // The orchestrator's downstream jobs are top-level Jenkins jobs prefixed `elohim-`.
  const res = await fetch(`${JENKINS_URL}/api/json?tree=jobs[name]`);
  if (!res.ok) throw new Error(`${res.status} on /api/json`);
  const data = await res.json();
  return (data.jobs || []).map(j => j.name).filter(n => n.startsWith('elohim-'));
}

async function main() {
  const registryNames = new Set(Object.keys(PIPELINES));
  const clusterNames = new Set(await listOrchestratorChildJobs());

  const registryOnly = [];
  for (const name of registryNames) {
    if (!(await jobExists(name))) registryOnly.push(name);
  }
  const clusterOnly = [...clusterNames].filter(n => !registryNames.has(n) && n !== 'elohim-orchestrator');

  console.log('# Registry vs Cluster Drift');
  console.log('');
  console.log(`Registry pipelines:      ${registryNames.size}`);
  console.log(`Cluster (elohim-*) jobs: ${clusterNames.size}`);
  console.log('');

  if (registryOnly.length === 0 && clusterOnly.length === 0) {
    console.log('✓ No drift — registry and cluster agree.');
    process.exit(0);
  }
  if (registryOnly.length > 0) {
    console.log('Registry-only (no Jenkins job on this branch):');
    for (const n of registryOnly) console.log(`  - ${n}`);
    console.log('');
  }
  if (clusterOnly.length > 0) {
    console.log('Cluster-only (Jenkins job exists but not in registry):');
    for (const n of clusterOnly) console.log(`  - ${n}`);
  }
  process.exit(1);
}

main().catch(e => { console.error('ERROR:', e.message); process.exit(2); });
```

- [ ] **Step 2: Make executable and run**

```bash
chmod +x genesis/orchestrator/scripts/registry-cluster-drift.mjs
node genesis/orchestrator/scripts/registry-cluster-drift.mjs
```

Expected: surfaces `elohim-epr` (and any other) as registry-only.

- [ ] **Step 3: Wire to trajectory tool's pattern section**

In `pipeline-trajectory.mjs`, when a pipeline's row is all 404/unreachable, emit a hint:

```javascript
// In the patterns block:
for (const job of PIPELINES) {
  const unreachable = pipelineBuildLists[job]?.length === 0;
  if (unreachable) {
    patterns.push({
      kind: 'registry-cluster-drift',
      pipeline: job,
      rate: 'no Jenkins job',
      note: 'pipeline declared in registry but Jenkins job unreachable — run scripts/registry-cluster-drift.mjs',
    });
  }
}
```

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/scripts/registry-cluster-drift.mjs genesis/orchestrator/scripts/pipeline-trajectory.mjs
git commit -m "feat(orchestrator): registry-cluster-drift detector + trajectory surfaces drift hint"
```

---

### Task 4.4: Stage-level trajectory (per-build stage roll-up + stage-level patterns)

**Background:** Pipeline-level results say `elohim-edge ✗ #982 41m` but hide *which* stage broke. When a 121-min build fails on stage 7-of-12 and 6 of those stages were green, "edge is red" is the wrong granularity to act on. Stage-level signal is what lets a developer (or shift operator) know whether the failure is in their changeset's surface area or somewhere ambient. Stages use the same SUCCESS/UNSTABLE/FAILURE/ABORTED vocabulary as builds, so `pipeline-results.mjs` already classifies them correctly — the change is purely additive: fetch stage data, render stage trajectories, run pattern detection at stage granularity.

Jenkins exposes stage data via the `wfapi` plugin: `/job/<job>/job/<branch>/<build>/wfapi/describe` returns the stage list with per-stage result + duration.

**Files:**
- Modify: `genesis/orchestrator/scripts/pipeline-trajectory.mjs` (add stage fetching + rendering — uses the existing `jen()` helper, no new module needed)

- [ ] **Step 1: Add a stage fetcher**

In `pipeline-trajectory.mjs`, add:

```javascript
/**
 * Returns the stages of a build, each with { name, status, durationMillis }.
 * status is SUCCESS, UNSTABLE, FAILED, ABORTED, NOT_EXECUTED, IN_PROGRESS, or PAUSED_PENDING_INPUT.
 * Returns [] if wfapi unavailable or the build has no stages.
 */
async function getBuildStages(jobName, branch, buildNumber) {
  try {
    const data = await jen(
      `/job/${jobName}/job/${branch}/${buildNumber}/wfapi/describe`,
    );
    return (data.stages || []).map(s => ({
      name: s.name,
      // wfapi uses FAILED (past tense); normalize to FAILURE for consistency
      // with pipeline-results.mjs vocabulary.
      result: s.status === 'FAILED' ? 'FAILURE'
            : s.status === 'NOT_EXECUTED' ? 'NOT_BUILT'
            : s.status === 'IN_PROGRESS' ? null
            : s.status,
      duration: s.durationMillis,
    }));
  } catch (e) {
    console.error(`WARN: stages for ${jobName}#${buildNumber} unreachable (${e.message})`);
    return [];
  }
}
```

- [ ] **Step 2: Add `--with-stages` flag**

In the arg-parsing block:

```javascript
const WITH_STAGES = argFlag('--with-stages');
```

(Off by default — N×M additional fetches if always on.)

- [ ] **Step 3: Augment row composition with stage data**

In `main()`, after the rows are composed, conditionally fetch stages for each downstream build:

```javascript
if (WITH_STAGES) {
  await Promise.all(
    rows.flatMap(r =>
      PIPELINES.flatMap(job => {
        const pb = r.downstream[job];
        if (!pb || pb.result == null) return [];
        return [
          getBuildStages(job, BRANCH, pb.number).then(stages => {
            pb.stages = stages;
          }),
        ];
      }),
    ),
  );
}
```

- [ ] **Step 4: Render per-stage trajectory section**

After the existing `## per-pipeline trajectory` block, add (gated on `WITH_STAGES`):

```javascript
if (WITH_STAGES) {
  console.log('');
  console.log('## per-stage trajectory (within each pipeline, most recent first)');
  for (const job of PIPELINES) {
    // Collect stages across the row stream, keyed by stage name.
    const stagesByName = new Map();
    for (const r of rows) {
      const pb = r.downstream[job];
      if (!pb?.stages) continue;
      for (const s of pb.stages) {
        if (!stagesByName.has(s.name)) stagesByName.set(s.name, []);
        stagesByName.get(s.name).push(s);
      }
    }
    if (stagesByName.size === 0) continue;
    console.log(`  ── ${shortPipelineName(job)} ──`);
    for (const [name, history] of stagesByName) {
      const stream = history.map(s => resultGlyph(s.result)).join(' ');
      const completed = history.filter(s => s.result != null);
      const success = completed.filter(s => isSuccess(s.result)).length;
      const rate = completed.length === 0 ? '0/0' : `${success}/${completed.length}`;
      const maxDur = Math.max(...history.map(s => s.duration ?? 0));
      console.log(`    ${pad(name, 32)} ${pad(stream, 24)}  rate=${rate}  max=${durationMin(maxDur)}`);
    }
  }
}
```

- [ ] **Step 5: Add stage-level pattern detection**

In the patterns block, add (gated on `WITH_STAGES`):

```javascript
if (WITH_STAGES) {
  // Persistent-failure at stage granularity — far more actionable than
  // pipeline-level because it names the gating leg.
  for (const job of PIPELINES) {
    const stagesByName = new Map();
    for (const r of rows) {
      const pb = r.downstream[job];
      if (!pb?.stages) continue;
      for (const s of pb.stages) {
        if (!stagesByName.has(s.name)) stagesByName.set(s.name, []);
        stagesByName.get(s.name).push(s);
      }
    }
    for (const [name, history] of stagesByName) {
      const completed = history.filter(s => s.result != null);
      const failures = completed.filter(s => isFailure(s.result)).length;
      if (
        completed.length >= 3 &&
        failures / completed.length >= PATTERN_THRESHOLDS.persistentFailureFraction
      ) {
        patterns.push({
          kind: 'persistent-stage-failure',
          pipeline: job,
          stage: name,
          rate: `${failures}/${completed.length}`,
          note: `stage '${name}' in ${shortPipelineName(job)} failed in more than half of recent builds — gating leg`,
        });
      }
    }
  }

  // Stage-duration outliers — surface stages that take >30% of total build time.
  for (const job of PIPELINES) {
    for (const r of rows) {
      const pb = r.downstream[job];
      if (!pb?.stages || !pb.duration) continue;
      for (const s of pb.stages) {
        if (s.duration && s.duration / pb.duration >= 0.3) {
          // Only emit once per stage-name to avoid noise across N rows
          const key = `${job}::${s.name}::dominant`;
          if (!patterns.some(p => p._key === key)) {
            patterns.push({
              kind: 'stage-time-dominant',
              pipeline: job,
              stage: s.name,
              rate: `${Math.round(100 * s.duration / pb.duration)}% of build`,
              note: `stage '${s.name}' dominates ${shortPipelineName(job)} wall-time — sharding/cache target`,
              _key: key,
            });
          }
        }
      }
    }
  }
}
```

- [ ] **Step 6: Smoke test against the live build #996 stages**

```bash
JENKINS_URL=https://jenkins.ethosengine.com node genesis/orchestrator/scripts/pipeline-trajectory.mjs --builds 5 --with-stages 2>&1 | tail -60
```

Expected:
- Per-stage trajectory section renders for each pipeline with stage data
- Pattern detection surfaces the actual gating stage in `elohim-edge` (e.g., "Build Storage" or "Quality Gate: Doorway" if those are the persistent failure points)
- `stage-time-dominant` flags any stage that's >30% of pipeline wall-time

- [ ] **Step 7: Update the doc-comment header to mention --with-stages**

In the top comment block, add:

```javascript
 *   --with-stages         additionally fetch per-stage results via wfapi —
 *                          surfaces persistent-stage-failure (which stage is
 *                          the gating leg) and stage-time-dominant (which
 *                          stage is consuming the build budget). Extra
 *                          fetches; default off.
```

- [ ] **Step 8: Commit**

```bash
git add genesis/orchestrator/scripts/pipeline-trajectory.mjs
git commit -m "feat(trajectory): --with-stages — per-stage trajectory + persistent-stage-failure / stage-time-dominant patterns"
```

---

### Task 4.5: Add `--watch` mode for shift operators

**Files:**
- Modify: `genesis/orchestrator/scripts/pipeline-trajectory.mjs`

- [ ] **Step 1: Add `--watch <seconds>` flag**

In the arg-parsing block, add:

```javascript
const WATCH_SECONDS = Number(argVal('--watch', '0'));
```

At the end of `main()`, wrap the body in a polling loop if `WATCH_SECONDS > 0`:

```javascript
async function main() {
  do {
    await renderOnce();
    if (WATCH_SECONDS > 0) {
      console.log(`\n(refresh in ${WATCH_SECONDS}s — Ctrl-C to exit)\n`);
      await new Promise(r => setTimeout(r, WATCH_SECONDS * 1000));
      console.clear();
    }
  } while (WATCH_SECONDS > 0);
}
```

(Extract the existing body into `renderOnce()`.)

- [ ] **Step 2: Smoke-test**

```bash
JENKINS_URL=https://jenkins.ethosengine.com node genesis/orchestrator/scripts/pipeline-trajectory.mjs --builds 5 --watch 30
```

Expected: refreshes every 30s, clears screen, continues until Ctrl-C.

- [ ] **Step 3: Commit**

```bash
git add genesis/orchestrator/scripts/pipeline-trajectory.mjs
git commit -m "feat(trajectory): --watch <seconds> polling mode for shift operators"
```

---

## Phase 5 — Docker cache structure

### Task 5.1: Remove `--no-cache` from edge storage build

**Files:**
- Modify: `elohim/holochain/Jenkinsfile:1149`

- [ ] **Step 1: Verify `CACHE_BUST` is in place**

```bash
grep -n "CACHE_BUST\|--no-cache" elohim/holochain/Jenkinsfile | head -10
grep -n "ARG CACHE_BUST\|\\${CACHE_BUST}" elohim/elohim-storage/Dockerfile 2>/dev/null
```

If `CACHE_BUST` is consumed by the Dockerfile's cargo build RUN line, `--no-cache` is redundant. If not, add it first.

- [ ] **Step 2: Replace the build invocation**

Find:

```groovy
nerdctl build --no-cache --build-arg CACHE_BUST=${env.GIT_COMMIT_HASH} ...
```

Replace with:

```groovy
nerdctl build --build-arg CACHE_BUST=${env.GIT_COMMIT_HASH} ...
```

- [ ] **Step 3: Add a comment**

```groovy
// CACHE_BUST=${GIT_COMMIT_HASH} forces re-execution of the cargo build
// layer when source changes. BuildKit retains layer cache for unchanged
// COPY+RUN pairs (base image, apt installs, cargo deps), which is what
// we want — removing --no-cache restores that ~15 min savings on rebuilds
// with no source change.
```

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/Jenkinsfile
git commit -m "perf(edge): remove --no-cache from storage build — CACHE_BUST already invalidates the cargo layer"
```

---

### Task 5.2: Restructure Doorway Dockerfile for cargo layer cache

**Files:**
- Modify: `doorway/doorway-service/Dockerfile` (or wherever the doorway Dockerfile lives)

- [ ] **Step 1: Identify the Dockerfile**

```bash
find doorway/ -name "Dockerfile" | head -5
```

- [ ] **Step 2: Apply the standard Rust cargo layer pattern**

The pattern: copy `Cargo.toml` + `Cargo.lock` + create empty `src/main.rs`, run `cargo build --release` to compile deps, THEN copy real source, THEN compile.

```dockerfile
# ── Cargo dependency cache layer ─────────────────────────────────────
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
# Optional workspace members: copy their Cargo.toml only
COPY crates/ crates/
RUN mkdir -p src && echo 'fn main(){}' > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/deps/doorway*

# ── Real source ──────────────────────────────────────────────────────
COPY src/ src/
RUN cargo build --release
```

(Adjust `doorway*` to match the actual binary name.)

- [ ] **Step 3: Validate with two consecutive local builds**

```bash
# Build #1
time nerdctl build --build-arg CACHE_BUST=dummy -f doorway/doorway-service/Dockerfile -t doorway-test:1 .
# Touch a source file
touch doorway/doorway-service/src/main.rs
# Build #2 — should be much faster (deps cached)
time nerdctl build --build-arg CACHE_BUST=different -f doorway/doorway-service/Dockerfile -t doorway-test:2 .
```

Expected: build #2 wall-time at least 5 min faster than build #1.

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/Dockerfile
git commit -m "perf(doorway): two-stage cargo layer — deps cache survives source changes"
```

---

## Phase 6 — DNA sweettest sharding

### Task 6.1: Categorize sweettests by DNA group

**Files:**
- Read: `elohim/holochain/tests/sweettest/tests/` (recursive)
- Create: `elohim/holochain/tests/sweettest/SHARDS.md` (documentation artifact)

- [ ] **Step 1: List sweettest source files**

```bash
find elohim/holochain/tests/sweettest/tests -name "*.rs" | sort
```

- [ ] **Step 2: Group by DNA touched**

For each test file, identify whether it primarily exercises lamad, imagodei/infrastructure, or mishpat. Document in `SHARDS.md`:

```markdown
# Sweettest Shards

## Shard A — lamad
- tests/lamad_*.rs
- tests/content_addressing_*.rs

## Shard B — imagodei + infrastructure
- tests/imagodei_*.rs
- tests/infrastructure_*.rs
- tests/auth_*.rs

## Shard C — mishpat + cross-cutting
- tests/mishpat_*.rs
- tests/governance_*.rs
- tests/two_agent_*.rs   # cross-DNA scenarios
```

- [ ] **Step 3: Commit the documentation**

```bash
git add elohim/holochain/tests/sweettest/SHARDS.md
git commit -m "docs(sweettest): shard map — A/B/C by DNA group"
```

---

### Task 6.2: Split DNA Jenkinsfile sweettest stage into parallel shards

**Files:**
- Modify: `elohim/holochain/dna/Jenkinsfile` (the sweettest stage)

- [ ] **Step 1: Find the existing stage**

```bash
grep -n "stage('Sweettest\|cargo nextest" elohim/holochain/dna/Jenkinsfile | head -10
```

- [ ] **Step 2: Replace single-stage with parallel shards**

Replace the existing sweettest stage with:

```groovy
stage('Sweettest (parallel shards)') {
  parallel {
    stage('Shard A — lamad') {
      steps {
        sh '''
          cargo nextest run \
            --test-threads=1 \
            --filter-expr 'test(/lamad_|content_addressing_/)'
        '''
      }
    }
    stage('Shard B — imagodei + infrastructure') {
      steps {
        sh '''
          cargo nextest run \
            --test-threads=1 \
            --filter-expr 'test(/imagodei_|infrastructure_|auth_/)'
        '''
      }
    }
    stage('Shard C — mishpat + cross-cutting') {
      steps {
        sh '''
          cargo nextest run \
            --test-threads=1 \
            --filter-expr 'test(/mishpat_|governance_|two_agent_/)'
        '''
      }
    }
  }
}
```

(Confirm `--filter-expr` syntax against the project's nextest version; some versions use `-E` or regex syntax.)

- [ ] **Step 3: Validate with a shadow build**

(Operator-driven — trigger the DNA pipeline with this change on a feature branch, measure wall-time delta.)

Expected: stage wall-time drops from ~90 min to ~35-40 min (limited by the slowest shard).

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/Jenkinsfile
git commit -m "perf(dna): split sweettest into 3 parallel shards (A/B/C) — wall-time bound by slowest"
```

---

## Phase 7 — Genesis stage parallelization

### Task 7.1: Parallelize independent genesis setup stages

**Files:**
- Modify: `genesis/Jenkinsfile` (stages around lines 571-756)

- [ ] **Step 1: Identify the independent stages**

```bash
grep -n "^[[:space:]]*stage(" genesis/Jenkinsfile | head -20
```

The first 4 stages (Install Seeder, Generate Schema Types, Validate Constants, Verify Target Health) split cleanly:

- Branch A (CPU-bound): Install Seeder → Generate Schema Types → Validate Constants
- Branch B (network-bound): Verify Target Health (polls doorway/health)

- [ ] **Step 2: Wrap in `parallel`**

Replace the four sequential stages with:

```groovy
stage('Setup (parallel)') {
  parallel {
    stage('Install + Schema + Validate') {
      stages {
        stage('Install Seeder')       { steps { /* existing body */ } }
        stage('Generate Schema Types'){ steps { /* existing body */ } }
        stage('Validate Constants')   { steps { /* existing body */ } }
      }
    }
    stage('Verify Target Health') {
      steps { /* existing body */ }
    }
  }
}
```

- [ ] **Step 3: Validate Branch B fail-fast**

In Verify Target Health, add a fast-fail check:

```groovy
// If the first 3 probes return 000 (connection refused), the doorway pod
// isn't starting — fail immediately instead of polling for 2 minutes.
def initial = 0
for (int i = 0; i < 3; i++) {
  def code = sh(returnStdout: true, script: "curl -s -o /dev/null -w '%{http_code}' ${DOORWAY_HEALTH_URL} || echo 000").trim()
  if (code == '000') initial++
  sleep 2
}
if (initial == 3) {
  error("Doorway pod not starting: 3/3 initial probes returned connection-refused")
}
// ... existing polling loop ...
```

- [ ] **Step 4: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "perf(genesis): parallelize setup stages + fast-fail health probe"
```

---

## Self-Review

**Spec coverage:** Each phase maps to one of the user's stated concerns:
- Phase 0 → "no clear trajectory between pipeline runs" + "two systems counting failures differently"
- Phase 1 → "1hr long integrations run 4 times in a row" (cascade containment)
- Phase 2 → trajectory's `supersede-waste 3/10` pattern (concurrency model)
- Phase 3 → "close the result resolution gaps between our local build-graph and husky processes"
- Phase 4 → "CI as extension of development" (instrumentation as primary lens)
- Phase 5 → Docker cache wastage (storage rebuild from scratch)
- Phase 6 → DNA integration wall-time
- Phase 7 → genesis stage parallelism

**Placeholder scan:** All steps have either exact code or exact shell commands. Phase 1 Task 1.2 requires reading the actual Dockerfile contents to confirm the source-root mapping — that's enumerated in Step 1 with the grep command to discover them. Phase 6 Task 6.1 requires the implementer to read and categorize tests — that's an inherent property of the task, not a placeholder.

**Type consistency:** `nonManualPipelines()` is defined in Task 0.1, consumed in Task 0.3. `pipeline-results.mjs` exports (`SUCCESSFUL_RESULTS`, `classifyResult`, `isSuccess`, `isFailure`, `isWasted`) are defined in Task 0.2 and consumed in Tasks 0.3 + 0.4. `pipeline-list.json` schema (Task 0.5) is consumed by Task 0.6 (`jq -r '.pipelines[] | select(.manualOnly | not) | .name'`).

**Risk notes (operator should read before executing):**

1. **Phase 2 — `abortPrevious: true`** can lose nearly-complete builds. Mitigation: per-pipeline opt-in; deploy in order [holochain, edge, app, genesis] with 24h soak between to verify supersede-waste drops without introducing new flake.
2. **Phase 1 Task 1.3** (cascade-skip on upstream-failed) introduces a NEW way for the orchestrator to mark itself UNSTABLE. Verify this doesn't trip the `_pre_dispatch_hard_fail_post_dispatch_unstable` invariant — the cascade-skip happens at dispatch time, so should be observational and UNSTABLE-marking, not hard-fail.
3. **Phase 5 Task 5.2** (Doorway cargo layer restructure) — the dummy-src trick can leave a stale `target/release/deps/doorway*` if not cleaned. The `rm -rf` in Step 2 handles this; verify after first successful local build.
4. **Phase 6 Task 6.2** (sweettest shards) — three parallel `cargo nextest` calls means 3× memory pressure on the agent. Verify the Kubernetes pod has resource headroom OR limit concurrency via Jenkins agent labels.

---

## Execution Handoff

Plan complete and saved to `genesis/docs/plans/2026-05-22-ci-cd-resolution-gap-close.md`.

**Recommended sequencing for operators:**
1. Phase 0 (foundation) — single PR, no production risk, all tests covered locally
2. Phase 3 (pre-push) — landed as standalone PR, exercised by every push from then on
3. Phase 2 (concurrency) — one downstream at a time, 24h soak each
4. Phase 1 Task 1.1 (genesis auto-include) — high ROI, low risk
5. Phase 4 (trajectory maturation) — independent
6. Phase 1 Task 1.2 (per-image selectivity) — needs Dockerfile audit
7. Phase 5, 6, 7 in any order

Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — execute tasks in this session with batch checkpoints.

Which approach?
