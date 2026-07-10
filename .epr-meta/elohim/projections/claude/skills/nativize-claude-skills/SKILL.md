---
name: nativize-claude-skills
description: Flip Claude-authored skills to elohim-native package-master (package becomes source-of-truth; .claude/.codex become generated, content-addressed, provenance-recorded projections) — one skill or in batch, fidelity-gate-guaranteed lossless.
metadata:
  author: elohim-protocol
  version: 1.0.0
  sourceRuntime: elohim-agent
  packageKind: SkillPackage
---
# Nativize Claude Skills

Flip a Claude-authored skill (or all of them) to **package-master**: the elohim-native package under `.epr-meta/elohim/packages/` becomes the source of truth, and `.claude`/`.codex` become generated projections — losslessly, with content-addressed provenance, and with the composing model recorded.

Use this when converting existing Claude skills/subagents to elohim-native package-first authoring, one skill or in batch.

## What "flip" means

- **Origin preserved.** The package keeps `metadata.sourceRuntime: "claude"` — it was *born from Claude*.
- **Authority flipped.** The package gains `metadata.master: "package"` — the package JSON `instructions.body` is now the source; editing `.claude`/`.codex` directly is drift.
- **Provenance recorded.** `eprfs-agent compose-graph` records the content-addressed edge from each projection back to its native package (`packageCid`), attributed to the model that composed it (`composedBy`). This is how every skill traces to root.

## The floor this stands on

The standing fidelity gate (`elohim-agent:packages:verify` -> `verifySourceFidelity`) proves `project(import(source)) === source` byte-for-byte. That is the guarantee that flipping loses nothing: the package already reproduces the Claude source exactly, so making the package authoritative changes *authority*, not *content*.

## Per-skill procedure

For a Claude-sourced skill `X`:

1. **Confirm the package is faithful.** `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` is green — `X`'s package already round-trips its `.claude` source losslessly. If `X` has no package yet, run `... import` first.
2. **Flip authority** in `.epr-meta/elohim/packages/skills/X.json`:
   - keep `metadata.sourceRuntime: "claude"`;
   - add `metadata.master: "package"`;
   - ensure a governance block `metadata.governance = { "eprRef": "epr:elohim-agent/skills/X", "policy": "capability-governance@1", "gates": ["epr-meta-resolver","elohim-agent:packages:verify"], "ledger": ".claude/data/governance-findings.jsonl" }` — hand-set (nothing generates it for a package-master skill);
   - record `metadata.composedBy: "<your model id>"` — note the model that did the flip.
3. **Regenerate the projections** from the package: `... project --write-fixtures` then `... project --write-runtime`. The projector now GENERATES `X`'s `.claude`/`.codex` frontmatter (carrying `master: package` + `sourceRuntime: claude` + the governance eprRef) instead of passing the old human frontmatter through. The body is the package body.
4. **Verify package-first.** `... verify` is green: `X` takes the package-first path (not re-imported), and `project(package) === .claude/.codex` freshness holds.
5. **Record composition.** `eprfs-agent compose-graph .epr-meta/elohim/packages --projections-root .epr-meta/elohim/projections --composed-by "<your model id>"` — `X`'s node now shows `master: "package"`, `packageCid` <-> projection CIDs, and `composedBy`.

## Batch mode

Apply the per-skill procedure to every `.claude/skills/*/SKILL.md` still marked `sourceRuntime: claude`, ONE skill at a time, verifying after each (fidelity gate + freshness), committing path-scoped. A skill already native (`sourceRuntime: elohim-agent`) needs no flip.

## Per-runtime interpretation (the compiler seam)

Projection is compilation: `projectClaude`/`projectCodex` are per-runtime *backends*. The native package holds the runtime-agnostic intent; a backend may compose it differently for Claude vs Codex when a skill declares tailoring. Default is identity (same body, per-runtime frontmatter). This is the seam by which **Claude and Codex can author each other's skills** — one native intent, two well-formed projections, each attributed and traceable to root.

## Invariants

- **CID is single-sourced in eprfs** (`eprfs-core::BlobCid::compute`, `CIDv1(dag-cbor, sha2-256)`). Never recompute a CID in JS — call `eprfs-agent`.
- **Never break fidelity for un-flipped skills:** only a `master: package` skill gets generated `.claude` frontmatter; a still-Claude skill keeps byte-identical passthrough.
- **Everything traces to root:** every projection's provenance is its `packageCid`; the composition graph is the static record of how — and by whom — each skill was composed.
