# Deprecate Legacy PIPELINES Algorithm

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the manifest-driven build graph the sole source of truth for change detection and pipeline triggering, eliminating the legacy PIPELINES algorithm and its baseline-skew divergences.

**Architecture:** The orchestrator currently runs two parallel algorithms: the legacy `PIPELINES` map + `analyzeChangeset()` (controls execution) and the manifest-driven build graph (shadow mode, observability only). We flip the build graph to primary, keep the legacy algorithm as a warning-only cross-check during a transition window, then remove it entirely.

**Tech Stack:** Groovy (Jenkinsfile + build-graph.groovy), JavaScript (orchestrator-strategy.mjs/test.mjs), JSON (build-manifest.json)

---

## Context

### Why the legacy algorithm causes problems

The legacy `PIPELINES` algorithm in `genesis/orchestrator/Jenkinsfile` (lines 29-90) uses **per-pipeline baselines** — each pipeline remembers the last commit it successfully built. The build graph uses a **global baseline** plus per-step source glob matching. When a pipeline's per-pipeline baseline gets stale (failed build, interrupted run, baseline wipe), its changeset grows to include files from older commits, causing false-positive BUILD decisions that diverge from the build graph's correct SKIP.

This is the "baseline-skew" class of divergence documented in `build-graph.groovy:589-596`. Three pipelines (`elohim`, `elohim-edge`, `elohim-genesis`) are in the known divergences list for this reason. Each time a new pipeline hits this issue, the orchestrator hard-fails until someone adds it to the allowlist.

### Current state

| Pipeline | Has Manifest? | In PIPELINES? | Status |
|----------|:---:|:---:|--------|
| elohim-holochain | Yes | Yes | Aligned |
| elohim-edge | Yes | Yes | Aligned (baseline-skew divergence known) |
| elohim | Yes | Yes | Aligned (baseline-skew divergence known) |
| elohim-genesis | Yes | Yes | Aligned (baseline-skew divergence known) |
| elohim-steward | Yes | Yes | Aligned (manualOnly) |
| elohim-sophia | **No** | Yes | Legacy-only gap |
| elohim-compute | Yes | **No** | Graph-only |
| elohim-doorway-app | Yes | **No** | Graph-only |
| elohim-orchestrator | Yes | **No** | Graph-only |

### Key files

| File | Role | Lines of interest |
|------|------|-------------------|
| `genesis/orchestrator/Jenkinsfile` | Legacy algorithm + stage orchestration | 29-90 (PIPELINES), 220-379 (analyze/match), 467-491 (cascade), 498-545 (baselines), 651-691 (shadow mode call), 894-1048 (Build Plan stage) |
| `genesis/orchestrator/build-graph.groovy` | Manifest-driven graph walker | 571-600 (known divergences), 763-834 (walkBuildGraph entry point) |
| `genesis/orchestrator/orchestrator-strategy.mjs` | JS mirror of PIPELINES for local testing | 24-276 (full mirror) |
| `genesis/orchestrator/orchestrator-strategy.test.mjs` | Tests for both algorithms | 43-603 |
| `**/build-manifest.json` | Per-pipeline manifests (8 files) | |

---

## Task 1: Add sophia build manifest

**Files:**
- Create: `sophia/build-manifest.json`
- Modify: `genesis/orchestrator/build-graph.groovy:571-600`
- Modify: `genesis/orchestrator/orchestrator-strategy.test.mjs:548-555`

The only legacy-only pipeline without a manifest. Must exist before we can retire PIPELINES.

- [ ] **Step 1: Create the sophia manifest**

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-sophia",
  "description": "Sophia assessment engine — build, test, UMD bundle",
  "steps": {
    "build-sophia-umd": {
      "description": "Build sophia-element UMD bundle for Angular consumption",
      "inputs": {
        "sources": ["sophia/"],
        "buildProcess": ["sophia.Jenkinsfile"]
      },
      "outputs": {
        "artifacts": ["sophia-element-umd"],
        "verify": "test -f sophia/packages/sophia-element/dist/sophia-element.umd.js"
      },
      "depends": [],
      "executor": {
        "stage": "Build Sophia",
        "function": null
      }
    }
  },
  "gate": {
    "projects": {
      "sophia": { "dir": "sophia", "steps": ["build-sophia-umd"] }
    }
  },
  "deployment": {}
}
```

Note: `sophia/` is a git submodule. The source glob `sophia/` will match the submodule gitlink pointer change in `git diff --name-only` output (the file is literally `sophia` with no trailing slash — the graph's `matchesGlob()` handles this via the `**` suffix behavior).

- [ ] **Step 2: Remove elohim-sophia from known divergences**

In `genesis/orchestrator/build-graph.groovy`, remove `'elohim-sophia'` from `getKnownDivergences()` (line 576).

In `genesis/orchestrator/orchestrator-strategy.test.mjs`, remove `'elohim-sophia'` from `KNOWN_DIVERGENCES` (line 549).

- [ ] **Step 3: Run the cross-validation test**

Run: `cd genesis/orchestrator && pnpm exec vitest run orchestrator-strategy.test.mjs`

Expected: All tests pass. The "PIPELINES changePatterns covered by manifest source globs" test for `elohim-sophia` should now run (no longer skipped) and pass.

- [ ] **Step 4: Commit**

```
feat(orchestrator): add sophia build manifest

Closes the last legacy-only gap — all PIPELINES entries now have
matching build-manifest.json files. Removes elohim-sophia from
known divergences.
```

---

## Task 2: Promote build graph to primary algorithm

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile:894-1048` (Determine Build Plan stage)
- Modify: `genesis/orchestrator/Jenkinsfile:651-691` (runBuildGraphShadow → runBuildGraph)
- Modify: `genesis/orchestrator/build-graph.groovy:763-834` (return pipelines-to-run)

This is the critical flip. The build graph decides which pipelines run. The legacy algorithm becomes advisory.

- [ ] **Step 1: Rename runBuildGraphShadow to runBuildGraph**

In `genesis/orchestrator/Jenkinsfile`, rename the function at line 651 from `runBuildGraphShadow` to `runBuildGraph`. Update the call site at line 1027. Update the function to return the pipeline set instead of just logging.

Current (line 651-691):
```groovy
def runBuildGraphShadow(changedFiles, analysis) {
    def buildGraph = load('genesis/orchestrator/build-graph.groovy')
    def graphResult = buildGraph.walkBuildGraph(changedFiles)
    // ... logging ...
    // Hard escalation on unexpected divergences
}
```

Change to:
```groovy
def runBuildGraph(changedFiles, analysis) {
    def buildGraph = load('genesis/orchestrator/build-graph.groovy')
    def graphResult = buildGraph.walkBuildGraph(changedFiles)

    // Log decision matrix
    echo graphResult.graph._decisionMatrix ?: ''
    def perFileMatrix = buildGraph.formatPerFileMatrix(
        graphResult.staleMap, graphResult.graph, changedFiles)
    echo perFileMatrix

    // Log comparison matrix (advisory — legacy divergences are warnings now)
    def comparison = buildGraph.formatComparisonMatrix(analysis, graphResult.staleMap, graphResult.graph)
    echo comparison.text

    if (comparison.unexpectedDivergences.size() > 0) {
        echo """
WARNING: Legacy PIPELINES algorithm disagrees with build graph for: ${comparison.unexpectedDivergences.join(', ')}
The build graph is authoritative. Legacy algorithm will be removed in a future sprint.
"""
    }

    // Save build state for next run
    buildGraph.saveBuildState(graphResult.graph, graphResult.staleMap,
        graphResult.buildProcessHashes, graphResult.previousState,
        env.GIT_COMMIT_FULL)

    return graphResult
}
```

- [ ] **Step 2: Use build graph result for pipeline selection**

In the "Determine Build Plan" stage (around line 1020-1027), change the pipeline selection from PIPELINES-driven to graph-driven:

Current:
```groovy
env.PIPELINES_TO_RUN = pipelines.join(',')
// ...
runBuildGraphShadow(changedFiles, analysis)
```

Change to:
```groovy
// Build graph is the primary algorithm
def graphResult = runBuildGraph(changedFiles, analysis)
def graphPipelines = graphResult.pipelineSteps.keySet().toList()

// Genesis auto-include on dev branches (preserve existing behavior)
if (env.BRANCH_NAME == 'dev' && graphPipelines.any { name ->
    PIPELINES[name]?.triggersGenesis
} && !graphPipelines.contains('elohim-genesis')) {
    graphPipelines.add('elohim-genesis')
}

// Manual-only pipelines never auto-trigger
graphPipelines.removeAll { name -> PIPELINES[name]?.manualOnly }

// Order by dependency levels from graph
env.PIPELINES_TO_RUN = graphPipelines.join(',')
```

- [ ] **Step 3: Keep legacy analysis for advisory comparison only**

The `analyzeChangeset()`, `analyzePipelineRequirements()`, and `propagateDependencies()` calls remain but their results only feed the comparison matrix — they no longer control `PIPELINES_TO_RUN`. Add a comment marking them as deprecated:

```groovy
// DEPRECATED: Legacy analysis — advisory only, for comparison matrix.
// Will be removed once the build graph has run successfully for 2+ weeks.
def analysis = analyzePipelineRequirements(changedFiles, pipelineBaselines)
```

- [ ] **Step 4: Update baseline archiving**

Per-pipeline baselines are no longer needed (the build graph uses per-step source matching). Simplify `archivePipelineBaselines()` to only save `__global__`:

```groovy
def archivePipelineBaselines(String checkpoint) {
    def baselines = [:]
    if (env.GIT_COMMIT_FULL) {
        baselines['__global__'] = env.GIT_COMMIT_FULL
    }
    env.PIPELINE_BASELINES = writeJSON(returnText: true, json: baselines)
    writeJSON file: 'pipeline-baselines.json', json: baselines, pretty: 2
    if (!fileExists('pipeline-baselines.json')) {
        error "pipeline-baselines.json missing after writeJSON (checkpoint=${checkpoint})"
    }
    archiveArtifacts artifacts: 'pipeline-baselines.json', fingerprint: true
    echo "📋 [baseline:${checkpoint}] archived — __global__=${baselines['__global__']?.toString()?.take(8) ?: '(null)'}"
}
```

- [ ] **Step 5: Remove baseline-skew entries from known divergences**

In `build-graph.groovy`, remove `'elohim'`, `'elohim-edge'`, `'elohim-genesis'` from `getKnownDivergences()`. With the build graph as primary, these divergences are expected and harmless — the comparison matrix is advisory.

In `orchestrator-strategy.test.mjs`, update `KNOWN_DIVERGENCES` to match.

- [ ] **Step 6: Run orchestrator tests**

Run: `cd genesis/orchestrator && pnpm exec vitest run orchestrator-strategy.test.mjs`

Expected: All tests pass.

- [ ] **Step 7: Commit**

```
feat(orchestrator): promote build graph to primary change detection algorithm

The manifest-driven build graph now controls which pipelines run.
The legacy PIPELINES algorithm remains as an advisory cross-check
(comparison matrix logs warnings instead of hard-failing on divergences).
Per-pipeline baselines simplified to global-only since the build graph
uses per-step source matching.
```

---

## Task 3: Soak period validation

**Files:** None (operational observation)

After Task 2 lands, the orchestrator runs with the build graph as primary for 1-2 weeks. During this period:

- [ ] **Step 1: Monitor orchestrator builds for correctness**

Watch for:
- False negatives: the graph says SKIP but the pipeline should have built (user reports broken deploy)
- False positives: the graph says BUILD but nothing changed (wasteful but harmless)
- The comparison matrix advisory warnings — do they correlate with actual issues?

- [ ] **Step 2: Verify graph-only pipelines trigger correctly**

`elohim-compute`, `elohim-doorway-app`, and `elohim-orchestrator` only have manifests — they never had PIPELINES entries. Confirm they trigger on relevant changes now that the graph is authoritative.

- [ ] **Step 3: Document any edge cases in a follow-up issue**

If the soak period reveals issues, create a plan for Task 4 adjustments before proceeding to Task 4.

---

## Task 4: Remove legacy algorithm

**Files:**
- Modify: `genesis/orchestrator/Jenkinsfile` — remove ~300 lines
- Modify: `genesis/orchestrator/build-graph.groovy` — remove comparison matrix and known divergences
- Modify: `genesis/orchestrator/orchestrator-strategy.mjs` — archive or delete
- Modify: `genesis/orchestrator/orchestrator-strategy.test.mjs` — remove legacy tests, keep manifest tests

Only execute after Task 3 soak period confirms stability.

- [ ] **Step 1: Remove PIPELINES map and legacy functions from Jenkinsfile**

Delete these sections:
- `PIPELINES` map (lines 29-90)
- `analyzeChangeset()` function (lines 220-263)
- `analyzePipelineRequirements()` function (lines 270-379)
- `propagateDependencies()` function (lines 467-491)
- `groupByDependencyLevel()` function (lines 443-460)
- `orderByDependencies()` function (lines 388-437)
- `loadPipelineBaselines()` function (lines 498-545) — replace with simplified global-only version
- Per-pipeline baseline update logic in Execute Builds stage

Keep:
- `archivePipelineBaselines()` (simplified to global-only in Task 2)
- The `node('built-in')` post block fix
- Build graph invocation and stage orchestration
- Pipeline metadata needed by Execute Builds (jenkinsPath, deploymentCheck, etc.) — move to manifests or a minimal config map

- [ ] **Step 2: Move pipeline execution metadata to manifests**

The Execute Builds stage needs `jenkinsPath` and `deploymentCheck` to trigger downstream jobs and verify deployments. Add these to the manifest format:

```json
{
  "manifestVersion": "1.1",
  "pipeline": "elohim-edge",
  "jenkinsPath": "elohim/holochain/Jenkinsfile",
  "deployment": {
    "targets": {
      "alpha": { "healthCheck": "https://alpha-edge.elohim.host/health" },
      "staging": { "healthCheck": "https://staging-edge.elohim.host/health" }
    }
  }
}
```

Update `build-graph.groovy` to surface `jenkinsPath` and deployment config from manifests.

- [ ] **Step 3: Remove comparison matrix from build-graph.groovy**

Delete:
- `getKnownDivergences()` (lines 571-600)
- `formatComparisonMatrix()` (lines 601-668)
- The comparison call in `runBuildGraph()`

Keep:
- `formatDecisionMatrix()` (the BUILD/SKIP decision matrix — always useful)
- `formatPerFileMatrix()` (per-file routing — always useful)

- [ ] **Step 4: Archive orchestrator-strategy.mjs**

The JS mirror of the PIPELINES algorithm was used for local testing. With PIPELINES gone, it's dead code. Either:
- Delete it and its tests
- Or move to `genesis/orchestrator/archive/` if someone wants the reference

The manifest-based tests in the test file should be kept and may need to be moved to a new test file that imports from `manifest-utils.mjs` instead.

- [ ] **Step 5: Update drift detection tests**

The tests at lines 504-533 that verify the JS mirror matches the Jenkinsfile are no longer needed. Replace with tests that verify:
- Every manifest is valid JSON and has required fields
- Every manifest's `pipeline` field is unique
- No cycles in the dependency graph
- The graph walker produces correct BUILD/SKIP decisions for synthetic changesets

- [ ] **Step 6: Update CLAUDE.md**

In the root `CLAUDE.md`, update the CI/CD section:
- Remove references to `PIPELINES` map
- Document that `build-manifest.json` files are the source of truth
- Update the pipeline table to reference manifest locations instead of change patterns

- [ ] **Step 7: Run full test suite and push**

Run: `cd genesis/orchestrator && pnpm exec vitest run orchestrator-strategy.test.mjs`

Let pre-push hooks validate everything.

```
refactor(orchestrator): remove legacy PIPELINES algorithm

The manifest-driven build graph is now the sole source of truth for
change detection. Removes ~300 lines of legacy Groovy code, the JS
mirror, and the comparison matrix. Pipeline execution metadata
(jenkinsPath, deploymentCheck) now lives in build-manifest.json files.
```

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Graph misses a pipeline that should build | Soak period (Task 3) catches this before legacy code is deleted |
| Sophia submodule gitlink not matched by glob | Test in Task 1 Step 3 validates this; `sophia` matches `sophia/` via `matchesGlob()` |
| Genesis auto-include breaks | Preserved explicitly in Task 2 Step 2 |
| `elohim-steward` accidentally triggers | `manualOnly` check preserved in Task 2 Step 2 |
| Build graph has a bug in staleness detection | Legacy comparison matrix warns during soak period |
| Rollback needed | Task 2 is one commit; `git revert` restores legacy as primary |
