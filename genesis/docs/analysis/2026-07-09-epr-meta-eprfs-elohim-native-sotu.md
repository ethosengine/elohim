---
title: "EPR Meta / EPRFS / Elohim-Native Capability SOTU"
id: epr-meta-eprfs-elohim-native-sotu-2026-07-09
kind: analysis
status: draft
written: 2026-07-09
author: codex
cites:
  - epr-meta-compose-gate | `.epr-meta` | sha256:6052ce071bfec509 | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - epr-meta-policy-registry-measure | `.epr-meta` Policy Registry + Measure Tier | sha256:474eee1686e3123b | path: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
  - ci-detection-convergence-epr-meta-fold | CI Change-Detection Convergence + `.ci-ignore`→`.epr-meta` Fold (P6) | sha256:c2c141e379cb5672 | path: genesis/docs/superpowers/specs/2026-06-25-ci-detection-convergence-epr-meta-fold-design.md
---

# EPR Meta / EPRFS / Elohim-Native Capability SOTU

## Verdict

This repo is ready to start dogfooding elohim-native skill and agent authoring as a local,
package-first workflow. The canonical local package home is now
`.epr-meta/elohim/packages`, projection fixtures live under
`.epr-meta/elohim/projections`, and Claude/Codex files are treated as runtime projections.

It is not yet an eprfs-native package graph. The eprfs layer can resolve
`.epr-meta/manifest.md` governance and can materialize generic projection manifests, but there
is not yet an adapter that turns elohim package JSON into an eprfs `ProjectionManifest`, stores
those packages as EPR-backed content, or exposes graph queries across arbitrary repositories and
submodules.

## Readiness Snapshot

| Surface | Status | Practical read |
|---|---|---|
| Root `.epr-meta` directory form | Ready for this repo | `.epr-meta/manifest.md` is the repo governance manifest. Legacy `.epr-meta` files are still readable during migration. |
| Python `.epr-meta` resolver | Ready, but parity-sensitive | The live hook-side resolver understands `.epr-meta/manifest.md` before legacy files and keeps root-first / nearest-wins cascade behavior. |
| Rust `eprfs-meta` resolver | Ready as a resolver | Rust resolution now follows the same directory-form lookup, but it is still resolver-only, not the live authoring gate. |
| Elohim package authoring | Ready to dogfood locally | Skills and agents validate from `.epr-meta/elohim/packages`, and package projection checks detect stale Claude/Codex fixtures. |
| Bulk Claude import | Ready with review | Existing `.claude/skills/*/SKILL.md` and `.claude/agents/*.md` can be imported into native package JSON. Generated native runtime projections are skipped to avoid re-import loops. |
| Runtime projection writes | Mostly ready | Claude and Codex projections are generated. In this managed workspace, `.codex` writes required escalation because `.codex` is read-only in the sandbox profile. |
| EPRFS package materialization | Not ready | eprfs has the generic projection contract, but not the package-domain adapter or storage-backed graph for these capability packages. |
| Submodule / arbitrary filesystem reuse | Pattern-ready, not productized | The layout is simple enough to reuse, but the CLI still assumes this monorepo root and needs configurable roots plus resolver parity gates. |

## Evidence Map

| Path | Status | What it proves |
|---|---|---|
| `.epr-meta/manifest.md` | Active source | Directory-form root governance manifest for this repository. |
| `.epr-meta/elohim/packages/skills/*.json` | Active source | Native elohim skill package fixtures. These are the local package source of truth for skills. |
| `.epr-meta/elohim/packages/agents/*.json` | Active source | Native elohim agent package fixtures. These are the local package source of truth for agents. |
| `.epr-meta/elohim/projections/claude/**` | Generated fixture | Expected Claude projection output from package JSON. |
| `.epr-meta/elohim/projections/codex/**` | Generated fixture | Expected Codex projection output from package JSON. |
| `elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs` | Active tooling | Implements `init`, `import`, `project`, and `verify`; imports Claude assets, validates package JSON, writes projection fixtures, and checks runtime projection freshness when present. |
| `elohim/sdk/domains/elohim-agent/schemas/skill-package.schema.json` | Active contract | JSON schema for native skill packages. |
| `elohim/sdk/domains/elohim-agent/schemas/agent-package.schema.json` | Active contract | JSON schema for native agent packages. |
| `elohim/sdk/domains/elohim-agent/CLAUDE.md` | Active domain guidance | Declares package-first local V1 behavior and explicitly defers eprfs materialization. |
| `.claude/scripts/_lib/epr_meta.py` | Active Python resolver | Live hook-side `.epr-meta` cascade, validation, policy expansion, and rule evaluation. |
| `.claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` | Active test | Covers legacy `.epr-meta` files, directory-form `.epr-meta/manifest.md`, and cascade behavior. |
| `elohim/eprfs/eprfs-meta/src/lib.rs` | Active Rust resolver | Rust `.epr-meta` authored-source adapter for eprfs, now resolving directory-form manifests. |
| `elohim/eprfs/eprfs-core/src/projection.rs` | Active model | Defines the domain-neutral `ProjectionManifest`, `ProjectionEntry`, source identity, and validation invariants. |
| `elohim/eprfs/eprfs-local/src/lib.rs` | Active materializer | Materializes generic eprfs projection manifests into ordinary filesystem trees. |
| `elohim/eprfs/README.md` | Active architecture note | States the split: eprfs owns filesystem projection semantics; domains own interpretation. |
| `.claude/scripts/ci-ignore-projector.py` | Active projection tooling | Keeps `.ci-ignore` as a projection from `.epr-meta` governance metadata. |
| `.husky/pre-push.bash` | Active local gate | Includes freshness checks around generated governance/projection artifacts. |
| `package.json` | Active command surface | Exposes package workflow commands such as `elohim-agent:packages:test`, `import`, `project`, and `runtime`. |
| `genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md` | Open concern | Tracks the risk that the Python live enforcer and Rust eprfs resolver can drift semantically. |

## What Is Working

The `.epr-meta` migration has a usable compatibility shape. New directory-form root
governance is read from `.epr-meta/manifest.md`, while legacy directory-local `.epr-meta`
files remain valid. The important behavior is preserved: cascade collection stays
root-first and nearest definitions win by rule or validator identity.

The elohim-agent package workflow is now useful for real authoring. The package CLI can
initialize the layout, import legacy Claude skills and agents, project package-derived Claude
and Codex fixtures, and verify freshness. This supports immediate dogfooding in this repo:
new canonical capability work can start in `.epr-meta/elohim/packages`, with runtime files
generated from packages.

The bulk migration path is also real. Claude skill directories and Claude agent markdown files
can be transformed into `SkillPackage` and `AgentPackage` JSON. The import path intentionally
skips generated native runtime skills marked with `metadata.sourceRuntime: elohim-agent`, which
prevents the native projections from being mistaken for legacy Claude sources on the next import.

## Drift And Migration Hazards

The biggest hazard is dual meaning. Python `_lib/epr_meta.py` is the live enforcement path,
while Rust `eprfs-meta` is the emerging native resolver. Both now understand the directory-form
manifest, but they are still independent parsers with different responsibilities. Until a shared
fixture/parity suite exists, grammar drift can make the hook layer and eprfs layer disagree about
what the same manifest means.

Runtime projection drift is now detectable, but it remains possible. Hand edits to `.claude/*`
or `.codex/*` are edits to projection surfaces, not canonical package edits. The package verifier
will report stale projections, but the team still needs to treat those failures as source-of-truth
violations rather than normal merge noise.

The eprfs boundary is easy to overstate. The current package tree is stored in the filesystem under
`.epr-meta/elohim/packages`; it is not yet content-addressed through eprfs, not materialized from
EPR storage, and not synchronized as a graph. eprfs can host this future shape, but the adapter is
not present in this slice.

The package CLI is still monorepo-shaped. It computes `REPO_ROOT` from the elohim-agent domain
script location and assumes `.claude`, `.codex`, and `.epr-meta/elohim` in this repository. That is
fine for this workspace dogfood. It is not yet the right interface for arbitrary submodules,
mounted filesystems, or another repo that wants the same graph pattern.

The managed sandbox exposed one operational hazard: `.codex` may be read-only to normal tool
execution even when it is part of the desired runtime projection surface. Verification can still
read it, but writing runtime Codex projections may require escalation or a host profile that marks
`.codex` writable.

## Bulk Migration Readiness

Ready now:

- Import existing Claude Code skills and agents into `.epr-meta/elohim/packages`.
- Author new package-native skills in `.epr-meta/elohim/packages/skills`.
- Generate Claude and Codex projection fixtures from packages.
- Generate runtime `.claude` and `.codex` projections when the filesystem profile permits writes.
- Verify package schemas and projection drift with `pnpm run elohim-agent:packages:test`.

Not ready yet:

- Treat package JSON as EPR-backed graph atoms.
- Materialize the package tree through eprfs.
- Query capability relationships through eprfs projection awareness.
- Use the package CLI unchanged in any arbitrary submodule or filesystem root.
- Retire legacy Claude sources without a projection policy for what remains editable by humans.

## Submodule And Filesystem Pattern Readiness

The portable pattern is sound:

```text
.epr-meta/manifest.md
.epr-meta/elohim/packages/skills/*.json
.epr-meta/elohim/packages/agents/*.json
.epr-meta/elohim/projections/{claude,codex}/...
```

For a submodule, the minimal viable pattern is to place a local `.epr-meta/manifest.md` at the
submodule root, keep packages under `.epr-meta/elohim/packages`, and run package projection checks
with roots pointed at that submodule. The missing part is root configurability: the package CLI
should accept `--repo-root`, `--package-root`, `--projection-root`, `--claude-root`, and
`--codex-root` or equivalent environment variables.

For a general filesystem graph, the eprfs layer needs a package-domain adapter. That adapter should
map native package JSON and generated projection artifacts into `ProjectionManifest` entries with
domain-neutral `ProjectionSource` identities, then let `eprfs-local` materialize the selected view.
Without that adapter, the package layout is portable by convention, not by eprfs graph semantics.

## Closeout Needed

> **Partly delivered 2026-07-10** by the `epr-meta-native-capability-dogfood-and-graph` sprint
> (`genesis/docs/superpowers/specs/2026-07-10-epr-meta-native-capability-dogfood-and-graph-design.md`):
> **#1** (Python↔Rust parity fixtures), **#3** (package `verify` wired into the pre-push gate), and
> **#5** (the eprfs package→`ProjectionManifest` adapter, with a byte-identical round-trip) are DONE.
> The governance layer also gained an `EprRef` backref + lodging ledger + a standing round-trip
> fidelity gate. Remaining: #2 (CLI root-configurability), #4 (human-edit policy / package-master
> flip — deferred by design), #6/#7 (doc audit + submodule fixture).

1. Add a Python/Rust `.epr-meta` parity fixture suite, using representative manifests for legacy
   files, directory-form manifests, policy bindings, measure rules, malformed manifests, and cascade
   conflict resolution.
2. Make `package-projections.mjs` root-configurable so the same package workflow can run in a
   submodule, fixture repository, or mounted eprfs tree.
3. Wire package projection verification into the normal local gate so stale runtime projections are
   treated the same way as other generated-artifact drift.
4. Decide the human edit policy for legacy Claude/Codex runtime files after bulk import. The clean
   default is: package JSON is canonical; runtime edits are projection drift.
5. Define the eprfs package adapter: package JSON to `ProjectionManifest`, package identity to
   `ProjectionSource`, and package/projection status to `ProjectionAwareness`.
6. Audit docs and comments that still describe `.epr-meta` only as a file. Directory-form
   `.epr-meta/manifest.md` should be the preferred root-governance example.
7. Add a reusable submodule fixture that proves the layout outside the main monorepo root.

## Verification Run On This Slice

- `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify`
- `pnpm run elohim-agent:packages:test`
- `pnpm run elohim-agent:test`
- `python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py`
- `python3 .claude/scripts/_lib/__tests__/ci_trigger_test.py`
- `cargo fmt --check -p eprfs-meta`
- `CARGO_TARGET_DIR=/tmp/eprfs-meta-target RUSTFLAGS="" cargo test -p eprfs-meta`
- `git diff --check`

All of the above passed during this slice. The pnpm commands and runtime Codex projection writes needed
host permissions outside the default sandbox because of cache and `.codex` filesystem restrictions.
