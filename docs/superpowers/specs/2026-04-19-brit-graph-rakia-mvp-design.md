# Brit Graph + Rakia MVP Design

**Date:** 2026-04-19
**Status:** Approved
**Author:** Matthew Dowell + Claude Opus 4.6

## TL;DR

Brit gains a generic graph engine (`brit-graph`) that makes DAG construction, affected tracking, fingerprinting, and topological planning protocol infrastructure — not build-specific. Rakia consumes brit-graph for build semantics: manifest discovery, constellation (build DAG) construction, and build planning. Change detection uses brit/gix (no git shell-outs). Schema formalization comes last as an IoC cleanup pass, forcing consistency across everything built pragmatically. The work proceeds B (DAG) -> C (change detection) -> A (schema), with parallel opportunities where they appear.

Moon (moonrepo/moon) inspires the graph patterns — two-graph separation, `AffectedBy` provenance, task fingerprinting, upstream/downstream scoping — but the identity layer is EPR-native (BritCid-keyed, ContentNode-typed), not file-path-native.

## Problem

53 of the last 96 fix commits in the Elohim monorepo were build/CI failures. The Groovy DAG walker (`genesis/orchestrator/build-graph.groovy`, 821 lines) and Jenkins orchestrator (1484 lines) have accumulated workarounds for baseline leapfrog, cold-start oscillation, GString equality bugs, LazyMap serialization, and silent baseline drops. These are symptoms of the build system being an imperative script that discovers dependencies at runtime rather than a declarative graph that knows its own inputs.

The existing rakia design spec (2026-04-12) calls for rakia to own the DAG. This design refines that: brit owns the generic graph engine (protocol infrastructure reusable for merge proposals, fork governance, attestation accumulation), and rakia consumes it for build-specific semantics. This matches brit's role as the foundation — you dogfood brit's graph primitives by building toward rakia.

## Architecture

### Where things live

```
Protocol Schemas (elohim/sdk/schemas/v1/)
    | defines ContentNode types, reach enum, attestation format
    v
Brit (covenant on git — fork of gitoxide)
    | brit-epr:     engine (trailers, ContentNode, CID, object store, signing)
    |               elohim (attestations, refs, reach, pillar validation)
    | brit-graph:    NEW — generic EPR-native graph engine
    | brit-verify:   trailer verification CLI
    | brit-build-ref: attestation ref CLI
    | brit-cli:      NEW — unified CLI (graph, affected, plan, fingerprint + existing)
    v
Rakia (firmament for builds — consumes brit)
    | rakia-core:    manifest parser + build DAG construction (consumes brit-graph)
    | rakia-brit:    NEW — change detection via brit/gix, baseline refs
    | rakia-cli:     build execution CLI (rakia build, rakia ci)
    v
elohim-storage / steward infrastructure
    | swarm, discovery, ContentNode storage (Stage 2+)
```

### Git compatibility

Every operation maintains stock git compatibility per brit's design framing #2:

- Manifests are JSON files in the repo (`git clone` gets them)
- Baseline refs live in `refs/notes/rakia/baselines/` (valid git refs, survive clone/fetch)
- Attestations use commit trailers (RFC-822, `git log` renders them)
- The constellation is discoverable from manifest files in the worktree
- No magic outside the repo — `.git/brit/objects/` is the only non-standard directory, and it's a local cache

### Existing foundation (not reinvented)

| Component | Location | Status |
|-----------|----------|--------|
| `ContentNode` trait | `brit-epr/src/engine/content_node.rs` | Complete |
| `BritCid` (blake3) | `brit-epr/src/engine/cid.rs` | Complete |
| `LocalObjectStore` | `brit-epr/src/engine/object_store.rs` | Complete |
| `AgentKey` signing | `brit-epr/src/engine/signing.rs` | Complete |
| `BritRefManager` | `brit-epr/src/elohim/refs.rs` | Complete |
| Build/Deploy/Validation attestation schemas | `brit-epr/src/elohim/attestation/` | Complete |
| Reach computation | `brit-epr/src/elohim/attestation/reach.rs` | Complete |
| `brit verify` CLI | `brit-verify/src/main.rs` | Complete |
| `brit build-ref` CLI | `brit-build-ref/src/` | Complete |
| Manifest parser | `rakia-core/src/manifest.rs` | Complete |
| 8 build manifests | `**/build-manifest.json` across monorepo | Complete |

## Phase B: The DAG — "brit can tell you what depends on what"

### brit-graph (new crate in brit workspace)

Generic, EPR-native graph engine. Pure computation — no IO, no git, no network. Any type implementing `ContentNode` can be a graph node.

**Inspired by moon's patterns, adapted for EPR:**

| Moon pattern | brit-graph adaptation |
|-------------|----------------------|
| `GraphData` / `GraphConnections` traits | Same shape, but nodes keyed by `BritCid` instead of string IDs |
| `AffectedBy` enum | Extended: `ChangedFile`, `UpstreamNode`, `DownstreamNode`, `InputFingerprint`, `AlwaysAffected` |
| Upstream/downstream scoping (None/Direct/Deep) | Same — caller controls propagation depth |
| `TaskFingerprint` (hash of all inputs) | `ContentFingerprint` — deterministic blake3 over sorted inputs via `BritCid::compute` |
| Priority-grouped topological sort | `TopoPlan` — nodes grouped by dependency level for parallel execution |

**File structure:**

```
brit-graph/
├── Cargo.toml              # petgraph, brit-epr (for BritCid + ContentNode)
├── src/
│   ├── lib.rs              # module exports
│   ├── graph.rs            # EprGraph<N> — petgraph wrapper with BritCid-keyed lookup
│   ├── traits.rs           # GraphData, GraphConnections (dependencies_of, dependents_of, deep variants)
│   ├── affected.rs         # AffectedTracker + AffectedBy enum + provenance tracking
│   ├── fingerprint.rs      # ContentFingerprint — deterministic hash over typed inputs
│   └── topo.rs             # TopoPlan — topological sort with dependency-level grouping
└── tests/
    ├── graph_construction.rs
    ├── affected_tracking.rs
    ├── fingerprint_determinism.rs
    └── topo_ordering.rs
```

**Key types:**

```rust
/// A content-addressed directed graph. Nodes implement ContentNode,
/// giving each node a BritCid identity derived from its content.
/// Edges carry a generic relationship type (default: unit).
pub struct EprGraph<N: ContentNode, E = ()> {
    graph: DiGraph<NodeIndex, E>,
    cid_to_index: BTreeMap<BritCid, NodeIndex>,
    node_data: Vec<N>,
}

// Note: BuildStep in rakia-core will impl ContentNode, making its CID
// derivable from its canonical JSON (inputs, outputs, depends, executor).
// This means two steps with identical declarations produce the same CID —
// content-addressed identity, not path-based identity.

/// Why a node was marked as affected.
pub enum AffectedBy {
    ChangedFile(String),        // a source file matched an input pattern
    UpstreamNode(BritCid),      // a dependency was affected (transitive)
    DownstreamNode(BritCid),    // a dependent needs rebuilding
    InputFingerprint,           // content hash of inputs changed
    AlwaysAffected,             // explicitly marked
}

/// Configurable propagation depth.
pub enum PropagationScope {
    None,       // don't propagate
    Direct,     // immediate neighbors only
    Deep,       // full transitive closure
}

/// A topological execution plan grouped by dependency level.
pub struct TopoPlan<N> {
    /// Level 0: no dependencies. Level 1: depends only on level 0. Etc.
    pub levels: Vec<Vec<N>>,
}

/// Deterministic content fingerprint over all inputs.
pub struct ContentFingerprint {
    pub cid: BritCid,
    pub inputs: BTreeMap<String, BritCid>,  // input name -> content hash
}
```

### rakia-core gains a graph module

Rakia consumes brit-graph for build-specific semantics:

```
rakia-core/src/
├── lib.rs
├── manifest.rs             # existing — BuildManifest parser
├── graph.rs                # NEW — BuildStep implements ContentNode; constellation builder
└── discover.rs             # NEW — find all build-manifest.json in a worktree
```

**Constellation builder:**

1. `discover_manifests(repo_root) -> Vec<(PathBuf, BuildManifest)>` — walk the worktree, find and parse all `build-manifest.json` files
2. Qualify step names: `lint` in pipeline `elohim-orchestrator` becomes `elohim-orchestrator:lint`
3. Build `EprGraph<BuildStep>` — nodes are qualified steps, edges are `depends` relationships
4. Validate: no cycles, all dependency targets exist, report unresolved external deps
5. `plan_from_changes(graph, changed_paths) -> TopoPlan<BuildStep>` — match changed paths against `inputs.sources` globs, propagate through graph, return topological plan

**Glob matching:** Uses the `globset` crate (same engine as ripgrep) for source pattern matching. Must handle `**`, `*`, `?`, `[^/]` semantics. Validated against the 8 existing manifests.

### brit CLI surface (Phase B)

```
brit graph discover [--repo <path>]              # list all manifests and their steps
brit graph show [--repo <path>] [--format json|dot]  # full DAG as JSON or Graphviz DOT
brit affected [--repo <path>] --files <path,...>  # which steps are affected and why
brit plan [--repo <path>] --files <path,...>      # topological build plan from affected steps
brit fingerprint <manifest-path> [--step <name>]  # content fingerprint of inputs
```

All output is structured JSON by default. `--format dot` produces Graphviz for visualization. These commands are pure — they read manifests and compute, they don't execute builds or touch git state.

## Phase C: Change Detection — "brit can tell you what changed"

### rakia-brit (new crate in rakia workspace)

Bridges brit/gix object store access into rakia's build planning.

```
rakia-brit/
├── Cargo.toml              # depends on brit-epr, gix
├── src/
│   ├── lib.rs
│   ├── changes.rs          # changed_paths_since(baseline, head) via gix diff
│   └── baselines.rs        # read/write/migrate baseline refs
└── tests/
    ├── change_detection.rs # same results as git diff --name-only
    └── baseline_migration.rs
```

**Change detection:** `changed_paths_since(repo, baseline_ref, head) -> Vec<String>` — uses gix object store diff. No `git` CLI shell-out. Returns workspace-relative paths.

**Baseline management:** Reuses `BritRefManager` from brit-epr for `refs/notes/rakia/baselines/{pipeline}`. Baselines are git refs — they survive executor death, pod eviction, and Jenkins post-block failures. This dissolves the baseline leapfrog and oscillation bugs by construction.

**Migration bridge:** `migrate_baselines(pipeline_baselines_json) -> Result<()>` — reads Jenkins `pipeline-baselines.json` artifact, writes equivalent baseline refs. One-way migration for cutover.

**Integration flow:**

```
repo state
  -> rakia-brit: changed_paths_since(baseline, HEAD)
  -> rakia-core: match paths against manifest input globs
  -> brit-graph: AffectedTracker marks affected steps with provenance
  -> brit-graph: TopoPlan groups by dependency level
  -> output: structured JSON build plan
```

### Enhanced brit CLI surface (Phase C)

```
brit baseline read <pipeline> [--repo <path>]     # show current baseline ref
brit baseline write <pipeline> <commit> [--repo <path>]  # set baseline
brit baseline migrate <json-path> [--repo <path>] # one-way migration from Jenkins artifact
brit affected --since <ref> [--repo <path>]       # changed files -> affected steps (end-to-end)
brit plan --since <ref> [--repo <path>]            # full build plan from baseline to HEAD
```

The `--since <ref>` flag replaces the `--files` flag from Phase B with automatic change detection. Both forms remain available.

## Phase A: Schema Formalization — IoC Cleanup

After B and C are working and validated against the 8 existing manifests, formalize with protocol schemas.

### Protocol schemas

| Schema | Path | Purpose |
|--------|------|---------|
| `BuildManifestContentNode` | `schemas/elohim-protocol/v1/build-manifest.schema.json` | Formalizes manifest format — inputs, outputs, deps, gate, deployment |
| `BuildAttestationContentNode` | `schemas/elohim-protocol/v1/build-attestation.schema.json` | Already designed in Phase 2a spec |
| `BuildSchema` meta-schema | `schemas/elohim-protocol/v1/build-schema.schema.json` | What a pluggable build schema IS — vocabulary declaration |
| `ElohimBuildSchema` | `schemas/elohim-protocol/v1/elohim-build-schema.json` | Concrete vocabulary: Angular, Rust, WASM, Docker, seed, pnpm |

### Codegen pipeline

Schema -> Rust types in rakia-core. Same pattern as `schema:codegen:rs` in the monorepo. The existing hand-written manifest parser types get replaced by generated types.

### The cleanup pass

1. `serde_json::Value` escape hatches (`gate`, `deployment`, `executor` in manifest parser) become properly typed
2. Schema validation: all 8 existing manifests must pass `pnpm run schema:validate`
3. Any inconsistency between B/C implementation and schema demands gets refactored
4. `ElohimBuildSchema` vocabulary declared in schema, not hardcoded in Rust match arms

## Acceptance Criteria

### Phase B (DAG)

- [ ] `brit-graph` crate compiles independently with `brit-epr` as only brit dependency
- [ ] All 8 existing manifests parse and produce a valid acyclic `EprGraph`
- [ ] Cross-manifest dependencies resolve (e.g., `elohim-sophia:build-sophia-umd`)
- [ ] `plan_from_changes(["sophia/src/foo.ts"])` returns sophia build + elohim-app build (transitive)
- [ ] `AffectedBy` provenance: each affected step carries the reason (which file, which upstream node)
- [ ] `TopoPlan` groups steps by dependency level; level 0 has no deps
- [ ] `brit graph show --format dot` produces valid Graphviz output
- [ ] `brit affected --files <path>` outputs JSON with affected steps + provenance
- [ ] Fingerprint is deterministic: same inputs -> same `BritCid`

### Phase C (Change Detection)

- [ ] `changed_paths_since(baseline, HEAD)` produces same paths as `git diff --name-only`
- [ ] Zero git CLI shell-outs in the entire pipeline
- [ ] Baselines read/write to `refs/notes/rakia/baselines/{pipeline}`
- [ ] Baselines survive simulated executor death (they're refs, not artifacts)
- [ ] Migration from `pipeline-baselines.json` produces equivalent baseline refs
- [ ] `brit plan --since <ref>` produces end-to-end build plan: repo state -> changed paths -> affected steps -> topological plan
- [ ] Shadow-mode validation: `brit plan` output matches Groovy `build-graph.groovy` output for the same changeset

### Phase A (Schema)

- [ ] `BuildManifestContentNode` JSON Schema validates all 8 existing manifests
- [ ] Rust types in rakia-core are generated from or constrained by schemas
- [ ] No `serde_json::Value` escape hatches remain in manifest types
- [ ] `ElohimBuildSchema` vocabulary entirely declared in schema
- [ ] `pnpm run schema:validate` passes
- [ ] Changing a schema field and running codegen updates the Rust type

## Parallel Work Opportunities

Phases B, C, A are sequential on the critical path, but within each phase there's parallelism:

| Track | Can run in parallel with |
|-------|------------------------|
| `brit-graph` crate (pure, no IO) | `rakia-core` discover + glob matching |
| `brit-graph` tests against fixtures | `brit-cli` subcommand wiring |
| `rakia-brit` change detection | `rakia-brit` baseline migration |
| Schema authoring (Phase A) | Codegen pipeline setup |

Additionally, brit Phase 2 (ContentNode adapter — git objects become CID-addressed content) can proceed independently. It doesn't block or depend on the graph work.

## What This Does NOT Cover

| Item | Where it lives | Why deferred |
|------|---------------|-------------|
| Build execution | `rakia-executor` (rakia Sprint 5) | DAG + change detection must work first |
| P2P build dispatch | `rakia-peer` (Stage 2) | Requires steward network |
| Threshold attestation | Stage 2 | Requires multiple builders |
| `rakia build` CLI | `rakia-cli` | Depends on executor |
| Discernment gate | Separate sprint (in progress) | Independent workstream |
| ContentNode adapter (git objects -> CIDs) | brit Phase 2 | Independent, can parallel |

## Relationship to Existing Plans

This design **refines** the existing rakia design spec (2026-04-12) and brit phase roadmap:

- **Rakia Sprint 1** (schema) becomes Phase A (last, not first) — pragmatic ordering
- **Rakia Sprint 2** (DAG) becomes Phase B, but the generic graph engine lives in brit, not rakia
- **Rakia Sprint 3** (change detection) becomes Phase C — same scope, same approach
- **Rakia Sprint 4** (CLI) is absorbed into brit's CLI surface for graph/affected/plan commands; rakia CLI remains for build execution
- **Brit Phase 2a** (build attestation primitives) is complete and provides foundation
- **Brit Phase 2** (ContentNode adapter) proceeds independently, not blocked by this work

No existing plans are contradicted. The reordering (B -> C -> A instead of 1 -> 2 -> 3) is pragmatic — build first, formalize after. The architectural shift (graph engine in brit, not rakia) is an improvement — the same graph patterns serve merge proposals, fork governance, and attestation accumulation beyond builds.

## Key Design Decisions

1. **Brit owns the graph engine.** The DAG is protocol infrastructure, not build infrastructure. Same patterns serve merge proposals, fork graphs, attestation accumulation. Build-specific semantics stay in rakia.

2. **Brit owns the CLI.** Brit replaces gix as the command-line tool. `brit graph`, `brit affected`, `brit plan` are subcommands. `rakia build` and `rakia ci` are separate — execution, not computation.

3. **B -> C -> A ordering.** Build the working code first, prove parity with the Groovy walker, then formalize with schemas. The schema pass is the IoC cleanup that forces consistency.

4. **AffectedBy provenance.** Every affected node knows why it was affected. This replaces Jenkins' opaque "this pipeline is stale" with debuggable reasoning.

5. **Baselines are git refs, not artifacts.** Dissolves baseline leapfrog, cold-start oscillation, and post-block continuity bugs by construction.

6. **No git CLI shell-outs.** All git operations through brit/gix object store. Deterministic, testable, no subprocess parsing.

7. **Git compatibility maintained.** Every operation produces valid git state. `git clone` from any forge works. Trailers are RFC-822. Manifests are files. Refs are refs.
