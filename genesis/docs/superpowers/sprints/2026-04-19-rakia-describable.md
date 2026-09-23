# Sprint Result: Rakia Describable via CLI + Schema-as-IoC

**Date:** 2026-04-19 (sprint close)
**Spec:** `docs/superpowers/specs/2026-04-19-rakia-describable-cli-and-schema-ioc.md`
**Plan:** `docs/superpowers/plans/2026-04-19-rakia-describable-cli-and-schema-ioc.md`
**Branches:** `feat/brit-cli-rakia-codegen` (brit submodule), `feat/rakia-schemas-and-codegen` (rakia submodule), `dev` (parent)

## What's Runnable Now

Eight `brit` subcommands operate on the rakia constellation. Three work directly on the live repo today; five require a self-consistent constellation (the live repo's `elohim:build-angular` declares an unresolved dep on `elohim-sophia:build-sophia-umd` — see Open Questions). All eight work on the frozen 9-manifest fixture snapshot.

### `brit graph discover` (works on live repo)

```bash
$ brit graph discover --repo .
{
  "manifests": [
    {
      "path": "app/elohim-app/build-manifest.json",
      "pipeline": "elohim",
      "description": "Elohim Angular app — build, service worker, site image",
      "step_count": 3,
      "steps": ["build-angular", "build-site-image", "lint-library"]
    },
    ... (7 more)
  ]
}
```

### `brit graph show --format dot` (fixture mode)

Pipes into Graphviz for visualization:

```bash
$ brit graph show --repo <fixture-root> --format dot | dot -Tsvg -o constellation.svg
```

JSON output also available via `--format json`.

### `brit affected --files <paths>` (fixture mode)

Affected steps with structured provenance:

```bash
$ brit affected --repo <fixture-root> --files "proj-a/src/foo.ts"
{
  "changed_paths": ["proj-a/src/foo.ts"],
  "affected": [
    {
      "qualified_name": "proj-a:build",
      "affected_by": [{ "kind": "changedFile", "path": "proj-a/src/foo.ts" }]
    },
    {
      "qualified_name": "proj-b:package",
      "affected_by": [{ "kind": "upstreamNode", "upstream": "proj-a:build" }]
    }
  ]
}
```

### `brit plan --files <paths>` (fixture mode)

Topologically grouped BuildPlan, validated against `build-plan.schema.json`:

```bash
$ brit plan --repo <fixture-root> --files "proj-a/src/foo.ts"
{
  "planVersion": "1.0",
  "baseline": { "ref": "(none)", "commit": "0000000000000000000000000000000000000000" },
  "head": { "commit": "0000000000000000000000000000000000000000" },
  "changedPaths": ["proj-a/src/foo.ts"],
  "levels": [
    [{ "pipeline": "proj-a", "name": "build", "qualifiedName": "proj-a:build",
       "fingerprint": "f9db27f25918cf7b", "depends": [],
       "affectedBy": [{ "kind": "changedFile", "path": "proj-a/src/foo.ts" }] }],
    [{ "pipeline": "proj-b", "name": "package", "qualifiedName": "proj-b:package",
       "fingerprint": "...", "depends": ["proj-a:build"],
       "affectedBy": [{ "kind": "upstreamNode", "upstream": "proj-a:build" }] }]
  ],
  "generatedAt": "2026-04-19T...",
  "tool": { "name": "brit", "version": "0.0.0" }
}
```

`--since <ref>` mode also works: derives changed paths from gix-based git diff between the ref and HEAD, fills in baseline/head commit SHAs from the ref resolution.

### `brit fingerprint <manifest>` (works on live repo, single-manifest)

Deterministic content hash of step inputs:

```bash
$ brit fingerprint app/elohim-app/build-manifest.json --step build-angular
{
  "manifest": "app/elohim-app/build-manifest.json",
  "fingerprints": [
    {
      "pipeline": "elohim",
      "step": "build-angular",
      "fingerprint": "c7782530c015b34f02000d87e58d5f120575170727204a1b6a6e2e8db9a336de",
      "input_count": 9
    }
  ]
}
```

Two invocations produce byte-identical output (verified).

### `brit baseline read/write/migrate` (works on live repo)

Git-ref-backed baselines, survive executor death:

```bash
$ brit baseline write elohim <commit-sha> --repo .
{ "pipeline": "elohim", "ref": "refs/notes/rakia/baselines/elohim", "commit": "<sha>", "written": true }

$ brit baseline read elohim --repo .
{ "pipeline": "elohim", "ref": "refs/notes/rakia/baselines/elohim", "commit": "<sha>" }

$ git show-ref refs/notes/rakia/baselines/elohim
<sha> refs/notes/rakia/baselines/elohim       # stock git can read it
```

`brit baseline migrate <pipeline-baselines.json>` reads a Jenkins migration artifact and writes one ref per pipeline — one-shot cutover tool.

## Schema-IoC Pass Results

### Eliminated escape hatches

`elohim/rakia/rakia-core/src/manifest.rs` had three `serde_json::Value` fields before the IoC pass:

| Field | Before | After |
|---|---|---|
| `BuildManifest.gate` | `serde_json::Value` (with `#[serde(default)]`) | `Option<BuildGate>` (typed via `GateConfig` in generated_types) |
| `BuildManifest.deployment` | `serde_json::Value` | `Option<BuildDeployment>` (typed via `DeploymentConfig`) |
| `BuildStep.executor` | `serde_json::Value` | `BuildExecutor` (required; supports both Jenkins-era `stage`/`function` and rakia-native `kind`-discriminated fields) |

All three are now generated from `elohim/rakia/schemas/v1/build-manifest.schema.json` via `pnpm run rakia:codegen:rs`. Hand-writing forbidden by convention; pre-push hook enforces freshness.

### Output contract locked in

`elohim/rakia/schemas/v1/build-plan.schema.json` defines the `brit plan` output. Two enforcement layers:

1. `tests/build_plan_schema_contract.rs` — direct contract test (Task 11)
2. `tests/fixture_runner.rs` — every fixture's BuildPlan output is also validated against the schema (Task 22)

Drift between Rust struct (generated) and schema would fail both.

### Closed follow-ups from predecessor spec

| Predecessor follow-up | Status | Where |
|---|---|---|
| `unwrap → expect` at constellation.rs:124 | Closed in predecessor sprint | n/a |
| `buildProcess` parsed but unused | **FIXED** in Task 12 | `plan_from_changes` now matches both source and buildProcess globs; fixture 07 + 08 lock the behavior |
| `gate`, `deployment`, `executor` as `serde_json::Value` | **FIXED** via codegen | This sprint |

### Discoveries / surprises during the sprint

- **Manifest authors today encode Jenkins dispatch via `executor: { stage, function }`.** Tasks 3 and 7 had to refine the `executor` schema to permit BOTH legacy Jenkins fields AND rakia-native `kind`-discriminated fields. The schema's `executor.kind` is intentionally extensible (no enum closure) per the Stage 2 trajectory — adding `experienceStory`/`containerImage`/`composition` later won't require schema-version bumps for the discriminator.
- **Task 7 introduced and Task 7-fix corrected a nullable-type regression.** The translator initially didn't handle JSON Schema's `"type": ["string", "null"]` form. Real manifests use `null` for `outputs.verify` (no verify command) and `executor.function` (inline Jenkins stages). The translator was extended to detect array-with-null types and emit `Option<T>` always (regardless of `required`).

### Carry-overs (not done this sprint)

| Item | Why deferred |
|---|---|
| `O(N*A)` traversal in `plan_from_changes` | Performance optimization — works at current scale (8 manifests × ~3 steps each) |
| `GlobSet` precompilation per `QualifiedStep` | Same — defer until profiling shows it matters |
| `AffectedBy::DownstreamNode` declared but not emitted | Needs purpose decision; not blocking describability |
| gix error string-matching for `NotFound` | Needs typed variant upstream in gix |
| Codegen support for `oneOf` discriminated unions | Used flat-with-optionals pattern instead this sprint (works for current schemas) |
| Folding `brit-verify`/`brit-build-ref` into unified `brit` binary | UX polish, not contract work |
| `planFingerprint: BritCid` field on BuildPlan (Stage 2 content-addressing) | Schema designed to accommodate it; not load-bearing until peer dispatch |
| Threshold attestation envelope around baseline writes | Single-node baselines work fine today; threshold required only for multi-builder swarm |
| Migration of executor blocks from `{stage, function}` to `{kind, ...}` | Belongs to rakia-executor sprint when new dispatch dispatching exists |
| Sophia stub manifest replacement | A real `build-manifest.json` in the sophia submodule (cross-repo work) |

## What's Next: Rakia Runnable End-to-End

The next sprint moves from **describable** to **runnable**. Scope sketch:

### `rakia-executor`

New crate at `elohim/rakia/rakia-executor/`. Takes a `BuildPlan` and executes the steps level-by-level (steps within a level run in parallel). Per step:

1. Spawn the `executor.kind`-specific process (shell, pnpm, cargo, rustCargo, noOp; future kinds added by extension)
2. Capture stdout/stderr to per-step log files (content-addressed by step fingerprint?)
3. Determine pass/fail from exit code (and `outputs.verify` if specified)
4. Emit an `ExecutionEvent` per step (start, finish, fail) — schema-defined

### `rakia ci` wrapper

CLI in a new `rakia-cli` crate (rakia workspace). Single command for the whole CI workflow:

1. `brit baseline read <pipeline>` → get baseline ref
2. `brit plan --since <baseline> --pipeline <pipeline>` → compute the plan
3. Hand to `rakia-executor` for execution
4. On all-pass: `brit baseline write <pipeline> <head>` → advance baseline
5. Emit a final `BuildAttestation` (composes the existing brit-epr `Build` attestation primitive)

### Schema-IoC pass for next sprint

Two new schemas to author + generate types from:
- `execution-event.schema.json` — start/finish/fail events with timing, exit codes, log refs
- `build-attestation.schema.json` — final attestation matching the existing brit-epr `Build` attestation primitive
- Possibly `executor-event.schema.json` for executor-internal events

### Acceptance criteria for next sprint

- [ ] `rakia ci --pipeline <name>` runs against a real pipeline (start with one, e.g., elohim-app) and produces an attestation
- [ ] Per-step logs captured and addressable
- [ ] Failure of one step in a level does not halt parallel steps in same level (configurable)
- [ ] Baseline advances only on full success
- [ ] All schemas validate; zero `serde_json::Value` escapes in executor types
- [ ] BuildAttestation composes brit-epr's existing Build attestation type (no new attestation primitive)

## Open Questions Surfaced

These came out of the IoC pass and warrant a brainstorm before next sprint:

### 1. Sophia needs a real build-manifest.json

`elohim:build-angular` declares `depends: ["elohim-sophia:build-sophia-umd"]` but the sophia git submodule has no `build-manifest.json`. Three resolutions:

- **a) Add a manifest to sophia submodule** (cross-repo PR; sophia owns its build).
- **b) Make rakia-core tolerate unresolved external deps** (warn instead of error; treat them as no-op). Less strict but more robust to the multi-repo reality of this monorepo.
- **c) Carry stub manifests in elohim for unresolved cross-repo deps** (the fixture pattern, but in production). Encodes the assumption "we know what sophia builds" inside elohim — fragile.

Without resolution, three CLI commands (`graph show`, `affected`, `plan`) cannot run against the live elohim repo end-to-end. They work on fixtures and any self-consistent sub-tree.

### 2. Should `gate.projects` be discriminated by project type?

Currently a flat map; some projects use `patterns`, others use `required`. A discriminated shape would be more precise but breaks existing manifests. Defer until next sprint exposes whether discriminator buys anything for execution.

### 3. Executor-block migration trigger?

Today's manifests carry Jenkins-era `{ stage, function }`. Rakia-executor sprint will need rakia-native `{ kind, ... }` blocks. Option: leave both supported (legacy fields ignored by rakia-executor, new fields ignored by Jenkins) during transition; cut over manifest-by-manifest.

### 4. How do we handle `manualOnly` pipelines in `rakia ci`?

Skip them? Require explicit `--pipeline` flag? Add an environment-mode flag? Defer to executor-sprint design.

### 5. Stage 2 attestation generation — protocol participation

The spec's "Rakia Trajectory" section sketches how rakia composes brit-epr Build attestations + reach computation for "artifact X verified by N peers." When does the work start? Probably AFTER rakia-executor exists (something to attest about) but BEFORE peer dispatch (where threshold becomes load-bearing). Scoping target for the sprint after rakia-runnable.

## Acceptance Criteria — Sprint Self-Check

Spec acceptance criteria, checked at sprint close:

### CLI Surface
- [x] `brit-cli` crate compiles, single `brit` binary
- [x] All 8 subcommands implemented + JSON output
- [x] `brit graph show --format dot` produces valid Graphviz (rendered to SVG)
- [x] `brit plan` output validates against `build-plan.schema.json`
- [x] `brit affected --since` and `--files` produce results (caveat: live repo blocked by sophia gap)
- [x] `brit baseline write` produces a valid git ref (verified via `git show-ref`)
- [x] Errors handled with clear messages + exit code 1 (or 2 for arg errors)

### Schemas
- [x] `build-manifest.schema.json` exists at `epr:schema:rakia:build-manifest:v1`
- [x] `build-plan.schema.json` exists at `epr:schema:rakia:build-plan:v1`
- [x] All 8 manifests validate against schema
- [x] `gate`, `deployment`, `executor` properly typed (no free-form objects)

### Codegen
- [x] `pnpm run rakia:codegen:rs` regenerates `generated_types.rs`
- [x] `--verify` mode fails on drift
- [x] Zero `serde_json::Value` in `manifest.rs`
- [x] Generated file rustfmts cleanly
- [x] Pre-push hook runs verify on schema/manifest changes

### Fixture Tests
- [x] At least 8 fixtures cover the documented scenarios
- [x] All fixtures pass
- [x] Fixture BuildPlan outputs validate against schema (every fixture, every run)

### Sprint-Result Artifact
- [x] This document exists with all 5 required sections (runnable, IoC results, next, open questions, carry-overs/sprint-self-check)
- [x] Demo transcript reproducible
- [x] Next-sprint scope concrete enough to brainstorm against

## Sprint-Cadence Pattern Validation

The spec elevated schema-as-IoC to a recurring sprint discipline. This sprint validated the pattern:

- Build pragmatically through Phases 1–6 (CLI surface emerged organically based on real schema/codegen learnings)
- Phase 7 closed with the IoC sweep — every `serde_json::Value` accounted for, every CLI output validating
- One regression (Task 7's nullable simplification) was caught by the validation script BEFORE merge — that's the discipline working as intended

For next sprint: same rhythm. Build the executor + `rakia ci` pragmatically; close with `execution-event.schema.json` + `build-attestation.schema.json` IoC pass. The `rakia/schemas/scripts/codegen-rs.mjs` translator will likely need an extension for `oneOf` (BuildAttestation may need it) — that's a known follow-up the next sprint will plan around.

## Sprint Statistics

- **Tasks completed:** 25 / 25
- **Submodule commits:** 16 (rakia) + 7 (brit) = 23
- **Parent commits:** 2 (package.json scripts, .husky/pre-push hook)
- **Schemas authored/refined:** 2 (build-manifest, build-plan)
- **Generated Rust types:** 16 structs + 1 enum (in `generated_types.rs`)
- **Subcommands shipped:** 8 (across 6 command groups)
- **Test count delta (rakia-core):** 5 baseline → 31 after sprint (16 unit + 15 integration)
- **Fixture cases:** 8 (with frozen 9-manifest snapshot)
- **`serde_json::Value` escapes eliminated:** 3 (BuildManifest.gate, BuildManifest.deployment, BuildStep.executor)
- **Lines added (per submodule):** rakia ~2500, brit ~1100, parent ~50 (excluding lockfile churn)
