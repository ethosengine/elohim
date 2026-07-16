---
name: app-port
description: Use when bringing an external app or prior-art onto the Elohim Protocol — "how would Hylo / this repo work native here", "port my app to elohim", "decompose this product onto the protocol", onboarding an external developer's codebase or product vision. For an INTERNAL feature exploration (not external prior-art), use atlas-grounding instead.
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/app-port"
---

# App Port

## Overview

Decompose an external app (a repo or a product vision) into an Elohim **re-composition plan**: which domains become app-manifests, which entities map to **existing** EPR / REA / Commitment primitives (**reuse, don't fork**), which features land on which seams + participation tracks + device rungs. **Analyst by default** — it produces the plan a human/agent then builds; a future `--scaffold` mode (v2) emits starting manifests, so author the plan structure to be scaffold-ready.

## When to Use

- "How would [external app] work native on Elohim?" / "port my app" / "decompose this repo onto the protocol."
- Onboarding an external developer's prior art.

Not for: an internal feature exploration (use `atlas-grounding`); designing a single new entity (use `p2p-design-gate` directly).

> **app-port runs its delegated skills as INLINE techniques, not separate output-producing passes.** `concept-mapping` and `p2p-design-gate` fill *columns of app-port's entity table*; do NOT emit `concept-mapping`'s per-concept write-up or the gate's full per-entity Output Format for each row. The compact table IS the output. (Run a delegated skill standalone only for its own primary case — a single "what's the equivalent of X?" question, or designing one genuinely novel entity.)

1. **Understand the prior art** — its domains, entities, key features, and flows (read the repo / the vision).
2. **Ground (default)** via `atlas-grounding` — fan out over **(a)** the prior art and **(b)** the seams it touches, so REUSE claims rest on current reality. *Analyst-from-atlas-only is allowed for a quick pass* — then mark every REUSE claim "unverified; confirm via grounding."
3. **Translate, inline** — using `concept-mapping`'s atlas-routing technique: **split each feature's compound concepts first**, then route each sub-concern to its seam + primitive. This fills the entity table's *primitive* + *seam* columns — not a separate pass.
4. **Classify every entity, inline** — using `p2p-design-gate`'s classification logic: notarized (A) / derived (A2) / agent-scoped (B/B2) / operational (C) + content-address. Fill the entity table's *class* + *address* columns. **REUSE existing primitives** — most entities are `Content` / `EconomicEvent` / `Mishpat::Commitment` / `Human` / `Attestation`; a net-new DNA entry type is the near-forbidden last resort (default to links/A2; check DNA headroom). *(Note: a Category-A notarized entity is addressed by its DHT entry/action hash — see `project_mishpat_commitment_cid_is_entry_hash`; the gate's three address Options apply to the content being notarized.)*
5. **Compose the plan** — domains → `domains/<app>/manifest.json`; entities → primitives tagged **REUSE / NET-NEW**; features → seams (§3) + tracks (T1–T4); audiences → device rungs (§2). Note how the app's core flows compose the five-verb grammar (`authorAtom · commit · runGovernor · rollupCoverage · bindCapability`) — the *destination* framing (atlas §6), not a shipped API.
6. **Output the re-composition plan** (Markdown, analyst). Flag **REUSE-vs-NET-NEW** prominently — the anti-fork discipline and the plan's most load-bearing column.

## Output Shape

1. **What this app IS, re-substrated** — one paragraph (agency-preserving, REA-economic, governance-bound by construction), naming how its core flows compose the five-verb grammar.
2. **Domains → app-manifest(s).**
3. **Entity table:** prior-art entity → elohim primitive → class (A/A2/B/B2/C) → REUSE / NET-NEW → content-address. *(primitive + seam from `concept-mapping` inline; class + address from `p2p-design-gate` inline — one compact row each, never a verbose per-entity block.)*
4. **Features → seams + tracks.**
5. **Device rungs** (which audiences run on which rung).
6. **Net-new work-list** — the genuinely-new pieces (should be small), tagged for follow-up. If this list is long, you're forking too much — re-run step 4.

## Composition

Composes `atlas-grounding` (the grounding engine, over prior-art + our seams) + `concept-mapping` (analog + placement per concept) + `p2p-design-gate` (entity classification). The **external** twin of `atlas-grounding`'s internal use. Reads the **durable** atlas (structure + placements); `atlas-grounding` pulls current build-state from the dated assessments.

## Common Mistakes

- **Running a delegated skill as a separate verbose pass** — `concept-mapping` and `p2p-design-gate` are inline column-fillers here; emitting a standalone concept-mapping write-up or a full gate Output-Format block per entity is the wrong shape. One compact entity-table row each.
- **Forking instead of reusing** — the cardinal error. Most entities are already-existing primitives; defaulting to new DNA entry types wastes scarce headroom and fragments the model. `p2p-design-gate` gates this; a long net-new list is the smell.
- **Designing HTTP routes first** (REST-shaped) — start from DHT entry classification (the `p2p-design-gate` order: zome → projection → route last).
- **Skipping the grounding step** — decomposing from assumptions about the prior art instead of reading it (and grounding our seams' reality).
- **Treating a feature as atomic** — split compound features into sub-concerns first (one feature often spans encryption + durability + scheduling + UI).
- **Scaffolding prematurely** — v1 is analyst (a plan a human reviews); generation bakes decisions a human should make.
