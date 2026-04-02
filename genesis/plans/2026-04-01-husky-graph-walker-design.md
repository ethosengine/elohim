# Husky Graph Walker Design

**Date:** 2026-04-01
**Status:** Approved
**Scope:** Replace pre-push hook's project detection with manifest-driven graph walker
**Depends on:** `genesis/plans/2026-03-31-build-graph-orchestrator-design.md` (build manifest format)

## Problem

The husky pre-push hook (`.husky/pre-push`) has ~60 lines of ad-hoc grep-based project detection (lines 76-131) and a ~20-line case statement mapping project names to directories (lines 324-341). This is a second, independent system solving the same change-detection problem the build manifests already solve. They drift independently.

## Decision

Replace the hook's project detection with a Node.js graph walker that reads the same `build-manifest.json` files the Jenkins orchestrator uses. One set of manifests, two consumers (Jenkins Groovy + husky Node.js), same result.

## Source-of-Truth Classification

The walker is **Category C (Operational)** — a build-time tool, not protocol data. The manifests it reads are also Category C (see build-graph-orchestrator-design.md).

## Design Decisions

1. **Gate mapping in manifests (not in hook):** Each manifest declares a `gate.projects` field mapping manifest steps to hook project names and working directories. This keeps the manifest as the single source of truth for "what does this pipeline affect locally."

2. **Full manifest convergence:** All quality-only checks (schema-validate, orchestrator lint, etc.) get manifest homes — either as steps in existing manifests or as new stub manifests. No grep patterns remain in the active path.

3. **Shared manifest-utils.mjs:** Extract manifest discovery/loading from `validate-manifests.mjs` into a shared module. Both the validation script and the graph walker import from it.

4. **Fallback preserved:** If Node.js isn't available or no manifests exist, the hook falls back to the current grep-based logic verbatim.

## Manifest Schema Extension

Add an optional `gate` field to `manifest.schema.json`:

```json
"gate": {
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "projects": {
      "type": "object",
      "additionalProperties": {
        "type": "object",
        "required": ["dir"],
        "additionalProperties": false,
        "properties": {
          "dir": {
            "type": "string",
            "description": "Working directory for running the quality gate"
          },
          "steps": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Which manifest steps trigger this gate project (default: all steps in this manifest)"
          }
        }
      },
      "description": "Map of hook project name -> gate config"
    }
  }
}
```

### Gate mapping examples

**1:1 pipeline-to-gate (simple case):**

```json
{
  "pipeline": "elohim",
  "gate": {
    "projects": {
      "elohim-app": { "dir": "app/elohim-app" }
    }
  }
}
```

When `steps` is omitted, any stale step in the manifest triggers that gate project.

**1:N pipeline-to-gates (edge manifest):**

```json
{
  "pipeline": "elohim-edge",
  "gate": {
    "projects": {
      "doorway": { "dir": "doorway/doorway-service", "steps": ["cargo-build-doorway"] },
      "elohim-storage": { "dir": "elohim/elohim-storage", "steps": ["cargo-build-storage"] }
    }
  }
}
```

**N:1 steps-to-gate (genesis manifest with quality steps):**

```json
{
  "pipeline": "elohim-genesis",
  "gate": {
    "projects": {
      "genesis": { "dir": "genesis/seeder", "steps": ["validate-seeds", "seed-content"] },
      "schema-validate": { "dir": ".", "steps": ["schema-validate"] },
      "schema-codegen": { "dir": ".", "steps": ["schema-codegen"] },
      "constants-sync": { "dir": ".", "steps": ["constants-sync"] },
      "genesis-a2o": { "dir": "genesis/a2o", "steps": ["lint-a2o"] }
    }
  }
}
```

## File Structure

```
genesis/orchestrator/
├── manifest-utils.mjs      # Shared: discover, load, parse manifests
├── graph-walker.mjs         # New: match changed files -> affected gate projects
├── validate-manifests.mjs   # Existing: refactored to import from manifest-utils
└── manifest.schema.json     # Existing: extended with gate field
```

### manifest-utils.mjs

Exports:
- `discoverManifests(rootDir)` — finds all `build-manifest.json` files, returns paths
- `loadManifests(rootDir)` — discovers + parses + returns `[{ path, content }]`
- `resolveStep(dep, currentPipeline)` — normalizes bare `"build-angular"` to `"elohim:build-angular"`

### graph-walker.mjs

Exports:
- `walkGraph(rootDir, changedFiles)` — main entry point, returns `{ projects }` (see return value below)

Also executable as CLI: reads changed files from stdin (one per line), writes JSON result to stdout.

**Dependency:** `picomatch` for glob matching.

### validate-manifests.mjs

Refactored to import `discoverManifests`/`loadManifests` from `manifest-utils.mjs` instead of inline discovery.

## Walker Algorithm

`walkGraph(rootDir, changedFiles)` runs four phases:

### Phase 1: Load & Index

```
discoverManifests(rootDir) -> loadManifests(rootDir)
Build stepIndex: Map<qualifiedName, { step, pipeline, manifest }>
Build sourceIndex: [{ pattern, qualifiedStep }]  (flattened from all steps' inputs.sources)
```

### Phase 2: Mark Stale

For each step, check two conditions:

1. **Source glob match:** For each changed file, test against `inputs.sources` patterns using picomatch. Match -> mark step stale with reason `"source: <file>"`

2. **Build process file match:** For each `buildProcess` entry:
   - `"Jenkinsfile"` (whole-file) -> if `Jenkinsfile` appears in changed files -> stale, reason `"buildProcess: Jenkinsfile"`
   - `"Jenkinsfile@funcName"` (function ref) -> if `Jenkinsfile` appears in changed files -> stale, reason `"buildProcess: Jenkinsfile@funcName"`

   The hook does NOT hash function bodies (that's Jenkins-only). It checks whether the referenced file was modified. Less precise but correct — if the Jenkinsfile changed, any step referencing it is stale.

### Phase 3: Propagate

Topological sort all steps. Walk in dependency order: if any step in `depends` is stale -> mark this step stale with reason `"depends: <dep-step>"`. Transitive — staleness cascades through the full chain.

### Phase 4: Map to Gate Projects

For each manifest with stale steps:
- Read `gate.projects`
- For each gate project, check if any of its `steps` (or all steps if `steps` omitted) are stale
- Collect unique project names with their `dir` and accumulated reasons
- **Output in dependency order:** Gate projects are sorted using the same topological order from Phase 3. If gate project A's steps depend (transitively) on gate project B's steps, B appears first. This ensures parity with Jenkins' execution order — dependencies run before dependents.

### Return Value

`projects` is an **ordered array** (not an object) to preserve dependency order:

```js
{
  projects: [
    {
      name: "sophia",
      dir: "sophia",
      reasons: ["source: sophia/src/foo.ts"]
    },
    {
      name: "elohim-app",
      dir: "app/elohim-app",
      reasons: ["depends: elohim-sophia:build-sophia-umd"]
    }
  ]
}
```

Projects appear in topological order — dependencies before dependents. Projects with no dependency relationship maintain stable detection order.

## Pre-Push Hook Integration

The hook's structure stays the same. Only the **project detection** block (lines 76-131) and **project-to-directory mapping** (lines 324-341) change.

### New flow

```sh
# -- Project Detection --
# Try manifest-driven detection first, fall back to grep patterns

PROJECTS=""
PROJECT_DIRS=""
USE_MANIFEST=false

if command -v node >/dev/null 2>&1; then
  MANIFEST_RESULT=$(echo "$CHANGED" | node genesis/orchestrator/graph-walker.mjs 2>/dev/null)
  if [ $? -eq 0 ] && [ -n "$MANIFEST_RESULT" ]; then
    USE_MANIFEST=true
    # Parse JSON output: extract project names and dirs (preserving dependency order)
    eval "$(echo "$MANIFEST_RESULT" | node -e "
      const d = JSON.parse(require('fs').readFileSync('/dev/stdin','utf8'));
      const names = [], dirs = [];
      for (const p of d.projects) { names.push(p.name); dirs.push(p.dir); }
      console.log('PROJECTS=\"' + names.join(' ') + '\"');
      console.log('MANIFEST_DIRS=\"' + dirs.join(' ') + '\"');
    ")"
  fi
fi

if [ "$USE_MANIFEST" = false ]; then
  # Fallback: current grep-based detection (preserved verbatim)
  # ... existing lines 76-131 ...
fi
```

### Directory resolution

When the walker succeeds, the JSON result includes `dir` per project. The hook parses these alongside project names and passes them to `run_gate`. When using the fallback, the existing case statement (lines 324-341) provides the directory as before. The `run_gate` function signature (`run_gate PROJECT_NAME PROJECT_DIR`) stays the same in both paths — only the source of `PROJECT_DIR` changes.

The inline JSON parser extracts both names and dirs into parallel shell lists, preserving dependency order:

```sh
eval "$(echo "$MANIFEST_RESULT" | node -e "
  const d = JSON.parse(require('fs').readFileSync('/dev/stdin','utf8'));
  const names = [], dirs = [];
  for (const p of d.projects) { names.push(p.name); dirs.push(p.dir); }
  console.log('PROJECTS=\"' + names.join(' ') + '\"');
  console.log('MANIFEST_DIRS=\"' + dirs.join(' ') + '\"');
")"
```

During the gate loop, the Nth project name pairs with the Nth directory from `MANIFEST_DIRS`, bypassing the case statement entirely. Projects run in dependency order — dependencies' gates complete before dependents'.

### What stays untouched

- Lockfile consistency check (lines 40-45)
- Change detection / stdin parsing (lines 48-73)
- Resource guard / rust-analyzer pause (lines 274-300)
- Gate runner / `run_gate` function and all fallback commands (lines 146-272)
- Results reporting (lines 302-383)

## Manifest Coverage Plan

### Existing manifests — new quality steps + gate mappings

**`genesis/build-manifest.json`** — add steps:
- `schema-validate`: sources `["genesis/seeds/**", "elohim/sdk/schemas/**"]`
- `schema-codegen`: sources `["elohim/sdk/schemas/**"]`
- `constants-sync`: sources `["genesis/data/**", "elohim/sdk/schemas/**/*enum*", "**/generated/schema-enums*"]`
- `lint-a2o`: sources `["genesis/a2o/**"]`

Gate mapping:
```json
"gate": {
  "projects": {
    "genesis": { "dir": "genesis/seeder", "steps": ["validate-seeds", "seed-content"] },
    "schema-validate": { "dir": ".", "steps": ["schema-validate"] },
    "schema-codegen": { "dir": ".", "steps": ["schema-codegen"] },
    "constants-sync": { "dir": ".", "steps": ["constants-sync"] },
    "genesis-a2o": { "dir": "genesis/a2o", "steps": ["lint-a2o"] }
  }
}
```

**`elohim/holochain/dna/build-manifest.json`** — add step:
- `schema-dna`: sources `["elohim/holochain/**", "elohim/sdk/schemas/**"]`

Gate mapping:
```json
"gate": {
  "projects": {
    "schema-dna": { "dir": ".", "steps": ["schema-dna"] }
  }
}
```

**`app/elohim-app/build-manifest.json`** — add step:
- `lint-library`: sources `["app/elohim-library/**"]`

Gate mapping:
```json
"gate": {
  "projects": {
    "elohim-app": { "dir": "app/elohim-app", "steps": ["build-angular", "build-site-image"] },
    "elohim-library": { "dir": "app/elohim-library", "steps": ["lint-library"] }
  }
}
```

**`elohim/holochain/build-manifest.json`** (edge) — add gate:
```json
"gate": {
  "projects": {
    "doorway": { "dir": "doorway/doorway-service", "steps": ["cargo-build-doorway"] },
    "elohim-storage": { "dir": "elohim/elohim-storage", "steps": ["cargo-build-storage"] }
  }
}
```

**`steward/device/build-manifest.json`** — add gate:
```json
"gate": {
  "projects": {
    "steward-node": { "dir": "steward/node" }
  }
}
```

### New stub manifests

**`sophia/build-manifest.json`** — pipeline `elohim-sophia`:
- Step `build-sophia-umd`: sources `["sophia/**"]`, buildProcess `["sophia.Jenkinsfile"]`
- Gate: `sophia` -> dir `sophia`
- Note: resolves the missing cross-manifest dependency target (`elohim:build-angular` depends on `elohim-sophia:build-sophia-umd`)

**`doorway/doorway-app/build-manifest.json`** — pipeline `elohim-doorway-app`:
- Step `build-doorway-app`: sources `["doorway/doorway-app/**"]`
- Gate: `doorway-app` -> dir `doorway/doorway-app`

**`elohim/elohim-compute/build-manifest.json`** — pipeline `elohim-compute`:
- Step `build-compute`: sources `["elohim/elohim-compute/**"]`
- Gate: `elohim-compute` -> dir `elohim/elohim-compute`

**`genesis/orchestrator/build-manifest.json`** — pipeline `elohim-orchestrator`:
- Step `lint-jenkinsfiles`: sources `["**/Jenkinsfile*", "genesis/orchestrator/**"]`
- Gate: `orchestrator` -> dir `genesis/orchestrator`

## Constraints

- Walker must work with zero manifests (returns empty projects, hook falls back)
- `picomatch` is the only new dependency
- No changes to what quality gates run or how they execute
- buildProcess matching is file-level only (no function body hashing — that's Jenkins-only)
- All source glob patterns in manifests are relative to repo root
