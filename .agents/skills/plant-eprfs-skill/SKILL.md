---
name: plant-eprfs-skill
description: Plant ONE runtime-authored skill in its Elohim-native package on the eprfs layer so its SKILL.md and every relative asset become authoritative, content-addressed inputs for Claude, Codex, and Antigravity projections. Use when planting or replanting a skill, adding packaged references/scripts/resources, or proving projection fidelity across runtimes.
metadata:
  runtime: antigravity
  sourceRuntime: elohim-agent
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/plant-eprfs-skill.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/plant-eprfs-skill"
---
# Plant EPRFS Skill

**Plant** a runtime-authored skill in its elohim-native package on the **eprfs** layer: the package under `.epr-meta/elohim/packages/skills/` becomes the authoritative root. Its `instructions.body` and packaged `assets` are the source of truth; `.claude`/`.codex`/`.agents` grow from that root as generated projections that trace back to it — losslessly, content-addressed (eprfs `BlobCid`), with the composing model recorded.

This is the **skill** member of the plant-eprfs family (siblings, grown one at a time: `plant-eprfs-agent`, `plant-eprfs-claude-md`, `plant-eprfs-hook`). It is runtime-agnostic — it plants a skill regardless of which runtime authored it. For the shared authoring discipline see the `elohim-package-authoring` skill.

**One capability type, one target per run.** Plant ONE skill, prove it, then the next.

## What planting does

- **Origin preserved.** The package keeps `metadata.sourceRuntime` (`"claude"`/`"codex"`/`"antigravity"`) — it records where the skill was *born*.
- **Authority rooted in the package.** The package gains `metadata.master: "package"` — editing `.claude`/`.codex`/`.agents` directly is now drift; the root is the package.
- **Assets rooted with the entrypoint.** Every relative `references/`, `scripts/`, `resources/`, or other skill file is encoded in the package `assets` collection; `metadata.assetRefs` is the exact ordered path list. A missing or undeclared extra asset is a failed plant, even when `SKILL.md` itself is fresh; generated targets prune assets removed from the package.
- **Provenance recorded.** `eprfs-agent compose-graph` records a content-addressed `Projection` edge whose source is the package CID and whose derived value is each entrypoint or asset CID, attributed to the model that composed it (`composedBy`). Every projection traces to root.

## The floor this stands on

The standing fidelity gate (`elohim-agent:packages:verify` -> `verifySourceFidelity`) proves `project(import(source)) === source` byte-for-byte. Planting therefore loses nothing: the package already reproduces the source exactly, so making it authoritative changes *authority*, not *content*.

## Procedure (one skill `X`)

1. **Confirm the package is faithful.** `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` is green — `X`'s package round-trips its source losslessly. If `X` has no package yet, `... import` first.
2. **Root authority and assets** in `.epr-meta/elohim/packages/skills/X.json`: keep `metadata.sourceRuntime`; capture every non-`SKILL.md` file beneath the skill root as `{path, contentBase64}` in `assets`; set `metadata.assetRefs` to those paths in the same order; add `metadata.master: "package"`; ensure `metadata.governance = { "eprRef": "epr:elohim-agent/skills/X", "policy": "capability-governance@1", "gates": ["epr-meta-resolver","elohim-agent:packages:verify"], "ledger": ".claude/data/governance-findings.jsonl" }` (hand-set); record `metadata.composedBy: "<your model id>"`.
3. **Plant atomically** with the replant wrapper: `node elohim/sdk/domains/elohim-agent/scripts/replant.mjs --compose X`. It runs the whole project → verify → compose sequence and is the supported way to plant: it snapshots every file the plant could touch, writes fixtures then runtime scoped `--only X` (never a bare tree-wide `--write-runtime`, which rewrites every other package's runtime file — including un-planted Claude-authored and ambiently-modified ones), re-runs `verify`, and restores the snapshot byte-exactly if the plant introduced a failure that was NOT in the pre-plant baseline. So a red plant leaves no half-planted tree, and a tree already red for unrelated reasons still lets a clean plant land. `--dry-run X` reports the file list and restores everything; several names plant one at a time and stop at the first failure (`--keep-going` continues), so a partial run leaves N good plants and zero half-plants. `--compose` records provenance only after every plant holds, and warns-and-continues when the `eprfs-agent` binary is absent rather than failing a plant that already verified green.
4. **Confirm what landed.** The projector GENERATED `X`'s `.claude`/`.codex`/`.agents` frontmatter (carrying `master: package` + `sourceRuntime` + the governance eprRef) instead of passing the old human frontmatter through; body and relative assets come from the package. Green means `X` takes the package-first path (not re-imported), every runtime `SKILL.md` and packaged asset is byte-fresh, and the compose graph for `X` shows a package node with `master: "package"`, package-to-entrypoint/asset `Projection` edges, and `composedBy` (attribution comes from the `metadata.composedBy` set in step 2 — `replant.mjs` passes no `--composed-by`). Underlying mechanism it runs for you: `... project --write-fixtures --only X`, `... project --write-runtime --only X`, `... verify`, `eprfs-agent compose-graph .epr-meta/elohim/packages --projections-root .epr-meta/elohim/projections`.
5. **Commit path-scoped**, then move to the next skill. Prove one, then the next.

## Per-runtime adapters (the compiler seam)

Projection is compilation through **runtime-aware lowering**. Claude import keeps the source-fidelity path; package-master projection generates Claude frontmatter, while Codex and Antigravity share the Markdown body and receive runtime-localized frontmatter and paths. The native package holds the runtime-agnostic intent; each supported lowering localizes only its runtime contract — **the identity (name/id) never forks**. This is the seam by which **Claude, Codex, and Antigravity can author each other's skills**: one native root, three well-formed projections, each attributed and traceable back.

## Invariants

- **CID is single-sourced in eprfs** (`eprfs-core::BlobCid::compute`). Never recompute a CID in JS — call `eprfs-agent`.
- **Never break fidelity for un-planted skills:** only a `master: package` skill gets generated `.claude` frontmatter; a still-source skill keeps byte-identical passthrough.
- **Asset paths never fork per runtime.** An asset keeps the same path relative to `SKILL.md` everywhere, is verified byte-for-byte as part of an exact asset set, and receives its own package→projection derivation edge.
- **Do not claim an undocumented ABI.** Skills are supported in Antigravity because `.agents/skills` is documented. Its project subagent, command, hook, and MCP adapters must be proven separately.
- **Identity never forks per runtime.** One capability, one root, many projections.
