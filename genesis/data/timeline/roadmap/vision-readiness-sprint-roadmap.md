---
id: "roadmap-vision-readiness-sprint-roadmap"
kind: "roadmap"
contentType: "roadmap-item"
contentFormat: "markdown"
title: "The vision × readiness sprint roadmap (the maintained prioritization home)"
slug: "vision-readiness-sprint-roadmap"
written: "2026-06-02"
regenerated: "2026-08-26"
author: "cartographer"
status: "active"
target_window: "open-ended"
themes: [prioritization, household-living-core, vision-readiness, rea-rails, sprint-ranking, gap-ledger, checkbox-verdict-drift]
relatedNodeIds:
  - "memory:project_household_living_core_lived_contrast_diffusion"
  - "memory:project_dwelling_hub_replication_pattern"
  - "memory:project_rea_compute_commitment_primitive"
  - "memory:project_recovery_grandma_standard"
  - "memory:project_memory_lifecycle_comet_shape"
  - "memory:project_elohim_active_observed_not_flagged"
  - "memory:feedback_household_nodes_is_the_stable_floor"
  - "epic:value_scanner"
  - "epic:living_memory"
  - "genesis/docs/content/elohim-protocol/architecture/MAP.md"
  - "genesis/docs/content/elohim-protocol/architecture/INDEX.md"
tags: [roadmap, prioritization, maintained-artifact, regenerated-each-ceremony, vision-readiness]
---

# The vision × readiness sprint roadmap

> **This is a MAINTAINED artifact, not a snapshot.** The cartographer regenerates it each
> memory ceremony and `/converge` from the decomposed gap ledger, the current environment focus,
> and the gospel-tier vision axis.
>
> **Current regeneration: 2026-08-26** (substrate-currency ceremony, Phase 1b). The body was stale:
> `notary-authority` had flipped green, alpha had returned, the environment hold had drained to zero,
> and the prior §4 still named a completed action. This pass re-ranks from today's measured inputs.

---

## The vision axis (what ranks everything) — re-mined, unchanged

The household remains the protocol's **living core**: foundation, seed, and driver, not one of four
equal examples. The manifesto still names lived contrast as the diffusion mechanism — get the
household real and the rest follows — and `architecture/MAP.md` still makes the household path the
default walk. The ranking rule remains:

- **Rank up** work that makes one household coherent and computable: care expressed as REA,
  grandma-standard recovery, bounded living memory, trustworthy serving, and one active elohim per
  node.
- **Rank down** breadth-first collective or network work unless it is required to make the household
  substrate itself honest. The seed composes outward without re-architecture.

This regeneration used the local gospel sources because the named anchor is represented through
`manifesto.md` and `MAP.md`; the ranking did not move.

---

## What moved since the last regeneration (2026-08-11 → 2026-08-26)

- **The prior §4 move is complete.** `notary-authority` flipped red → green on 2026-08-17 with edge
  #1362 evidence. It no longer belongs at the top of a current roadmap.
- **Alpha returned on 2026-08-20.** `placement-audit.py --focus` now reports five available
  capabilities (`alpha-cluster-6peer`, `harbor-registry`, `household-nodes`, `observability`, `shem`),
  37 declared markdown surfaces in scope, and **0 BLOCKED-BY-ENV**. `owned-substrate` remains false,
  correctly, for a shared fleet; no declared markdown gap is blocked by it.
- **Iroh delivery moved onto the testable plate.** Its master still carries 22 OPEN gaps and a stale
  2026-06 verification block, while August landed discovery, sync initiation, eager announce,
  heal-on-read, transport-matrix parity, and recovery evidence. It is now verify-and-reconcile work,
  not environment-held work and not a license to replay the old plan blindly.
- **`doorway-failover` flipped green** on household-lane evidence (10/10) after edge #1381 proved the
  fleet lane intentionally holds its Act-I scenarios. The current first red is
  **`dataplane-convergence`**.
- **The ledger widened:** 4,332 OPEN / 672 CLAIMED across 221 docs with items (231 files scanned), up
  from 4,249 / 651 across 217 docs (226 files scanned). This is an upper bound, not a workload
  estimate: landed-but-unchecked plans still inflate OPEN.
- **Pressure grew 220 → 253 files:** MEM-UNLINKED 154, UNKNOWN-STATUS 55, NEEDS-TRIAGE 28,
  CLAIMED-ONLY 10, SUPERSEDED 6. That remains a parallel memory lane, not a gate on product work.
- **Measure-ontology slice 1 moved from implementation to verification:** 0 OPEN / 21 CLAIMED. It
  leaves the forward-sprint list and joins §2.

---

## The ledger in one breath (measured 2026-08-26)

- **606 files accounted:** 353 settled and 253 under pressure.
- **4,332 OPEN** decomposed gaps to implement and **672 CLAIMED** gaps to verify across 221 docs with
  items; 231 files were scanned.
- **37 environment-declaring markdown surfaces are testable now; 0 are BLOCKED-BY-ENV.**
- **Pressure queue:** 154 MEM-UNLINKED · 55 UNKNOWN-STATUS · 28 NEEDS-TRIAGE · 10 CLAIMED-ONLY ·
  6 SUPERSEDED.

The units matter. OPEN/CLAIMED are decomposed gaps; pressure counts are files. A checked box is not
a verdict, and a nonzero OPEN count on a `verified_by:` plan may be stale bookkeeping rather than
unfinished behavior.

---

## §1 — Ranked sprints (vision × readiness)

### Sprint 1 — REA rails at the household: economic-event emit + commitment graduation

- **Drains:** `2026-06-08-epr-acquisition-slice2a-rea-rails-plan.md` — **21 OPEN / 0 CLAIMED**,
  `requires_env: [household-nodes]`, in scope.
- **Readiness: READY (highest).** The conductor write wrapper and emit service were already observed
  on disk; the residual rail is commitment graduation, commitment→content scorer data, and the
  two-conductor end-to-end proof. Reconcile the plan against disk before implementing residual work.
- **Why #1:** This is the household vision made executable: care becomes a bounds-validated,
  notarized EconomicEvent rather than prose about value.

### Sprint 2 — Iroh delivery: reconcile landed planes, then close the live verification gates

- **Drains:** `2026-05-10-iroh-delivery-master.md` — **22 OPEN / 0 CLAIMED**, requiring
  `harbor-registry` + `alpha-cluster-6peer`; both are available.
- **Readiness: READY FOR VERIFICATION, NOT BLANKET RE-IMPLEMENTATION.** The master says only 1/12
  gates were independently stable on 2026-06-01, but August landed and measured much of the plane.
  First reconcile every gate against current code, the 3×3 transport matrix, recovery evidence, and
  edge #1380/#1381; only then schedule genuinely missing soaks or rollback proof.
- **Why #2:** It is network-shaped work, so the household axis normally down-ranks it. It rises here
  because the current top red is serving-critical convergence and the now-available verification
  surface can distinguish landed truth from stale checkboxes at low incremental cost.

### Sprint 3 — Grandma-standard recovery completion + mutual-aid reciprocal pair

- **Drains:** `recovery-m4-completion-shamir-optional-plan` (**98 OPEN / 0 CLAIMED**) plus audit tails:
  **6 OPEN / 26 CLAIMED** across fast-path revocation, stage 4c/4d, brainstorm, and stage 1.
- **Readiness: MOSTLY READY.** The environment hold is gone; the remaining problem is the size and
  mixed verification state of the 98-item block. Start by separating landed tails from residual
  behavior, then form the Gertrude↔Dowell mutual-aid-as-REA pair.
- **Why #3:** Recovery is the vision's own acceptance test — if the substrate cannot get her back in,
  nothing else matters — but it is materially larger and less sharply bounded than Sprint 1.

### Sprint 4 — Living-memory / records-lifecycle substrate gaps

- **Drains:** `records-lifecycle-part-d-substrate-gaps-plan` — **56 OPEN / 0 CLAIMED**.
- **Readiness: READY.** Household-local, no environment hold.
- **Why #4:** The household ledger needs bounded, life-shaped consolidation so the events Sprint 1
  emits do not turn small nodes into permanent archives.

### Sprint 5 — Records-lifecycle canonical-surface completion

- **Drains:** part-a primitives 19 OPEN + applications 20 + master orchestration 19 + wave-2 addendum
  9 + phase-2 findings 8 = **75 OPEN / 0 CLAIMED**.
- **Readiness: READY, doc-tier.** Useful parallel work, but it describes the substrate rather than
  making care fire end to end.

### Sprint 6 — Thin edge-elohim: DevContext stub → first real inference

- **Drains:** no decomposed plan; the subsystem remains architecture-seed-orphaned.
- **Readiness: PARTIAL.** Write the smallest canonical seed and wire one real inference path.
- **Why #6:** One elohim per node is irreducible, but the work is not yet a clean ledger drain.

### Sprint 7 — Pillar-EPR decomposition

- **Drains:** plan **143 OPEN / 0 CLAIMED** + design **6 OPEN / 4 CLAIMED**.
- **Readiness: READY but large.** This is a heavy bundle/domain seam, not filler.

### Sprint 8 (conditional) — Qahal collective substrate

- **Drains:** viewer-symmetry 50 OPEN + qahal MVP roadmap 4 OPEN = **54 OPEN / 0 CLAIMED**.
- **Readiness: READY, vision-down-weighted.** Pick after household sprints; collective breadth should
  not displace the seed.

`2026-08-13-agentic-harness-borrows-implementation-plan.md` is also testable now at 47 OPEN, but it
is a development-system improvement rather than a household-product sprint. Keep it in its owning
delivery lane instead of letting infrastructure outrank the living core.

---

## §2 — Verification track (parallel, never a gate)

The corpus carries **672 CLAIMED gaps**. Ten files are CLAIMED-ONLY pressure items; the queue includes
auth-wire contracts, SDK entrypoints, seed-bearer gating, facings extraction, and Mishpat lenses.
Run this track alongside §1, not in front of it.

Three bounded first passes:

1. **Measure ontology slice 1:** 0 OPEN / 21 CLAIMED — verify and close or return precise residuals.
2. **Iroh master reconciliation:** compare its 22 OPEN gates with the August landings and measurements;
   do not trust either the stale self-reported tracker or the stale 2026-06 block without re-checking.
3. **Landed-plan checkbox reconciliation:** identify `status: landed|complete` + `verified_by:` plans
   with nonzero OPEN and correct the ledger only where evidence supports it.

This lane lowers the budget and makes future rankings honest, but it does not substitute for moving
the household seed.

---

## §3 — BLOCKED-BY-ENV

**None in the declared markdown surface.** Today's focus readout is:

- AVAILABLE: `alpha-cluster-6peer`, `harbor-registry`, `household-nodes`, `observability`, `shem`
- UNAVAILABLE: `owned-substrate` (correct for a shared fleet)
- IN SCOPE: 37
- BLOCKED-BY-ENV: **0**

The old alpha hold is retired. If a capability degrades, move only its resolved gap-items out; if
`owned-substrate` is needed, run that destructive scenario on an Act-I household mesh rather than
pretending the shared alpha fleet owns its substrate.

---

## §4 — Single highest-leverage next move

**Measure the latest apex convergence cure on the fleet, then let that evidence choose the next
dataplane action.** `dataplane-convergence` is the habits register's first red. Commit `971857934`
pins `/apps/{resolved-hash}/{file}` cache misses to the bundle hash already resolved, while
`28799e3f5` gives the multi-doorway app pipeline a stage-bounded seed authority. Both are locally
proved and fleet-unmeasured.

*Pre-authored Objective (drop-in for `/shift`):* Deploy the edge revision carrying `28799e3f5` and
`971857934`; run a fresh app landing so all four `seed elohim.host` legs are green; then run
`pnpm look https://elohim.host/` and prove the SSR shell's named main chunk returns 200 with no failed
request in `capture.json`. Record the edge/app build numbers on `dataplane-convergence`. If the apex
is clean, return to the independently observed `p2p.caughtUp` flap and measure that residual before
writing another cure; if the apex still fails, the failed seed or immutable bundle fetch identifies
the next bounded defect.

**Why this is the move:**

- It serves the root work contract: move the first red toward green with proof.
- It is household-serving truth — a household cannot live the substrate if its canonical app shell
  cannot arrive — not breadth for breadth's sake.
- It tests two already-landed cures at deployment cost instead of opening a speculative fourth fix.
- It immediately sharpens Sprint 2: a clean apex isolates remaining convergence work to the peer
  plane; a failed apex keeps the work at the doorway/app boundary.

**Then start Sprint 1, REA rails, in parallel.** The deploy-and-measure loop should not park the
forward household sprint.

This is the move the next `next-actions.md` must name first. If that handoff and this section
disagree, the disagreement is the drift.

---

## Vision × readiness scoreboard (regeneration 2026-08-26)

| # | Sprint | Vision | Readiness | Measured gap surface | Environment |
|---|--------|:------:|:---------:|----------------------|-------------|
| ⚑ | Measure apex cure for `dataplane-convergence` | 10 | 9 | 0 new; deploy + observe landed fixes | available |
| 1 | REA rails — emit + graduation | 10 | 9 | 21 OPEN | household available |
| 2 | Iroh delivery verification | 7 | 8 | 22 OPEN, stale tracker | harbor + alpha available |
| 3 | Grandma recovery + mutual-aid pair | 9 | 7 | 104 OPEN + 26 CLAIMED | available |
| 4 | Living-memory substrate gaps | 9 | 9 | 56 OPEN | available |
| 5 | Records-lifecycle canonical surface | 8 | 9 | 75 OPEN | available |
| 6 | Thin edge-elohim | 9 | 5 | spec-orphaned | available |
| 7 | Pillar-EPR decomposition | 5 | 7 | 149 OPEN + 4 CLAIMED | available |
| 8 | Qahal collective substrate (conditional) | 5 | 8 | 54 OPEN | available |
| V | Verification lane (parallel) | — | — | 672 CLAIMED corpus-wide | available surface only |
| ⛔ | BLOCKED-BY-ENV | — | — | 0 | owned-substrate false, no declared gap blocked |

---

## Operator-decision items (not sprint-ranked)

- `2026-06-14-vision-gap-care-valueflows-stub.md` remains `GREENLIGHT-TO-EXPAND`; its own contract
  asks for operator blessing before expansion. It is a natural downstream of Sprint 1.
- The shared alpha fleet deliberately does not provide `owned-substrate`; destructive process-control
  stories belong on the Act-I household mesh.
- **Horizon freshness:** latest report is 2026-08-13, 13 days old. It is under the 90-day gate, so no
  horizon scan was invoked in this ceremony leg.

---

## Regeneration contract

Every memory ceremony and `/converge` pass:

1. Run `placement-audit.py --ledger`; read OPEN/CLAIMED from each gap-item's `state`, never estimate.
2. Run `placement-audit.py --focus`; rank only testable gaps and move newly available work back in.
3. Re-mine the household-living-core vision axis from the gospel-tier sources.
4. Re-check §4 for completion before carrying it forward.
5. Re-stamp `regenerated` even when rankings hold.

The frontmatter remains `status: active`; only the operator retires this roadmap. When a sprint
drains to 0 OPEN, remove it from §1 and hand the historian the moment for a chronicle entry.

## Related

- `genesis/docs/content/elohim-protocol/architecture/MAP.md` — household-first canonical walk
- `genesis/docs/content/elohim-protocol/architecture/INDEX.md` — realizes graph
- `.claude/memory-kit/Q1-canonical-organization.md` — canonical-walk axis
- `genesis/data/timeline/roadmap/memory-team-as-triadic-os.md` — prioritization capability
- `.claude/agents/cartographer.md` — ROADMAP-CURRENCY owner
- `genesis/docs/PLACEMENT.md` — ledger/focus placement contract
