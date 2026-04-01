# Build Graph Orchestrator Design

**Date:** 2026-03-31
**Status:** Draft
**Scope:** Replace pattern-based changeset analysis with declarative build graph

## Problem

The orchestrator matches changed file paths to pipeline patterns, but doesn't understand build-process invalidation. A Jenkinsfile change that adds `buildServiceWorker` invalidates the site output even though no `app/` source files changed — the orchestrator skips the rebuild. 53 of 96 recent fix commits are build/CI sync failures, not application bugs. The build system cannot express its own inputs.

## Source-of-Truth Classifications

| Data Entity | Category | Source of Truth | Stage 1 Evolution |
|-------------|----------|----------------|-------------------|
| `build-manifest.json` | **C (Operational)** — build declarations checked into git | Git repo (file per pipeline directory) | Becomes ContentNode (`contentType: "build-manifest"`), CID-addressed, gossipped via DHT |
| `build-state.json` | **C (Operational)** — ephemeral build result state | Jenkins artifact storage | Becomes `build-attestation` ContentNode, signed by builder's agent key |
| Step hashes in `stepStates` | **C (Operational)** — computed, not authored | Derived from `build-manifest.json` inputs at build time | Attestation payload — hash of inputs + outputs, signed |

**Rationale:** Both schemas are operational (Category C) — they support the build process, not the protocol's notarized data. No DHT entry types are created. In Stage 1, the manifests graduate to Category A (notarized ContentNodes) and attestations graduate to Category B2 (agent-scoped with attestation). That graduation is out of scope for this design.

## Decision

**Approach B: Per-pipeline build manifests with central composition.**

Each pipeline directory gets a `build-manifest.json` declaring its build steps, inputs, outputs, and dependencies. The orchestrator discovers all manifests, composes them into a unified DAG, and walks it to determine the minimal rebuild set. Zero guessing.

**Granularity:** Build-step-level. Each named build step (e.g., `build-angular`, `build-service-worker`, `cargo-build-doorway`) is an independent node in the graph with its own inputs and outputs.

**Stage 1 alignment:** The manifest format is designed to become a ContentNode. No Jenkins-specific fields outside the `executor` block. When steward nodes become builders, the manifests move to the DHT — the graph algorithm stays the same, only the executor changes.

**Existing art:** The graph-walking and input-hashing patterns are borrowed from Nx/Bazel. We don't adopt those tools (Rust support is immature, neither maps to protocol-native builds) but use their proven algorithms: DAG composition, topo-sort into parallel levels, input hashing for staleness.

## Manifest Schema

Each pipeline directory contains a `build-manifest.json`:

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim",
  "description": "Elohim Angular app, service worker, and site image",

  "steps": {
    "build-angular": {
      "description": "Build Angular production bundle",
      "inputs": {
        "sources": [
          "app/elohim-app/src/**",
          "app/elohim-app/angular.json",
          "app/elohim-app/tsconfig*.json",
          "app/elohim-library/**"
        ],
        "buildProcess": [
          "Jenkinsfile@buildAngularApp"
        ]
      },
      "outputs": {
        "artifacts": ["elohim-app-dist"],
        "verify": "test -d app/elohim-app/dist/elohim-app"
      },
      "depends": ["elohim-sophia:build-sophia-umd"],
      "executor": {
        "stage": "Build Angular App",
        "function": "buildAngularApp"
      }
    },
    "build-service-worker": {
      "description": "Compile and inject service worker into app bundle",
      "inputs": {
        "sources": [],
        "buildProcess": [
          "Jenkinsfile@buildServiceWorker"
        ]
      },
      "outputs": {
        "artifacts": ["service-worker"],
        "verify": "test -f app/elohim-app/dist/elohim-app/browser/sw.js"
      },
      "depends": ["build-angular"],
      "executor": {
        "stage": "Build Service Worker",
        "function": "buildServiceWorker"
      }
    },
    "build-site-image": {
      "description": "Package site into container image",
      "inputs": {
        "sources": [
          "app/elohim-app/Dockerfile",
          "app/elohim-app/nginx*.conf"
        ],
        "buildProcess": [
          "Jenkinsfile@buildSiteImage"
        ]
      },
      "outputs": {
        "artifacts": ["site-image"],
        "verify": null
      },
      "depends": ["build-service-worker"],
      "executor": {
        "stage": "Build Site Image",
        "function": "buildSiteImage"
      }
    }
  },

  "deployment": {
    "targets": {
      "alpha": { "healthCheck": "https://alpha.elohim.host/version" },
      "staging": { "healthCheck": "https://staging.elohim.host/version" }
    }
  }
}
```

### Input Categories

| Category | Purpose | Staleness detection |
|----------|---------|-------------------|
| `sources` | Files the step transforms | Changed file matches a glob pattern |
| `buildProcess` | Code that performs the build | `Jenkinsfile@functionName` — orchestrator extracts function body, SHA-256 hashes it, compares to stored hash |
| `dependencies` | Implicit via `depends` field | If any dependency step is stale, this step is stale |

### Field Reference

| Field | Required | Description |
|-------|----------|-------------|
| `manifestVersion` | Yes | Schema version for the manifest format itself |
| `pipeline` | Yes | Pipeline name — must match orchestrator's pipeline identifier |
| `description` | Yes | Human-readable description of this pipeline's purpose |
| `steps` | Yes | Map of step name → step definition |
| `steps.*.description` | Yes | What this step does |
| `steps.*.inputs.sources` | Yes | Glob patterns for source files (empty array if step has no direct sources) |
| `steps.*.inputs.buildProcess` | Yes | References to build logic that invalidates this step (empty array if none) |
| `steps.*.outputs.artifacts` | Yes | Named output artifacts this step produces |
| `steps.*.outputs.verify` | No | Shell command to confirm artifact was produced (null if not verifiable locally) |
| `steps.*.depends` | Yes | Step dependencies — bare name for same-manifest, `pipeline:step` for cross-manifest |
| `steps.*.executor` | Yes | Jenkins-specific execution mapping (replaced by different executor in Stage 1) |
| `steps.*.executor.stage` | Yes | Jenkins stage name |
| `steps.*.executor.function` | Yes | Jenkinsfile helper function name |
| `deployment` | No | Deployment targets and health checks |

### Cross-Manifest References

Dependencies use `pipeline:step` syntax for cross-manifest references:

```json
"depends": ["elohim-sophia:build-sophia-umd"]
```

Local dependencies (same manifest) use bare step names:

```json
"depends": ["build-angular"]
```

The orchestrator validates at composition time that every dependency target exists across all discovered manifests.

### The `@functionName` Convention

Build process references use `File@functionName` syntax:

```
"buildProcess": ["Jenkinsfile@buildServiceWorker"]
```

The orchestrator:
1. Parses the target file (Jenkinsfile)
2. Extracts the body of `def buildServiceWorker() { ... }`
3. SHA-256 hashes the function body
4. Compares to the stored hash from the last successful build
5. If different → step is stale, regardless of source file changes

This replaces fragile line-number tracking. Function names are stable across refactors. If a function is renamed, the manifest is updated in the same commit.

## Manifest Locations

```
sophia/build-manifest.json                  → pipeline: elohim-sophia
app/elohim-app/build-manifest.json          → pipeline: elohim
elohim/holochain/dna/build-manifest.json    → pipeline: elohim-holochain
elohim/holochain/build-manifest.json        → pipeline: elohim-edge (includes doorway + storage steps)
genesis/build-manifest.json                 → pipeline: elohim-genesis
steward/device/build-manifest.json          → pipeline: elohim-steward
```

6 manifests, not 7 — doorway and storage don't have their own Jenkinsfiles. Both are built by the edge pipeline (`elohim/holochain/Jenkinsfile`), so their build steps live in the edge manifest. Source globs still reference the correct source directories (`doorway/doorway-service/**`, `elohim/elohim-storage/**`).

Each manifest is co-located with the Jenkinsfile that executes it. The orchestrator discovers them via `glob('**/build-manifest.json')` — no central registry.

## Graph Composition

### Phase 1: Discover

```
glob('**/build-manifest.json')
→ Parse each file
→ Validate schema (manifestVersion, required fields)
→ Collect into map: pipeline name → manifest
```

### Phase 2: Merge

```
Union all steps across manifests into a single graph
→ Prefix step names with pipeline for uniqueness (internal only — manifests use bare names)
→ Resolve cross-manifest depends references (pipeline:step → internal key)
→ Detect cycles (error if found)
→ Validate: every dependency target exists
→ Validate: no duplicate pipeline names across manifests
```

### Phase 3: Walk

```
For each step in the graph:
  1. Check sources: any changed file matches a source glob? → mark stale
  2. Check buildProcess: any function hash changed? → mark stale
  3. Propagate: any dependency marked stale? → mark stale (recursive/transitive)

Group stale steps by pipeline
Topo-sort into parallel execution levels
Emit: { pipeline → [stale step names] } per level
```

### Parallel Execution Levels

Steps with no inter-dependencies run in parallel:

```
Level 0: [build-sophia-umd, build-dna-wasm]       (no deps, parallel)
Level 1: [build-angular, build-happ]               (depend on level 0)
Level 2: [build-service-worker, cargo-build-doorway]
Level 3: [build-site-image, build-edge-image]
Level 4: [validate-seeds, seed-content]
```

## Change Detection

### Staleness Rules

A step is stale if ANY of these are true:

1. **Source change:** A file in the changeset matches any pattern in `inputs.sources`
2. **Build process change:** A `buildProcess` function hash differs from the stored hash in `build-state.json`
3. **Dependency change:** Any step listed in `depends` is stale (transitive — if A depends on B depends on C, and C is stale, all three rebuild)

### Build State Persistence

The orchestrator maintains `build-state.json` as a Jenkins artifact (extends the existing `pipeline-baselines.json`):

```json
{
  "version": "1.0",
  "lastSuccessfulCommit": "0e2198f9",
  "stepStates": {
    "elohim:build-angular": {
      "buildProcessHash": "a1b2c3d4e5f6...",
      "lastBuiltCommit": "0e2198f9",
      "outputVerified": true
    },
    "elohim:build-service-worker": {
      "buildProcessHash": "d4e5f67890ab...",
      "lastBuiltCommit": "98a0fdff",
      "outputVerified": true
    }
  }
}
```

Step state keys use `pipeline:step` format for global uniqueness.

### Decision Matrix Output

The orchestrator prints a detailed matrix showing what's building and why:

```
╔══════════════════════════════════════════════════════════════════════╗
║                     BUILD GRAPH DECISION MATRIX                      ║
╠══════════════════════════════════════════════════════════════════════╣
║ Step                      │ Status  │ Reason                         ║
║ build-sophia-umd          │ SKIP    │ no source/process changes       ║
║ build-angular             │ SKIP    │ no source changes               ║
║ build-service-worker      │ BUILD   │ buildProcess hash changed       ║
║ build-site-image          │ BUILD   │ depends: build-service-worker   ║
║ cargo-build-doorway       │ SKIP    │ no source/process changes       ║
║ build-edge-image          │ SKIP    │ no source/process changes       ║
║ build-dna-wasm            │ SKIP    │ no source/process changes       ║
║ validate-seeds            │ SKIP    │ no source changes               ║
╚══════════════════════════════════════════════════════════════════════╝
```

Every row has a reason. No guessing.

## Execution Model

### Orchestrator → Pipeline Communication

The orchestrator triggers pipelines with an explicit step list:

```groovy
build(
  job: 'elohim',
  parameters: [
    string(name: 'STEPS', value: 'build-service-worker,build-site-image'),
    string(name: 'TRIGGER_COMMIT', value: '0e2198f9')
  ]
)
```

### Pipeline-Side Step Gating

Each pipeline's Jenkinsfile checks whether a step is in the `STEPS` parameter:

```groovy
stage('Build Angular App') {
    when { expression { shouldRunStep('build-angular') } }
    steps { container('builder') { script { buildAngularApp() } } }
}

stage('Build Service Worker') {
    when { expression { shouldRunStep('build-service-worker') } }
    steps { container('builder') { script { buildServiceWorker() } } }
}

def shouldRunStep(stepName) {
    def steps = (params.STEPS ?: 'all').split(',')
    return steps.contains('all') || steps.contains(stepName)
}
```

### Fallback Safety

- `STEPS=all` (or missing/null parameter) runs every stage — identical to today's behavior
- If the orchestrator fails to parse manifests, it falls back to the existing PIPELINES pattern matching and logs a warning
- Any pipeline can be triggered manually without the orchestrator and builds everything

### Output Verification

After each step completes, the pipeline runs the step's `outputs.verify` command (if defined) inside the build container to confirm the artifact was actually produced. The verification result is reported back to the orchestrator via the build result. This closes the loop — the graph declares what should exist, then checks.

## Migration Plan

### Phase 1: Shadow Mode

Write manifests and the graph walker. Run it in parallel with the existing `analyzeChangeset()` + `analyzePipelineRequirements()`. Log divergences but don't act on the graph result:

```
╔══════════════════════════════════════════════════════════════════╗
║                    CHANGESET ANALYSIS COMPARISON                  ║
╠══════════════════════════════════════════════════════════════════╣
║                        │ PIPELINES map │ Build Graph             ║
║ elohim (app)           │ SKIP          │ BUILD (buildProcess Δ)  ║  ← DIVERGENCE
║ elohim-edge            │ BUILD         │ BUILD (sources Δ)       ║
║ elohim-sophia          │ SKIP          │ SKIP                    ║
╚══════════════════════════════════════════════════════════════════╝
```

Every divergence is either a manifest bug (fix the manifest) or a PIPELINES bug (the graph caught something real).

### Phase 2: Graph-Primary

Once shadow mode produces zero false negatives across multiple builds, flip to graph-primary with PIPELINES as fallback:

```groovy
def analysis = null
try {
    analysis = walkBuildGraph(changedFiles)
} catch (Exception e) {
    echo "⚠️ Build graph failed: ${e.message}, falling back to PIPELINES"
    analysis = analyzePipelineRequirements(changedFiles)
}
```

### Phase 3: PIPELINES Removal

After N successful graph-primary builds with zero fallbacks, remove `analyzeChangeset()`, `analyzePipelineRequirements()`, and the PIPELINES map. The manifests are the single source of truth.

### What Doesn't Change

- Jenkins pipeline structure (stages, containers, agents)
- Webhook trigger mechanism (orchestrator receives, decides, delegates)
- Deployment logic (deploy stages stay in each pipeline's Jenkinsfile)
- Credential handling, artifact archiving, notifications

### What Gets Simpler

- Orchestrator shrinks — graph composition + walking replaces ~150 lines of pattern matching
- Adding a new build step = add to the manifest, no orchestrator change
- Each pipeline gains `shouldRunStep()` but otherwise stays the same

## Stage 1 Seam (Protocol-Native Build System)

The manifest format is designed to become a ContentNode without redesign:

| Manifest field | ContentNode equivalent |
|---------------|----------------------|
| `manifestVersion` | `schemaVersion` |
| `pipeline` | `contentType: "build-manifest"` |
| `steps[*].inputs` | Content body |
| `steps[*].outputs.artifacts` | EPR references to output ContentNodes |
| `steps[*].depends` | `relatedNodeIds` |

### What Stage 1 Adds (Not Scope of This Design)

- Each `build-manifest.json` gets a CID (content-addressed identity)
- `build-state.json` becomes a `build-attestation` ContentNode — "I, Jenkins, attest that step X with input hash Y produced output hash Z"
- Attestations are signed by the builder's agent key
- Multiple builders can produce competing attestations — threshold agreement = trust
- WASM-based executors replace the `executor` block — build steps run in sandboxed WASM modules on steward nodes instead of Docker containers on Jenkins agents

### Design Constraints for Stage 1 Readiness

1. No Jenkins-specific fields outside the `executor` block
2. `outputs.artifacts` are logical names, not filesystem paths — they become CID references
3. `build-state.json` records per-step hashes — attestation data in embryo
4. Manifest format is JSON — parseable by any future runtime (Groovy, Rust, WASM)
5. The graph algorithm (discover, merge, walk) is runtime-agnostic — same logic works whether executed by Groovy in Jenkins or by a zome in Holochain

## Complete Build Graph (Current System)

### Manifests to Create

**sophia/build-manifest.json**
- `build-sophia-umd`: sources `sophia/**`, outputs `sophia-element.umd.js`, no deps

**app/elohim-app/build-manifest.json**
- `build-angular`: sources `app/elohim-app/src/**`, `app/elohim-library/**`, depends `elohim-sophia:build-sophia-umd`
- `build-service-worker`: buildProcess `Jenkinsfile@buildServiceWorker`, depends `build-angular`
- `build-site-image`: sources `app/elohim-app/Dockerfile`, depends `build-service-worker`

**doorway/doorway-service/build-manifest.json**
- `cargo-build-doorway`: sources `doorway/doorway-service/**`, `doorway/doorway-client/**`
- `build-edge-image`: sources `doorway/doorway-service/Dockerfile`, depends `cargo-build-doorway`, `elohim-holochain:build-happ`

**elohim/holochain/dna/build-manifest.json**
- `build-dna-wasm`: sources `elohim/holochain/dna/**`, `elohim/elohim-cache-core/**`
- `build-happ`: depends `build-dna-wasm`

**elohim/elohim-storage/build-manifest.json**
- `cargo-build-storage`: sources `elohim/elohim-storage/**`
- `export-ts-bindings`: depends `cargo-build-storage`

**genesis/build-manifest.json**
- `validate-seeds`: sources `genesis/seeds/**`, `elohim/sdk/schemas/**`
- `seed-content`: depends `validate-seeds`, `elohim:build-site-image`, `elohim-edge:build-edge-image`, `elohim-storage:cargo-build-storage`

**steward/node/build-manifest.json** (manual-only)
- `cargo-build-steward`: sources `steward/node/**`, `crates/**`
- `build-steward-app`: depends `cargo-build-steward`, `elohim-holochain:build-happ`

### Full Dependency Topology

```
Level 0 (parallel):
  build-sophia-umd          [sophia]
  build-dna-wasm            [elohim-holochain]

Level 1 (parallel):
  build-angular             [elohim]         ← depends: build-sophia-umd
  build-happ                [elohim-holochain] ← depends: build-dna-wasm

Level 2 (parallel):
  build-service-worker      [elohim]         ← depends: build-angular
  cargo-build-doorway       [elohim-edge]    (independent)
  cargo-build-storage       [elohim-storage] (independent)
  validate-seeds            [genesis]        (independent, sources only)

Level 3 (parallel):
  build-site-image          [elohim]         ← depends: build-service-worker
  build-edge-image          [elohim-edge]    ← depends: cargo-build-doorway, build-happ
  export-ts-bindings        [elohim-storage] ← depends: cargo-build-storage

Level 4:
  seed-content              [genesis]        ← depends: validate-seeds, build-site-image,
                                                         build-edge-image, cargo-build-storage
```
