---
id: "backlog-pillar-island-recompose-recipe"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Per-pillar island recompose — the proven recipe + firing order (lamad was the rep)"
slug: "pillar-island-recompose-recipe"
written: "2026-06-11"
author: "distilled from the lamad island recompose (proving rep) + the avodah ceremony pilot"
status: "refined"
priority: "high"
tags: [island-recompose, authorship-discipline, per-locus, subject-routing, recipe]
cites:
  - genesis/data/timeline/backlog/subject-routing-locus-census.md
  - genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md
  - genesis/data/timeline/chronicle/2026-06-11-substrate-currency-avodah-locus-pilot.md
shift_objective: |
  Run the <pillar> island recompose per the recipe in this entry: Phase 0 ground-truth (inventory,
  inbound refs, liveness, duplication, prior-art), Phase 1 authorship fan-out (staged drafts, one agent
  per disposition group, verification rules baked in), Phase 2 serial placement (spot-verify → place →
  derived_from for retiring sources → seal → gospel one-liners), Phase 3 operator gate (disposition map
  → approve → git rm → live-ref sweep → registry updates), Phase 4 close (verify, commit, distill deltas
  back into this recipe).
---

# Per-pillar island recompose — the recipe

Proven end-to-end on `app/lamad/docs/` (12 files / ~6,100 lines → 1 architecture seed + 2 history
records + 6 backlog entries + 1 relocated vision doc + 1 gospel one-liner; island retired to git).
Authorship discipline rules come from the avodah ceremony pilot (see its chronicle).

## Phase 0 — ground truth (inline, deterministic, ~10 min)

1. **Inventory** the island; **inbound-ref sweep** (`grep -rn '<island-path>/'` repo-wide) — know the
   blast radius before anything moves.
2. **Liveness-check** the island's central design claims against code/manifest (is the designed system
   live? → CANONIZE; superseded? → harvest+retire).
3. **Duplication check** for relocate candidates; **prior-art** via `spec-coherence-index.py --query`
   (compose with existing seeds, never fork).

## Phase 1 — authorship fan-out (staged drafts, one agent per disposition group)

Dispatch one agent per group: canonical-design / plan-decompose / backlog-items / superseded-spec-harvest /
handoff-harvest / relocate / readme-reconcile. Bake in the RULES verbatim:
substrate citation for every claim · NO invented slugs or ids · `cites:` as plain paths (tooling seals) ·
uncertainty = explicit "OPEN QUESTION:" · **drafts to a staging dir, return SHORT summaries** (full bodies
in returns blow context and weaken the gate). Backlog drafts follow `timeline/CONVENTIONS.md` frontmatter;
history drafts are museum-shaped (`tier: history`); the canonical seed separates AS-IMPLEMENTED (cited
per-mechanic) from §Vision-remainder (gap ledger → backlog companion).

## Phase 2 — serial placement (operator-side; cite tooling is shared state)

1. **Spot-verify** the seed's load-bearing claims at source before placing (the avodah lesson: the only
   fictions are confident uncited assertions).
2. Place: seed → `architecture/` · history → `history/2026-MM-DD-<slug>.md` · backlog → bare-slug
   filenames · relocations per target reasoning.
3. **Provenance of retiring sources goes in `derived_from:` — never `cites:`** (a cite to a deleted file
   goes DEAD; derived_from is the lineage breadcrumb, audit-exempt).
4. **Strip staged-draft frontmatter completely when wrapping into entity bodies** (`tail -n +2` only
   removes one line — flatten the whole block to a provenance sentence; the lamad rep shipped orphan
   metadata mid-body and had to fix it).
5. Seal doc-roots (`cite-gen --seal`, serial); verify each. Harvested cross-bundle contracts → one-line
   gospel rails, not new docs.

## Phase 3 — the gate, then retirement

1. Present ONE disposition map (file → recomposed-into) + the retirement list. Operator gates once.
2. `git rm` the island → **live-ref sweep** (lineage prose mentions are fine; fix `cites:` entries and
   "tracked in <island-file>" lines that now point at nothing).
3. Update the locus registries: the pillar's `.claude/subject-routing.yaml` `docs_island` line + the
   census entry.

## Phase 4 — close

Verify all placed docs (`cite-gen --verify`), `locus-drift.py` back at stasis for the locus, selective-stage
commit, and **fold any new lesson back into this recipe** (it is the living artifact).

## Firing order (remaining islands, census-ranked)

1. **qahal** — `QAHAL_API_SPECIFICATION_v1.0.md` (census row 4)
2. **elohim-pillar + shell root** — `ELOHIM_PROTOCOL_ARCHITECTURE.md`, `ARCHITECTURE.md`, shell-root
   copies out of `app/elohim-app/` (rows 5+9 — highest-leverage cleanup)
3. **elohim-storage** — `P2P-ARCHITECTURE`, `EDGE-ARCHITECTURE`, `REACH` (row 10)
4. **doorway-service** — `ARCHITECTURE`, `FEDERATION`, `SCALING`, `RECOVERY-*`, `EDGE-DESIGN` (row 12;
   derived-truth: residue cites the resilience canon)
5. **holochain** — 11 docs incl. `LINK_ARCHITECTURE` (row 11; derived-truth: cites protocol-specification)
6. **steward/node** — `ARCHITECTURE.md`, `P2P-COMPUTE-FOOTPRINT` (row 15; pairs with its create-or-decline
   gospel decision)

One pillar per session; frontend-type loci (elements/graphos) are NOT this recipe — they route to
`/looking-at-frontend`. shefa's session is design-forward (its own seed entry), not island cleanup.
