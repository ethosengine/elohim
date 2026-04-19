# Rakia Describable via CLI + Schema-as-IoC Sprint

**Date:** 2026-04-19
**Status:** Approved (brainstorming complete, awaiting spec review)
**Author:** Matthew Dowell + Claude Opus 4.7
**Predecessor spec:** `docs/superpowers/specs/2026-04-19-brit-graph-rakia-mvp-design.md`

## TL;DR

Make rakia *describable* — every primitive built in Phase B/C exposed through a brit CLI surface that an operator can actually invoke. Close the sprint with a **schema-as-IoC pass** that defines the input contract (BuildManifest) and output contract (BuildPlan) as rakia-owned JSON Schemas, generates Rust types from them, and replaces every `serde_json::Value` escape hatch with a typed field. Sprint isn't done when code works — sprint is done when the schema validates and the code conforms.

The end-of-sprint artifact is a single `docs/superpowers/sprint-results/2026-04-19-rakia-describable.md` document capturing what's runnable, what's next (rakia-runnable: executor + `rakia ci`), and any open questions surfaced by the IoC pass.

This sprint replaces shadow-mode validation. Jenkins is not a target consumer — it's legacy reference documentation for what problems rakia must solve. The upgrade path is rakia replacing Jenkins, not rakia plugging into Jenkins.

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

## Relationship to Predecessor Spec

Refines but does not contradict `docs/superpowers/specs/2026-04-19-brit-graph-rakia-mvp-design.md`:

- **Phase A (schema formalization)** is reframed: not a single end-of-project IoC pass, but a *recurring* end-of-sprint discipline. This sprint does the first pass.
- **Phase A schema work** in this sprint is scoped to BuildManifest + BuildPlan. Future schemas (BuildAttestation, ExecutionEvent) come in their own sprints when their codepaths exist.
- **CLI surface** in the predecessor spec is brought forward into this sprint (was deferred earlier).
- **Shadow-mode validation** is removed from the roadmap entirely. Replaced with fixture-based regression tests against the 8 real manifests. Jenkins is requirements-gathering, not a target consumer.
- **Phase B (DAG)** and **Phase C (change detection)** remain complete and merged.

## P2P Design Gate Classification

The brit/rakia stack is intentionally **outside the P2P protocol layer** — it's developer tooling that operates on git repositories, not protocol-replicated data. Per the `p2p-design-gate` skill, every entity introduced by this sprint is classified below. None require DHT entry types, attestations, or storage projections.

### Entity: BuildManifest

- **Classification:** Operational (C)
- **Justification:** A build-configuration file (`build-manifest.json`) committed to the source repo. The git tree IS the source of truth. Not protocol data — developer tooling configuration. No notarization need; the manifest IS the source-code intent, version-controlled by git. Reconstructable from any git checkout.
- **Content address strategy:** Slug/UUID (file path on disk + pipeline name)
- **Address justification:** Manifests are files in a git repo addressed by `(commit, path)`. Brit/git compat means the git substrate handles addressing — no CID layer needed. Not content-derived (same content at different paths = different manifests). Not agent-scoped (repo-shared, not agent-private).
- **Source of truth:** Git repository (file on disk, version-controlled)
- **Coordinator zome:** N/A — no zome involvement; build system lives outside protocol primitives
- **Storage projection:** N/A — parsed in-memory from disk per invocation, no SQLite
- **HTTP route:** N/A — CLI tool, no HTTP surface
- **Anti-pattern check:** None apply. "REST route as design starting point" doesn't apply (no HTTP). Source-of-truth is documented (git tree).

### Entity: BuildPlan

- **Classification:** Operational (C)
- **Justification:** Ephemeral output of `brit plan`. Pure deterministic function of (constellation, changed_paths, baseline, head). Reconstructable on demand. Never persisted.
- **Content address strategy:** Slug/UUID (not addressed at all — streamed to stdout for one-shot machine consumption)
- **Address justification:** BuildPlan is never stored. The next-sprint `rakia ci` pipes it directly to the executor. No identity, no addressing.
- **Source of truth:** Computation
- **Coordinator zome:** N/A
- **Storage projection:** N/A
- **HTTP route:** N/A
- **Anti-pattern check:** None apply.

### Entity: Baseline ref (`refs/notes/rakia/baselines/{pipeline}`)

- **Classification:** Operational (C)
- **Justification:** Git ref tracking the last-known-good commit per pipeline. Git ref namespace IS the source of truth — survives executor death because it's a ref, not an artifact (which is precisely the design improvement vs Jenkins's `pipeline-baselines.json`).
- **Content address strategy:** Slug (pipeline name → ref path). Justified because git ref naming conventions require human-readable paths.
- **Source of truth:** Git ref namespace under `refs/notes/rakia/baselines/`
- **Coordinator zome:** N/A
- **Storage projection:** N/A — the git ref IS the storage
- **HTTP route:** N/A — accessed via brit CLI which uses brit/gix
- **Anti-pattern check:** None apply. Brit/git compat preserved (these are valid stock git refs; `git show-ref` can read them).

### Design constraints discovered

The classification work confirms a crisp architectural boundary:

| Layer | Where data lives | Source of truth | Subject to p2p-design-gate? |
|---|---|---|---|
| Protocol primitives (Content, Mastery, Attestation, EconomicEvent, ...) | Holochain DHT | DHT entry types | Yes — every entity needs full classification |
| Build/CI domain (BuildManifest, BuildPlan, baselines) | Git repo + git refs | Git tree + ref namespace (brit/git compat) | Classified once here as Category C — sprint scope is closed under "operational" |
| Developer dev-loop (codegen artifacts, fixtures, generated_types.rs) | Files on disk | Schemas (regenerated) | N/A — derived artifacts, no entity status |

This boundary mirrors the SDK CLAUDE.md test ("Could this capability be captured at scale for rent extraction?"). Build/CI cannot — anyone with a git repo can run `brit`. There is no "the build authority" to capture. So it lives outside the protocol, in developer-tooling space, and the gate's notarization machinery does not apply.

**Implication for future rakia sprints:** new entities introduced by `rakia-executor` (ExecutionEvent) and the cutover work (BuildAttestation) WILL hit the gate again — BuildAttestation is Category A (notarized via the existing `Build` attestation entry type in `brit-epr/src/elohim/attestation/`), and ExecutionEvent is Category C (operational, ephemeral logs). That classification belongs to the next sprint's spec, not this one.

## Key Design Decisions

1. **Schemas are rakia-owned, not sdk-owned.** Build-domain semantics are meaning-defined interpretations of EPR primitives; SDK stays focused on protocol primitives.

2. **Generated Rust types from JSON Schema, not hand-written.** Hand-writing produces drift that surfaces as bugs when schema enforcement turns on. Generation is the IoC mechanism.

3. **Extend the existing node-script codegen pattern, don't introduce build.rs.** Generated artifacts are checked into git, visible in review, no invisible compile-time magic. Consistent with every other generated type in this monorepo.

4. **Schema-as-IoC is a recurring sprint discipline, not a one-shot.** Every sprint closes with formalization of new contracts. Drift surfaces as work, not as incident.

5. **Fixture tests over shadow-mode.** Groovy is buggy reference (53 fix commits). Fixture tests against real manifests give cleaner regression signal without enshrining Groovy bugs.

6. **CLI is the operator surface, not the Jenkins integration.** Designed for the post-Jenkins world — humans, future `rakia ci`, machine consumers. Jenkins gets retired, not integrated.

7. **End-of-sprint artifact in `docs/superpowers/sprint-results/`.** Sprint produces a permanent record of what was built and what's next. Artifact is a sprint contract, like the schema.
