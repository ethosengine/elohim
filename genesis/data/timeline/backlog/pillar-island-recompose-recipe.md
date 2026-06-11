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
5. **Re-tense drafts against the disposition map and the session's own repairs** (doorway lesson):
   fan-out agents write before dispositions/repairs are final — one draft cited a retiring sibling as
   "staying in-tree" (a `cites:` that would go DEAD at the gate; belongs in `derived_from:`), another
   reported a code-comment defect in present tense after the session had already repaired it. At
   placement, sweep every draft for references to (a) island files on the retirement list and (b)
   defects the Phase 0-1 drift repairs already fixed.
6. Seal doc-roots (`cite-gen --seal`, serial); verify each. Harvested cross-bundle contracts → one-line
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

## Fan-out prompts (paste-ready, one per session — firing order)

Each block below opens its own session verbatim. The RAILS are identical for all and live in this
file's Phases 1-4 (the session reads this recipe first); each block carries only what is pillar-specific:
locus, island inventory (verified 2026-06-11), and the liveness/caution lines earned from the census's
corrected-truth column. The `ultracode` keyword authorizes the Phase-1 agent fan-out. One pillar per
session. Frontend-type loci (elements/graphos) are NOT this recipe — they route to `/looking-at-frontend`.

### 1 — qahal

```
Run the QAHAL island recompose per the proven recipe:
genesis/data/timeline/backlog/pillar-island-recompose-recipe.md — read it first; it IS the
method (4 phases, authorship rules, gate). This prompt only scopes the session. ultracode.

LOCUS: qahal — subject home `qahal-domain-gospel` (elohim/sdk/domains/qahal/CLAUDE.md, cites
qahal-architecture-vision), consumer `qahal-pillar-gospel` (app/elohim-app/src/app/qahal/CLAUDE.md).
Truth: self + sdk vocabulary home. Census row 4.

ISLAND: app/elohim-app/src/app/qahal/QAHAL_API_SPECIFICATION_v1.0.md (no shell-root copy —
verified 2026-06-11). Phase 0 sweeps inbound refs (MAP.md and pillar gospels reference it).

LIVENESS CAUTIONS: governance vocabulary is CO-OWNED — qahal manifest
(collective/proposal/challenge/appeal/statement) vs the mishpat DNA (`mishpat-domain-gospel`
is the judgment substrate qahal escalates into; do NOT recompose mishpat's share into qahal
canon). Verify spec claims against elohim/sdk/domains/qahal/manifest.json + the mishpat zomes
before any CANONIZE verdict. Psephos renders formal ballots (levels 3+); casual governance is
Angular — keep that ladder straight in the seed.

RAILS: this recipe §Phases 1-4, non-negotiable. One operator gate before retirement.
```

### 2 — elohim pillar + shell sub-manifest

```
Run the ELOHIM-PILLAR island recompose per the proven recipe:
genesis/data/timeline/backlog/pillar-island-recompose-recipe.md — read it first. ultracode.

LOCUS: elohim (protocol-core pillar) — subject home `elohim-domain-gospel`
(elohim/sdk/domains/elohim/CLAUDE.md, cross-cutting signalKinds/constitutionalRatios),
consumer `elohim-pillar-gospel` (app/elohim-app/src/app/elohim/CLAUDE.md). Truth: SELF for the
TS models (`models/` is canonical — census verifier correction), but protocol-wide *architecture
prose* is likely superseded by `elohim-protocol-specification` + the anchored sdk canon. Census rows 5+9.

ISLAND: app/elohim-app/src/app/elohim/{ELOHIM_PROTOCOL_ARCHITECTURE.md, ARCHITECTURE.md}
(no shell-root copies — verified 2026-06-11).

LIVENESS CAUTIONS: ELOHIM_PROTOCOL_ARCHITECTURE.md is the most-referenced island doc in the
repo — qahal/imagodei pillar gospels carry "**Architecture:**" pointer lines at minimum, and
MAP/docs reference it; the inbound-ref sweep is the critical step. Expect heavy SUPERSEDED
verdicts (protocol truth now lives in elohim-protocol-specification + sdk schemas); the residue
test is "still true AND homed nowhere else." Also in scope: census row 9's shell decision —
declare app/elohim-app/.claude/subject-routing.yaml (multi-pillar delivery locus, consumer of
sdk/domains + elohim-storage; sub-manifests stay DECLARATIVE until deep-merge lands).

RAILS: this recipe §Phases 1-4. One operator gate before retirement.
```

### 3 — elohim-storage

```
Run the ELOHIM-STORAGE island recompose per the proven recipe:
genesis/data/timeline/backlog/pillar-island-recompose-recipe.md — read it first. ultracode.

LOCUS: elohim-storage — gospel `elohim-storage-gospel` (elohim/elohim-storage/CLAUDE.md,
truth: SELF for the HTTP/blob/P2P surface; cites tiered-quilt-stewardship-design). Census row 10.

ISLAND: elohim/elohim-storage/{P2P-ARCHITECTURE.md, EDGE-ARCHITECTURE.md, REACH.md}.

LIVENESS CAUTIONS: EDGE-ARCHITECTURE.md describes DOORWAY edge performance — part of it may
belong to the doorway locus, not storage (decide per-section, don't move wholesale). REACH.md
must be reconciled against the LIVE reach machinery — substrate enforcement in
elohim/epr/src/reach.rs + the 8-value reach.schema.json + the known three-vocabulary drift
([[project_reach_enum_drift_reconciliation]]) — do not canonize a stale reach vocabulary.
P2P-ARCHITECTURE (dual-plane: Holochain control / storage data) composes with
tiered-quilt-stewardship-design and the three-layer truth model — cite, never restate.

RAILS: this recipe §Phases 1-4. One operator gate before retirement.
```

### 4 — doorway-service

```
Run the DOORWAY-SERVICE island recompose per the proven recipe:
genesis/data/timeline/backlog/pillar-island-recompose-recipe.md — read it first. ultracode.

LOCUS: doorway-service — gospel `doorway-service-gospel` (doorway/doorway-service/CLAUDE.md).
Truth: DERIVED — these docs are refinements OF the resilience canon (`resilience-protocol-spec`,
Parts V/VI); residue cites genesis, never claims truth:self. Census row 12.

ISLAND: doorway/doorway-service/{ARCHITECTURE.md, FEDERATION.md, SCALING.md, REACH.md,
RECOVERY-PROTOCOL.md, RECOVERY-SPRINT-PLAN.md, EDGE-DESIGN.md} (7 docs).

LIVENESS CAUTIONS: doorway/CLAUDE.md (the parent gospel) lists all 7 under "Reference
Documentation" — that section must be rewritten as part of retirement (inbound-ref repair).
RECOVERY-SPRINT-PLAN is plan-shaped → decompose to history + verified-open backlog, not seed.
The no-per-domain-proxy + no-blob-fan-out rules are ALREADY gospel in doorway/CLAUDE.md —
dedupe toward the gospel, never restate in residue. SCALING's two-axis model + graduation
flywheel may be the strongest CANONIZE candidate — verify against the live deployment first.

RAILS: this recipe §Phases 1-4. One operator gate before retirement.
```

### 5 — holochain (biggest island — consider two passes)

```
Run the HOLOCHAIN island recompose per the proven recipe:
genesis/data/timeline/backlog/pillar-island-recompose-recipe.md — read it first. ultracode.

LOCUS: holochain substrate — gospel `holochain-integrity-layer-gospel`
(elohim/holochain/dna/CLAUDE.md). Truth: DERIVED — implements `elohim-protocol-specification`;
residue cites the protocol canon, never claims truth:self. Census row 11.

ISLAND (11 docs, two clusters):
- dna/: LINK_ARCHITECTURE.md, NETWORK_UPGRADES.md, SCHEMA_VERSIONS.md
- docs/: ARCHITECTURE.md, ARCHITECTURE-GAP.md, P2P-DATAPLANE.md, COMMUNITY-COMPUTE.md,
  SYNC-ENGINE.md, DEPLOYMENT-RUNTIMES.md, DEVELOPMENT.md, REACH.md
If one session can't hold it, split: dna/ cluster first (load-bearing: link architecture,
upgrade/schema governance), docs/ cluster second.

LIVENESS CAUTIONS: census verifier flagged a broken /elohim-node/ reference and stale paths in
this tree — verify every pointer. docs/claude.md is lowercase (invisible to the cite graph) —
normalize as part of the pass. LINK_ARCHITECTURE is integrity-layer governance: verify against
the actual zome link types before CANONIZE. zome-sweettest-sync applies if any zome source is
touched (it should NOT be — docs only). DNA workspaces stay plain cargo (no CARGO_TARGET_DIR).

RAILS: this recipe §Phases 1-4. One operator gate before retirement.
```

### 6 — steward/node (pairs with its gospel decision)

```
Run the STEWARD-NODE island recompose per the proven recipe:
genesis/data/timeline/backlog/pillar-island-recompose-recipe.md — read it first. ultracode.

LOCUS: steward/node — NO gospel exists yet (held create-or-decline, census). Step 1 of this
session IS the decision: create `steward-node-gospel` (steward/node/CLAUDE.md) as an
implementation-crate gospel citing the orchestration epic ABOVE it
(elohim-hub-boundaries-design) — the layered-drift rule: an implementation crate cites the
epic, it is not its own domain. Census row 15.

ISLAND: steward/node/ARCHITECTURE.md (+ check simulation/ — P2P-COMPUTE-FOOTPRINT lives there;
it was judged simulation/analysis, likely research-home or stay, not retire).

LIVENESS CAUTIONS: verify ARCHITECTURE.md against the live crate (libp2p 0.53 — macros+ed25519
features, with_codec(), StreamExt::next()) and against elohim-hub-boundaries-design before any
CANONIZE; the hub-composition content belongs to the epic, the crate-mechanics to the gospel.

RAILS: this recipe §Phases 1-4. One operator gate before retirement.
```

### (shefa — different recipe)

shefa's session is design-forward, seeded at
`genesis/data/timeline/backlog/shefa-sensemaking-surface-session-seed.md` (four-surface lens,
exchange-definition EPRs through the p2p-design-gate). Its small islands
(README-EXCHANGE.md, README-INSURANCE-MUTUAL.md, banking-bridge/README.md) ride that session
as a Phase-0 side-task using this recipe's rules — not a separate island session.
