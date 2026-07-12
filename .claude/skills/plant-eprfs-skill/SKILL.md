---
name: plant-eprfs-skill
description: Plant ONE runtime-authored skill in its elohim-native package on the eprfs layer (the package becomes the authoritative source-of-truth; .claude/.codex become generated, content-addressed, provenance-recorded projections that grow from and trace back to it). The skill member of the plant-eprfs family; fidelity-gate-guaranteed lossless.
metadata:
  sourceRuntime: elohim-agent
  master: package
  governance: "epr:elohim-agent/skills/plant-eprfs-skill"
---
# Plant EPRFS Skill

**Plant** a runtime-authored skill in its elohim-native package on the **eprfs** layer: the package under `.epr-meta/elohim/packages/skills/` becomes the authoritative root. Its `instructions.body` is the source of truth; `.claude`/`.codex` grow from that root as generated projections that trace back to it — losslessly, content-addressed (eprfs `BlobCid`), with the composing model recorded.

This is the **skill** member of the plant-eprfs family (siblings, grown one at a time: `plant-eprfs-agent`, `plant-eprfs-claude-md`, `plant-eprfs-hook`). It is runtime-agnostic — it plants a skill regardless of which runtime authored it. For the shared authoring discipline see the `elohim-package-authoring` skill.

**One capability type, one target per run.** Plant ONE skill, prove it, then the next.

## What planting does

- **Origin preserved.** The package keeps `metadata.sourceRuntime` (`"claude"`/`"codex"`) — it records where the skill was *born*.
- **Authority rooted in the package.** The package gains `metadata.master: "package"` — editing `.claude`/`.codex` directly is now drift; the root is the package.
- **Provenance recorded.** `eprfs-agent compose-graph` records the content-addressed edge from each projection back to its native package (`packageCid`), attributed to the model that composed it (`composedBy`). Every projection traces to root.

## The floor this stands on

The standing fidelity gate (`elohim-agent:packages:verify` -> `verifySourceFidelity`) proves `project(import(source)) === source` byte-for-byte. Planting therefore loses nothing: the package already reproduces the source exactly, so making it authoritative changes *authority*, not *content*.

## Procedure (one skill `X`)

1. **Confirm the package is faithful.** `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` is green — `X`'s package round-trips its source losslessly. If `X` has no package yet, `... import` first.
2. **Root authority** in `.epr-meta/elohim/packages/skills/X.json`: keep `metadata.sourceRuntime`; add `metadata.master: "package"`; ensure `metadata.governance = { "eprRef": "epr:elohim-agent/skills/X", "policy": "capability-governance@1", "gates": ["epr-meta-resolver","elohim-agent:packages:verify"], "ledger": ".claude/data/governance-findings.jsonl" }` (hand-set); record `metadata.composedBy: "<your model id>"`.
3. **Regenerate projections** from the package: `... project --write-fixtures` then `... project --write-runtime`. The projector GENERATES `X`'s `.claude`/`.codex` frontmatter (carrying `master: package` + `sourceRuntime` + the governance eprRef) instead of passing the old human frontmatter through. Body is the package body.
4. **Verify package-first.** `... verify` is green: `X` takes the package-first path (not re-imported), and `project(package) === .claude/.codex` freshness holds.
5. **Record composition.** `eprfs-agent compose-graph .epr-meta/elohim/packages --projections-root .epr-meta/elohim/projections --composed-by "<your model id>"` — `X`'s node shows `master: "package"`, `packageCid` <-> projection CIDs, and `composedBy`.
6. **Commit path-scoped**, then move to the next skill. Prove one, then the next.

## Per-runtime adapters (the compiler seam)

Projection is compilation through **per-runtime adapter modules** — a `claude` adapter and a `codex` adapter, each owning that runtime's template to/from (read in on import, write out on project) and its localization (framing/examples tailored to the runtime). The native package holds the runtime-agnostic intent; each adapter localizes it — **the identity (name/id) never forks**. This is the seam by which **Claude and Codex can author each other's skills**: one native root, two well-formed projections, each attributed and traceable back.

## Invariants

- **CID is single-sourced in eprfs** (`eprfs-core::BlobCid::compute`). Never recompute a CID in JS — call `eprfs-agent`.
- **Never break fidelity for un-planted skills:** only a `master: package` skill gets generated `.claude` frontmatter; a still-source skill keeps byte-identical passthrough.
- **Identity never forks per runtime.** One capability, one root, many projections.
