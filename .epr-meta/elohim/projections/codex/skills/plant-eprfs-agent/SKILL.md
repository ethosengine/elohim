---
name: plant-eprfs-agent
description: Plant ONE runtime-authored subagent in its elohim-native package on the eprfs layer (the package becomes the authoritative source-of-truth for the agent's system prompt AND execution contract — tools/model/color; .claude/.codex become generated, content-addressed, provenance-recorded projections that grow from and trace back to it). The agent member of the plant-eprfs family; mcp-less v1, contract-round-trip-guaranteed lossless.
metadata:
  runtime: codex
  sourceRuntime: elohim-agent
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/plant-eprfs-agent.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/plant-eprfs-agent"
---
# Plant EPRFS Agent

**Plant** a runtime-authored subagent in its elohim-native package on the **eprfs** layer: the package under `.epr-meta/elohim/packages/agents/` becomes the authoritative root. Its `instructions.body` is the source of truth for the system prompt, and its `metadata` (`role`, `modelHints`, `toolRefs`, `mcpServerRefs`, `governance`) is the source of truth for the agent's *execution contract*. `.claude/agents/<id>.md` and `.codex/agents/<id>.md` grow from that root as generated projections that trace back to it — losslessly, content-addressed (eprfs `BlobCid`), with the composing model recorded.

This is the **agent** member of the plant-eprfs family (siblings, grown one at a time: `plant-eprfs-skill`, `plant-eprfs-claude-md`, `plant-eprfs-hook`). It is runtime-agnostic — it plants an agent persona regardless of which runtime authored it. For the shared authoring discipline see the `elohim-package-authoring` skill.

**One capability type, one target per run.** Plant ONE agent, prove it, then the next.

## What planting does

- **Origin preserved.** The package keeps `metadata.sourceRuntime` (`"claude"`/`"codex"`) — where the agent was *born*.
- **Authority rooted in the package.** The package gains `metadata.master: "package"` — editing `.claude`/`.codex` directly is now drift; the root is the package.
- **The execution contract is generated, not passed through.** Unlike a skill (name/description only), an agent's frontmatter carries a *contract*: `model`, `color`, `tools`, and (for some) `mcpServers`. On flip, the projector reconstructs `tools` from `metadata.toolRefs`, `model` from `metadata.modelHints.claudeModel`, and `color` from `metadata.modelHints.claudeColor`. These fields are load-bearing — Claude scopes tool access and model choice from them — so the plant is not lossless unless they round-trip. This skill's floor (below) is that round-trip.
- **Provenance recorded.** `eprfs-agent compose-graph` records the content-addressed edge from each projection back to its native package (`packageCid`), attributed to the model that composed it (`composedBy`).

## The floor this stands on

Two gates, both green before and after:

1. **Source fidelity** (`verifySourceFidelity`) — for *un-flipped* agents, `project(import(source)) === source`, byte-for-byte. Planting one agent must never break this for the others.
2. **Contract round-trip** (the agent analog) — for the *flipped* agent, the generated `.claude`/`.codex` frontmatter's `tools`/`model`/`color` equal the package's `toolRefs`/`modelHints`. Freshness alone (`project(package) === file`) is tautological because the file was regenerated from the package; this contract assertion is what proves the plant lost no capability.

## Only mcp-less agents are plantable in v1

An agent's full `mcpServers` block (`type`, `url`) is not yet reconstructable from package metadata (`mcpServerRefs` holds only the server *names*). If the agent carries a non-empty `mcpServers` block, **STOP** — planting it would drop its MCP wiring. The verify gate refuses to treat an mcp-bearing agent as flippable. Plant an mcp-less agent instead until the family adds structured `metadata.mcpServers` preservation.

## Procedure (one agent `X`)

1. **Confirm mcp-less.** `X`'s `.claude/agents/X.md` has no `mcpServers:` block. Otherwise stop (see above).
2. **Confirm the package is faithful.** `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` is green — `X`'s package round-trips its source losslessly. If `X` has no package yet, `... import` first.
3. **Root authority** in `.epr-meta/elohim/packages/agents/X.json`: keep `metadata.sourceRuntime`; add `metadata.master: "package"`; confirm `metadata.modelHints.claudeModel`/`claudeColor` and `metadata.toolRefs` are populated (they carry the contract now); ensure `metadata.governance = { "eprRef": "epr:elohim-agent/agents/X", "policy": "capability-governance@1", "gates": ["epr-meta-resolver","elohim-agent:packages:verify"], "ledger": ".claude/data/governance-findings.jsonl" }`; record `metadata.composedBy: "<your model id>"`.
4. **Regenerate projections** from the package: `... project --write-fixtures` then `... project --write-runtime`. The projector GENERATES `X`'s `.claude`/`.codex` frontmatter — the nested `metadata` block (`master`/`sourceRuntime`/`governance`) PLUS the reconstructed contract (`tools`/`model`/`color` for claude; `model`/`tools` for codex). Body is the package body.
5. **Verify package-first + contract.** `... verify` is green: `X` takes the package-first path (not re-imported), `project(package) === .claude/.codex` freshness holds, AND the contract round-trip holds (`tools`/`model`/`color` match metadata).
6. **Eyeball the runtime file.** Open `.claude/agents/X.md`: `model:`, `color:`, and the exact `tools:` line must still be present and correct, and `metadata.master: package` must appear. A dropped tool line here is the failure this skill exists to prevent.
7. **Record composition.** `eprfs-agent compose-graph .epr-meta/elohim/packages --projections-root .epr-meta/elohim/projections --composed-by "<your model id>"` — `X`'s node shows `master: "package"`, `packageCid` <-> projection CIDs, and `composedBy`.
8. **Commit path-scoped**, then move to the next agent. Prove one, then the next.

## Per-runtime adapters (the compiler seam)

Projection is compilation through **per-runtime adapter modules** — a `claude` adapter and a `codex` adapter, each owning that runtime's frontmatter dialect. The native package holds the runtime-agnostic contract (`toolRefs`, `modelHints`); each adapter localizes it — claude carries `color`, codex re-roots `sourcePath` to the package. **The identity (name/id) never forks.** This is the seam by which Claude and Codex can author each other's agents: one native persona, two well-formed projections, each attributed and traceable back.

## Invariants

- **The contract must round-trip.** `tools`/`model`/`color` in the generated `.claude` equal `metadata.toolRefs`/`modelHints`; `model`/`tools` in the generated `.codex` likewise. A flip that drops any of these is a capability regression, not a cosmetic diff.
- **CID is single-sourced in eprfs** (`eprfs-core::BlobCid::compute`). Never recompute a CID in JS — call `eprfs-agent`.
- **Never break fidelity for un-planted agents:** only a `master: package` agent gets generated frontmatter; a still-source agent keeps byte-identical `frontmatterRaw` passthrough.
- **Identity never forks per runtime.** One persona, one root, many projections.
- **mcp-bearing agents wait** for structured `mcpServers` preservation before they can be planted.
