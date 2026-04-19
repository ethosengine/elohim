# Rakia Describable via CLI + Schema-as-IoC Sprint

**Date:** 2026-04-19
**Status:** Approved (brainstorming complete, awaiting spec review)
**Author:** Matthew Dowell + Claude Opus 4.7
**Predecessor spec:** `docs/superpowers/specs/2026-04-19-brit-graph-rakia-mvp-design.md`

## TL;DR

Make rakia *describable* — every primitive built in Phase B/C exposed through a brit CLI surface that an operator can actually invoke. Close the sprint with a **schema-as-IoC pass** that defines the input contract (BuildManifest) and output contract (BuildPlan) as rakia-owned JSON Schemas, generates Rust types from them, and replaces every `serde_json::Value` escape hatch with a typed field. Sprint isn't done when code works — sprint is done when the schema validates and the code conforms.

The end-of-sprint artifact is a single `docs/superpowers/sprint-results/2026-04-19-rakia-describable.md` document capturing what's runnable, what's next (rakia-runnable: executor + `rakia ci`), and any open questions surfaced by the IoC pass.

This sprint replaces shadow-mode validation. Jenkins is not a target consumer — it's legacy reference documentation for what problems rakia must solve. The upgrade path is rakia replacing Jenkins, not rakia plugging into Jenkins.

**Architectural placement:** Brit (the local CLI fork of gitoxide) sits outside the P2P layer. Rakia (the build/CI application that brit-cli surfaces) IS in the P2P layer — its long-term form is peer-dispatched builds with threshold attestations across the swarm. This sprint ships local-only describability, but the schemas it locks in are the durable wire contracts that future peer dispatch will gossip. Designing them as casual local artifacts now would force breaking re-design later. See the **P2P Design Gate Classification** section for per-entity classification at both this-sprint and Stage 2 horizons.

## The Sprint Cadence Pattern (Standing Discipline)

Every rakia sprint follows this rhythm, not just this one:

1. **Build pragmatically through the sprint** — implement against tests, ship working code
2. **Close with schema IoC pass** — formalize the new contracts as JSON Schema, generate types from schema, replace escape hatches
3. **Drift between schema and code surfaces as work** — anything that doesn't conform is either a code bug or a schema gap; both get fixed before sprint close

The schema file is the sprint's permanent record of "this is what we agreed the contract is." Hand-writing types that mirror schemas is forbidden — it's how you discover all your mistakes when the schema starts enforcing. Generated types only.

For this sprint: BuildManifest input contract + BuildPlan output contract.
For next sprint (rakia-runnable): BuildAttestation output contract + ExecutionEvent contract.

## Sprint Scope

| In | Out (next sprint or later) |
|---|---|
| `brit-cli` crate with 8 subcommands | rakia-executor (build execution) |
| `build-plan.schema.json` (output contract, rakia-owned, NEW) | `rakia ci` wrapper command |
| `build-manifest.schema.json` (input contract, rakia-owned, ported from genesis) | Fold `brit-verify`/`brit-build-ref` into unified `brit` binary |
| Rakia codegen pipeline → Rust types in rakia-core | TypeScript codegen distribution (no TS consumer yet) |
| Replace `serde_json::Value` escapes (`gate`, `deployment`, `executor`) with typed fields | Peer dispatch / Stage 2 |
| Fixture-based regression tests for the 8 existing manifests | Migrating Jenkinsfiles |
| Schema validation in pre-push hook | |
| Sprint-result artifact in `docs/superpowers/sprint-results/` | |

## Architecture

### Schema Home: Rakia-Owned

```
elohim/rakia/
├── schemas/
│   ├── v1/
│   │   ├── build-manifest.schema.json    # input contract (port from genesis/orchestrator/)
│   │   └── build-plan.schema.json         # output contract (NEW)
│   └── scripts/
│       ├── codegen-rs.mjs                 # JSON Schema → Rust structs (extends sdk pattern)
│       └── validate.mjs                   # validate all 8 build-manifest.json files
├── rakia-core/
│   └── src/
│       ├── generated_types.rs             # AUTO-GENERATED — do not edit
│       ├── manifest.rs                    # uses generated types, no serde_json::Value
│       └── ...
└── package.json                           # rakia:codegen:rs, rakia:schema:validate scripts
```

**Why rakia-owned, not sdk-owned:** Build-domain schemas are *meaning-defined interpretations* of EPR core primitives. The SDK owns protocol primitives (ContentNode, BritCid, attestation envelope). Rakia owns build-system semantics (BuildManifest, BuildPlan, BuildAttestation). Coupling the SDK to build-system concerns would conflate layers. Rakia-owned schemas keep the SDK focused and let rakia evolve its contracts independently.

### Codegen: Extend the Existing Node-Script Pattern

The monorepo already has two relevant patterns:

| Existing | Path | Scope |
|---|---|---|
| `schema:codegen:rs` | `elohim/sdk/schemas/scripts/codegen-rs.mjs` | Enum constants only (`CORE_*`, `ALL_*`) |
| `lamad:codegen` | `elohim/sdk/domains/lamad/scripts/codegen.mjs` | Lightweight schema → TypeScript struct translator |

The `lamad:codegen` script demonstrates the lightweight schema-walker approach (no `json-schema-to-typescript` dependency, ~80 lines). We port that approach to Rust output and add it to the existing `codegen-rs.mjs` discipline (`--verify` mode, rustfmt, write to `.rs` file checked into git).

**Why not `typify` or `schemafy`:** Those are build.rs tools. The existing repo discipline favors generated artifacts checked into git (visible in code review, no invisible build-time magic). Extending the node-script pattern keeps consistency with how every other generated type in this monorepo works.

**Codegen output conventions:**
- Output file: `elohim/rakia/rakia-core/src/generated_types.rs`
- Header: `//! AUTO-GENERATED from rakia/schemas/v1/. Do not edit.`
- Derives: `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]` + `#[serde(rename_all = "camelCase")]`
- `type: object` + properties → `pub struct`
- `type: object` + additionalProperties only (no properties) → `BTreeMap<String, T>`
- `type: array` → `Vec<T>`
- `type: string` enum → Rust enum with `#[serde(rename_all = "kebab-case")]` (or whatever matches schema values)
- `required` fields → `T`; non-required → `Option<T>` or `#[serde(default)]`
- `$ref` → reference to another generated type (cross-file refs supported)
- `$defs` → nested types in same file

**Verification mode:** `node codegen-rs.mjs --verify` runs codegen to a tmp file, runs `rustfmt`, compares against the checked-in file. CI/pre-push hook fails on drift.

### BuildManifest Schema (Port + Refine)

`genesis/orchestrator/manifest.schema.json` (148 lines) already defines the input contract well. The port is:

1. Copy to `elohim/rakia/schemas/v1/build-manifest.schema.json`
2. Update `$id` to `epr:schema:rakia:build-manifest:v1`
3. Refine three sections that are currently loose:
   - **`gate`** — currently `additionalProperties: false` with empty `properties`. Define what `gate.projects` actually means based on real usage in the 8 manifests.
   - **`deployment`** — define `targets` properly (currently just `{ healthCheck: uri }`).
   - **`executor`** — currently NOT in the schema at all (BuildStep doesn't list `executor` as a property). Add it as an optional discriminated union: `{ kind: "shell" | "rust-cargo" | "pnpm" | ... , ...kind-specific-fields }` based on what the 8 manifests actually use.
4. Validate the 8 existing manifests against the refined schema. Anything that fails is either a manifest bug (fix the manifest) or a schema gap (refine the schema).

After the IoC pass, `BuildManifest` in rakia-core has zero `serde_json::Value` fields.

### BuildPlan Schema (New, Output Contract)

The upgrade target — what `brit plan` returns. Designed for machine consumers (future `rakia ci`, dashboards, peer dispatch) and human inspection (pretty JSON).

```json
{
  "$id": "epr:schema:rakia:build-plan:v1",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "BuildPlan",
  "type": "object",
  "required": ["planVersion", "baseline", "head", "levels", "generatedAt"],
  "properties": {
    "planVersion": { "type": "string", "const": "1.0" },
    "baseline": {
      "type": "object",
      "required": ["ref", "commit"],
      "properties": {
        "ref": { "type": "string", "description": "e.g. refs/notes/rakia/baselines/elohim" },
        "commit": { "type": "string", "description": "git commit SHA-1 hex" }
      }
    },
    "head": {
      "type": "object",
      "required": ["commit"],
      "properties": { "commit": { "type": "string" } }
    },
    "changedPaths": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Workspace-relative paths that differ between baseline and head"
    },
    "levels": {
      "type": "array",
      "items": {
        "type": "array",
        "items": { "$ref": "#/$defs/plannedStep" }
      },
      "description": "Topologically grouped — level 0 is parallelizable with no deps"
    },
    "generatedAt": { "type": "string", "format": "date-time" },
    "tool": {
      "type": "object",
      "properties": {
        "name": { "type": "string", "const": "brit" },
        "version": { "type": "string" }
      }
    }
  },
  "$defs": {
    "plannedStep": {
      "type": "object",
      "required": ["pipeline", "name", "fingerprint", "affectedBy"],
      "properties": {
        "pipeline": { "type": "string" },
        "name": { "type": "string" },
        "qualifiedName": { "type": "string", "description": "pipeline:name" },
        "fingerprint": { "type": "string", "description": "BritCid hex" },
        "depends": {
          "type": "array",
          "items": { "type": "string", "description": "qualified names" }
        },
        "affectedBy": {
          "type": "array",
          "items": { "$ref": "#/$defs/affectedReason" }
        }
      }
    },
    "affectedReason": {
      "oneOf": [
        { "type": "object", "required": ["kind", "path"], "properties": {
            "kind": { "const": "changedFile" }, "path": { "type": "string" } } },
        { "type": "object", "required": ["kind", "upstream"], "properties": {
            "kind": { "const": "upstreamNode" }, "upstream": { "type": "string" } } },
        { "type": "object", "required": ["kind"], "properties": {
            "kind": { "const": "inputFingerprint" } } },
        { "type": "object", "required": ["kind"], "properties": {
            "kind": { "const": "alwaysAffected" } } }
      ]
    }
  }
}
```

This shape is intentional:
- Stable for machine parsing (`planVersion` discriminates future revisions)
- Provenance preserved (`affectedBy` per step — debuggable "why is this in the plan?")
- Topological grouping (`levels`) directly drives parallel scheduling
- Includes baseline/head metadata so the plan is self-describing without needing the repo state

### Brit CLI Surface

New crate `elohim/brit/brit-cli` added to the brit workspace. Single binary, clap-based, subcommand structure.

```bash
brit graph discover [--repo <path>]
    # List all build-manifest.json files and their pipelines+steps. JSON output.

brit graph show [--repo <path>] [--format json|dot]
    # Full constellation (build DAG). JSON for machines, DOT for Graphviz.

brit affected --files <comma-sep-paths> [--repo <path>]
brit affected --since <ref> [--repo <path>]
    # Which steps are affected and why (full AffectedBy provenance). JSON output.

brit plan --files <comma-sep-paths> [--repo <path>]
brit plan --since <ref> [--repo <path>]
    # Topologically grouped build plan, conforming to build-plan.schema.json.

brit fingerprint <manifest-path> [--step <name>] [--repo <path>]
    # Deterministic ContentFingerprint of step inputs (BritCid hex).

brit baseline read <pipeline> [--repo <path>]
    # Show current baseline ref content (commit SHA).

brit baseline write <pipeline> <commit-sha> [--repo <path>]
    # Set/update the baseline ref for a pipeline.

brit baseline migrate <pipeline-baselines.json> [--repo <path>]
    # One-shot migration from Jenkins pipeline-baselines.json artifact.
```

**CLI conventions:**
- Default output: pretty JSON to stdout
- `--format` flag where multiple output forms make sense (`graph show`)
- `--repo <path>` for explicit repo selection (default: discover from cwd)
- Errors: human-readable to stderr, exit code 1; JSON error envelope when `--json-errors` (for machine callers)
- `--verbose` for debug logging to stderr (never to stdout — stdout is the contract)
- All commands are pure (no execution, no mutation outside `baseline write/migrate`)

**Existing binaries (`brit-verify`, `brit-build-ref`) stay standalone this sprint.** Folding them into a unified `brit` binary is deferred — it's UX polish, not contract work.

### Fixture-Based Regression Tests

Replace shadow-mode validation with **fixture tests in rakia-core/tests/**. Each fixture is `(manifests-snapshot, changed-paths, expected-plan)` and asserts that `plan_from_changes` produces the expected output.

```
elohim/rakia/rakia-core/tests/
├── fixtures/
│   ├── 01-elohim-app-css-change/
│   │   ├── changed-paths.json          # ["app/elohim-app/src/styles.scss"]
│   │   └── expected-plan.json          # conforms to build-plan.schema.json
│   ├── 02-sophia-source-change/        # transitive: sophia → elohim-app
│   ├── 03-holochain-dna-change/
│   ├── 04-cross-pillar-change/         # multiple pipelines affected
│   ├── 05-no-change/                   # empty plan
│   ├── 06-readme-only/                 # no pipeline affected
│   ├── 07-jenkinsfile-only/            # orchestrator pipeline only
│   └── 08-build-process-change/        # tests buildProcess input matching (currently a gap)
├── fixture_runner.rs                    # parameterized test, reads fixtures dir
└── manifests-snapshot/                  # frozen copy of 8 manifests at sprint start
```

The 8 manifests are frozen at sprint start and snapshot-tested. If anyone changes a real manifest mid-sprint, fixtures don't break — they're isolated. Updating fixtures is a deliberate act.

Initial fixture set: 8 cases drawn from real Groovy fix commits (cases that previously caused production CI bugs are the most valuable). The exact 8 are picked during implementation — sprint plan handles selection.

### Schema Validation

Two validation entry points:

```bash
pnpm run rakia:schema:validate
    # Validates all build-manifest.json files in the worktree against build-manifest.schema.json
    # Fails on any non-conforming manifest. Run in pre-push hook for relevant projects.

pnpm run rakia:codegen:rs --verify
    # Verifies generated_types.rs is up-to-date with schemas. Pre-push hook integration.
```

Both wired into `.husky/pre-push` for changes touching `elohim/rakia/**` or any `build-manifest.json`.

### Sprint-Result Artifact

End-of-sprint deliverable: `docs/superpowers/sprint-results/2026-04-19-rakia-describable.md`

Required sections:

1. **What's runnable now** — concrete CLI demo transcript using the live binary against a real repo state. Copy-pasteable commands and abbreviated outputs.
2. **Schema-IoC pass results** — what was inconsistent before the IoC pass; what was tightened; what `serde_json::Value` escapes were eliminated.
3. **What's next: rakia-runnable scope** — the executor + `rakia ci` wrapper. Acceptance criteria for next sprint.
4. **Open questions surfaced** — anything the IoC pass uncovered that needs decision (e.g., should `gate.projects` be discriminated by project type?). These become brainstorming inputs for the next sprint.
5. **Carry-overs** — known follow-ups that didn't fit (the original list: `O(N*A)` traversal, `GlobSet` precompilation, `AffectedBy::DownstreamNode` purpose, gix typed `NotFound` upstream).

## Acceptance Criteria

### CLI Surface

- [ ] `brit-cli` crate compiles, ships a single `brit` binary
- [ ] All 8 subcommands implemented and produce JSON conforming to documented shape
- [ ] `brit graph show --format dot` produces valid Graphviz output
- [ ] `brit plan` output validates against `build-plan.schema.json`
- [ ] `brit affected --since <ref>` and `brit affected --files <list>` produce equivalent results when paths match
- [ ] `brit baseline write` produces a valid git ref readable by stock git tooling
- [ ] All commands handle missing repos, malformed manifests, unknown pipelines with clear errors and exit code 1

### Schemas (Rakia-Owned)

- [ ] `elohim/rakia/schemas/v1/build-manifest.schema.json` exists with `$id: epr:schema:rakia:build-manifest:v1`
- [ ] `elohim/rakia/schemas/v1/build-plan.schema.json` exists with `$id: epr:schema:rakia:build-plan:v1`
- [ ] All 8 existing `build-manifest.json` files validate against the schema
- [ ] `gate`, `deployment`, `executor` are properly typed in the schema (not free-form objects)

### Codegen

- [ ] `pnpm run rakia:codegen:rs` regenerates `rakia-core/src/generated_types.rs`
- [ ] `pnpm run rakia:codegen:rs --verify` fails on schema/code drift
- [ ] `BuildManifest` in `manifest.rs` has zero `serde_json::Value` fields — uses generated types
- [ ] Generated file has clean rustfmt output and compiles standalone
- [ ] Pre-push hook runs `--verify` for changes touching `elohim/rakia/schemas/**` or `elohim/rakia/rakia-core/src/manifest.rs`

### Fixture Tests

- [ ] At least 8 fixtures exist covering: single-pillar change, transitive change, cross-pillar, no-op, README-only, Jenkinsfile-only, buildProcess-input change, and one historical Groovy bug case
- [ ] `cargo test -p rakia-core fixture` runs all fixtures and they pass
- [ ] Fixture expected-plans validate against `build-plan.schema.json`

### Sprint-Result Artifact

- [ ] `docs/superpowers/sprint-results/2026-04-19-rakia-describable.md` exists
- [ ] Contains all 5 required sections
- [ ] Demo transcript is reproducible (anyone can run the same commands and get equivalent output)
- [ ] Next-sprint scope (rakia-runnable) is concrete enough to brainstorm against

## Out of Scope

| Item | Why deferred | Where it goes |
|---|---|---|
| `rakia-executor` (running build steps) | Needs its own design — execution semantics, parallelism, failure handling | Next sprint (rakia-runnable) |
| `rakia ci` wrapper command | Depends on executor | Next sprint |
| Migrating any Jenkinsfile | Requires `rakia ci` | After next sprint |
| Folding `brit-verify`/`brit-build-ref` into unified binary | UX polish, not contract work | Later, lower priority |
| TypeScript codegen for rakia schemas | No TS consumer yet | When dashboard/UI work begins |
| Peer dispatch of build steps | Requires steward swarm integration | Stage 2 |
| Threshold attestation across builders | Requires multiple builders | Stage 2 |
| Brit `ContentNode` adapter for git objects | Independent brit Phase 2 work | Parallel track, no dependency |
| `planFingerprint: BritCid` field on `BuildPlan` (content-addressed plan identity for peer dispatch) | Schema designed to accommodate it (instance metadata already separated from identity fields), but not load-bearing until peer dispatch lands | Rakia-peer (Stage 2) |
| Threshold attestation envelope around baseline writes | Single-node baselines work fine today; threshold required only for multi-builder swarm | Rakia-peer (Stage 2) |

## Relationship to Predecessor Spec

Refines but does not contradict `docs/superpowers/specs/2026-04-19-brit-graph-rakia-mvp-design.md`:

- **Phase A (schema formalization)** is reframed: not a single end-of-project IoC pass, but a *recurring* end-of-sprint discipline. This sprint does the first pass.
- **Phase A schema work** in this sprint is scoped to BuildManifest + BuildPlan. Future schemas (BuildAttestation, ExecutionEvent) come in their own sprints when their codepaths exist.
- **CLI surface** in the predecessor spec is brought forward into this sprint (was deferred earlier).
- **Shadow-mode validation** is removed from the roadmap entirely. Replaced with fixture-based regression tests against the 8 real manifests. Jenkins is requirements-gathering, not a target consumer.
- **Phase B (DAG)** and **Phase C (change detection)** remain complete and merged.

## P2P Design Gate Classification

**Architectural boundary:** Brit is a local CLI on the git substrate (outside the P2P layer). Rakia IS in the P2P layer — its long-term architecture is peer-dispatched builds with threshold attestations across the swarm. This sprint ships local-only describability, but every schema we author here is the durable wire contract that peers will eventually exchange. Designing them as casual local artifacts now would force a breaking re-design when peer dispatch lands.

Every entity is classified at two horizons: **this sprint** (local CLI artifacts) and **rakia-peer (Stage 2)** (P2P dispatch + threshold attestation). The classification at "this sprint" tells us what we ship; the classification at "Stage 2" constrains how we shape the schemas now.

### Entity: BuildManifest

- **This sprint classification:** Operational (C) — a file in the git repo, parsed locally per invocation.
- **Stage 2 classification:** Operational (C) still — manifests stay in the git repo. The git tree IS the source of truth, and brit/git compat means peers fetch them via standard git operations (clone/fetch). Manifests are SHARED inputs that all peers see consistently because they share the git substrate.
- **Justification:** No notarization need — the git commit hash addresses the manifest unambiguously. Two peers looking at the same `(commit, path)` see the same manifest. The git layer below provides P2P-safe sharing.
- **Content address strategy:** Slug-by-path within a git tree. The git commit hash provides the cryptographic addressing.
- **Source of truth:** Git repository (which itself is P2P-replicable via brit/git)
- **Wire format implication:** `build-manifest.schema.json` is the contract for what every peer expects to find when parsing this file. The schema is the agreement.
- **Anti-pattern check:** None apply. Source-of-truth documented (git tree).

### Entity: BuildPlan

- **This sprint classification:** Operational (C) — ephemeral CLI output, never persisted, never sent over the wire.
- **Stage 2 classification:** Notarized-attestation candidate (A2) — the BuildPlan IS the agreement that peers dispatch against. Multiple peers receiving the same plan must compute identical results to attest. The plan itself is content-addressable (deterministic function of constellation + baseline + head + changed_paths).
- **Justification:** Today it's local computation. Tomorrow it's the gossip message that announces "I want these steps built; here are the fingerprints; here's my baseline." Designing the schema casually now means re-designing the wire format later.
- **Content address strategy (today):** Slug/UUID (not addressed — stdout pipe to next process).
- **Content address strategy (Stage 2):** Content-derived (`planFingerprint: BritCid`) — hash of the deterministic identity fields (`baseline`, `head`, `levels[].fingerprint`, `levels[].depends`). Excludes `generatedAt` and `tool.version` (instance metadata, not identity). Two peers computing the same plan from the same inputs MUST produce identical fingerprints — that's the dispatch coordination primitive.
- **Source of truth (today):** Computation. **Source of truth (Stage 2):** Computation, with optional notarization via attestation when the plan becomes a peer-shared agreement.
- **Wire format implication:** This sprint's schema choices ARE the future wire contract. Specifically: the schema must remain deterministically serializable (sorted maps, no insertion-order deps, no host-specific metadata in identity-relevant fields). The current schema satisfies this — the `generatedAt` and `tool` fields are intentionally separate from identity fields.
- **Sprint-scope deferred work:** Adding `planFingerprint: BritCid` field to `BuildPlan`. Not blocking describability. Carry-over for the rakia-peer sprint where the field becomes load-bearing.
- **Anti-pattern check:** None apply, given the schema separates instance metadata (`generatedAt`, `tool`) from identity-relevant fields.

### Entity: Baseline ref (`refs/notes/rakia/baselines/{pipeline}`)

- **This sprint classification:** Operational (C) — git ref read/written locally; brit/git compat preserved.
- **Stage 2 classification:** Notarized via attestation (A2) — when the network advances a baseline, peers need verifiable agreement that "commit X is the new baseline for pipeline Y." This will use the existing `Build` attestation entry type in `brit-epr/src/elohim/attestation/`. The git ref remains the local cache; the attestation is the network truth.
- **Justification today:** Single-node CI. The git ref is sufficient. **Justification Stage 2:** Multi-builder swarm. A baseline change must be witnessed by enough peers (threshold) to prevent a rogue node from rolling baselines back or skipping pipelines.
- **Content address strategy:** Slug (pipeline name → ref path) for the local cache. Stage 2 attestation is content-addressed by BritCid of the attestation envelope.
- **Source of truth (today):** Git ref namespace. **Source of truth (Stage 2):** Threshold attestation set; git ref is the local materialization.
- **Wire format implication:** None this sprint. Attestation envelope schema is brit-epr's existing contract.
- **Anti-pattern check:** None apply.

### Design constraints surfaced by the gate

| Layer | Source of truth (this sprint) | Source of truth (Stage 2) | P2P transport |
|---|---|---|---|
| BuildManifest | Git tree | Git tree (unchanged) | brit/git replication |
| BuildPlan | Local computation | Same computation, optionally + threshold attestation envelope | libp2p direct dispatch (gossip the plan, peers attest results) |
| Baseline | Git ref (local) | Threshold attestation set; git ref is local cache | brit-epr Build attestation |
| BuildAttestation (next sprint) | N/A — doesn't exist yet | Notarized (A) via existing brit-epr `Build` attestation entry type | brit-epr attestation transport |
| ExecutionEvent (next sprint) | N/A | Operational (C) — ephemeral logs, agent-scoped | None — local to executor |
| BuilderPresence (next sprint) | N/A | Agent-scoped + attestation (B2) — "I'm available to build" presence | libp2p gossip / brit-epr presence |

**Why this matters for rakia-vs-brit and rakia-vs-SDK separation:**

Rakia composes protocol primitives (brit-epr `Build` attestation, agent presence, etc.) into build-domain semantics. It doesn't add new protocol primitives — it consumes them. That's why the schemas stay rakia-owned (build-domain meaning) even though rakia operates over the P2P network. The SDK still owns the protocol-core primitives; rakia is the application that uses them for build/CI.

The brit/rakia split is now precisely: **brit is the local substrate (git + EPR primitives, including attestation)**, **rakia is the distributed application that orchestrates builds across the network using those primitives**. Brit-cli is local because the operator surface is local; the network behavior happens through rakia's executor (next sprint) and rakia-peer (Stage 2).

**Implication for this sprint's schema design:**

1. **BuildPlan schema MUST remain deterministically serializable** — no insertion-order maps, no host-specific identity fields. ✓ (Already designed this way.)
2. **`generatedAt` and `tool.version` are explicitly outside the future plan-identity hash** — documented above as instance metadata. ✓ (Already separated.)
3. **`planFingerprint: BritCid` field is a known future extension** — added to spec carry-overs.
4. **`executor.kind` must remain an extensible discriminated union**, NOT a closed enum locked to today's five kinds. Stage 2 and beyond will add at minimum `experienceStory` (a2o BDD scenarios), and likely `containerImage`, `composition` (multi-peer choreography), others. Closing the enum now would force a schema-version bump per new execution kind, fragmenting the contract. **Action this sprint:** use the discriminated-union pattern (`kind` + kind-specific fields), but leave `kind` as `type: string` without an `enum` constraint — OR include the enum with an explicit comment that it's extensible and peers tolerating unknown kinds is the expected behavior. The planner task that refines the executor definition (Task 3.4 in the plan) should favor extensibility over closure.
5. **No premature P2P infrastructure** — we don't add gossip, attestation envelopes, builder presence, peer-diversity fields, or compensation fields this sprint. Those land when the executor sprint produces real build outputs to attest about and when multi-peer dispatch becomes the execution path.

## Rakia Trajectory (Forward Context, Non-Binding)

This section documents the long-term vision that rakia's schema design must not paint itself out of. None of it is in scope for this sprint, but every decision here is checked against "does this close a door we'll want open later?"

### Execution modes, weakest to strongest

| Mode | Peers involved | Dispatch | Compensation | Protocol primitives composed |
|---|---|---|---|---|
| **Local** (this sprint + next) | 1 (you) | None — brit-cli local, rakia-executor local | N/A | None beyond brit/git and brit-epr attestations |
| **Same-work threshold** | N peers run the SAME step, results compared | `BuildPlan` gossiped; any willing peer accepts | Free-in-kind (reciprocal) | `Build` attestation + peer presence + compute substrate signal |
| **Distributed choreography** | N peers cooperate in ONE logical run (e.g., a2o P2P-sync scenario needs 2 peers by definition of the test) | Role-assigned by plan | Free-in-kind OR mutual credit | Agreement + Commitment + `compute` signal + attestation |
| **Contracted compute** | Any peers honoring an Agreement | Open bid OR directed assignment | Unyt / shefa REA flows | Full REA: Agreement → Commitment → EconomicEvent + substrate signal + fulfillment attestation |

### Experience-stories as first-class execution units

The a2o BDD scenarios in `genesis/a2o/features/**/*.feature` are not just tests — they are **the meaningful unit of work** rakia eventually dispatches. A story may require a single peer (unit-like test), multiple identical peers (hardware-profile diversity), or multiple role-differentiated peers (choreography). The `BuildStep` abstraction generalizes: a step is a work unit with inputs, outputs, executor, and (eventually) participation requirements. Builds and stories share the plan/dispatch/attest machinery; they differ in executor kind.

### Rakia as shefa/REA participant

Running a build or story consumes **compute** — one of the seven substrate signals (`attention, compute, storage, bandwidth, energy, time, resource`). When a peer executes a rakia job, they emit a compute substrate signal that feeds into shefa's economic accounting. Paid execution is mediated by an `Agreement` (REA primitive) with `Commitment`s from willing executors; fulfillment produces `EconomicEvent`s and a `Build` attestation. Free-in-kind execution is the same machinery with a null compensation leg — reciprocity tracked but uncontracted.

### Attestation flow: "artifact X verified by N peers"

The verification model composes brit-epr primitives that already exist; rakia adds NO new attestation type. The flow:

1. **Per-peer execution** — each willing peer runs the step against the same inputs and produces an outcome (artifact CID, test result, log fingerprint).
2. **Per-peer attestation** — each peer emits a `Build` attestation: "I, peer P, executed step S of plan Π and obtained outcome O at time T." Schema: existing `brit-epr/src/elohim/attestation/build.rs`. Signed by the peer's `AgentKey`. Anchored on the git commit (head) being built.
3. **Convergence check** — outcomes are compared; peers that disagree fork the attestation set (an Inflexionable signal — "we built the same thing and got different answers" is information).
4. **Reach computation** — `brit-epr/src/elohim/attestation/reach.rs` already computes reach (private → self → intimate → ... → commons) over an attestation set. "Verified by N peers" is just reach-of-N at some level.
5. **Composite witness** — when reach reaches the threshold the plan declared (e.g., `quorum: { peers: 3, profiles: ["browser","steward"] }`), a composite attestation references the underlying per-peer attestations: "Artifact X verified at reach R by attestations [A1, A2, A3]." This composite is what advances the baseline (`brit baseline write` → `rakia baseline advance` at Stage 2).
6. **Disputes** — divergent outcomes don't get a composite. The plan re-dispatches, or the divergence becomes a signal (probably a discernment-gate trigger: "this artifact is contested across peers, route to humans").

What this means today: **nothing new for THIS sprint.** The brit-epr Build-attestation + reach machinery already exists. This sprint's BuildPlan schema must carry enough plan-identity information (`planFingerprint`, `levels[].fingerprint`, baseline/head commits) that future per-peer attestations can reference *which plan they were honoring*. The current schema satisfies this. Next sprint (rakia-executor) emits the single-peer attestation; Stage 2 (rakia-peer) adds threshold composition.

What this means for plan design: a `BuildPlan` IS a peer-shareable contract. Every peer accepting a step from the plan is committing to honor the plan-as-stated. The plan's `planFingerprint` (Stage 2 field) is the handle on which attestations anchor. If we sloppy the schema now, we sloppy the future attestation graph.

### What this forces on the schema design NOW

- `executor.kind` extensible (see implication 4 above)
- `BuildStep` shape must allow future optional fields for participation requirements (`peerDiversity`, `compensation`, `quorum`) without breaking today's manifests — meaning `additionalProperties` on step and executor stays `false` BUT the schema is versioned (bump `manifestVersion` when extending)
- `BuildPlan` identity fields deterministic (already ✓) so peers compute identical fingerprints when dispatching the same plan
- No field today claims authority that Stage 2 will need to displace — e.g., baseline advancement is a local git-ref write today, but the CLI surface doesn't lock out future threshold-attestation-witnessed advancement (`brit baseline write` stays, Stage 2 adds `rakia baseline advance` with attestation wrapping)

### What this forces on next sprint's design

- `rakia-executor` must implement `executor.kind` as an extensible dispatch table (new kinds plug in without refactoring)
- `ExecutionEvent` schema (next sprint's IoC pass) needs to carry enough metadata to become the compute substrate signal envelope at Stage 2
- `BuildAttestation` schema (next sprint's IoC pass) is the bridge to brit-epr's existing `Build` attestation primitive — hand off to the primitive, don't re-invent

None of this is work for this sprint. It is the frame against which this sprint's decisions are checked.

## Key Design Decisions

1. **Schemas are rakia-owned, not sdk-owned.** Build-domain semantics are meaning-defined interpretations of EPR primitives; SDK stays focused on protocol primitives.

2. **Generated Rust types from JSON Schema, not hand-written.** Hand-writing produces drift that surfaces as bugs when schema enforcement turns on. Generation is the IoC mechanism.

3. **Extend the existing node-script codegen pattern, don't introduce build.rs.** Generated artifacts are checked into git, visible in review, no invisible compile-time magic. Consistent with every other generated type in this monorepo.

4. **Schema-as-IoC is a recurring sprint discipline, not a one-shot.** Every sprint closes with formalization of new contracts. Drift surfaces as work, not as incident.

5. **Fixture tests over shadow-mode.** Groovy is buggy reference (53 fix commits). Fixture tests against real manifests give cleaner regression signal without enshrining Groovy bugs.

6. **CLI is the operator surface, not the Jenkins integration.** Designed for the post-Jenkins world — humans, future `rakia ci`, machine consumers. Jenkins gets retired, not integrated.

7. **End-of-sprint artifact in `docs/superpowers/sprint-results/`.** Sprint produces a permanent record of what was built and what's next. Artifact is a sprint contract, like the schema.
