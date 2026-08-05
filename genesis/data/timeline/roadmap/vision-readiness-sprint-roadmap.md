---
id: "roadmap-vision-readiness-sprint-roadmap"
kind: "roadmap"
contentType: "roadmap-item"
contentFormat: "markdown"
title: "The vision × readiness sprint roadmap (the maintained prioritization home)"
slug: "vision-readiness-sprint-roadmap"
written: "2026-06-02"
regenerated: "2026-07-30"
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

> **This is a MAINTAINED artifact, not a snapshot.** It is the standing prioritization home —
> the roadmap readout of the unified memory loop. The cartographer **regenerates it each
> ceremony** from three live inputs intersected:
>
> 1. **the gap-item ledger** — `placement-audit.py --ledger` (per-file position + state +
>    next-action) and the decomposed `gap-items/*.json` (OPEN = implement / CLAIMED = verify),
> 2. **cluster-state** — `placement-audit.py --focus` (TESTABLE-now vs BLOCKED-BY-ENV, read
>    from `cluster-state.yaml`), and
> 3. **the vision axis** — the gospel-tier priority re-mined each cycle (currently
>    `project_household_living_core_lived_contrast_diffusion`).
>
> **Current regeneration: 2026-07-30** (substrate-currency ceremony, Phase 1b). Prior body was
> written 2026-06-02 and had drifted for ~8 weeks; the drift and its cost are named in
> "What closed since the last regeneration" below. When any of the three inputs moves — a sprint
> drains, the operator flips an env flag in `cluster-state.yaml`, or the vision re-mine shifts the
> #1 axis — re-run the cartographer pass and rewrite the body. The frontmatter `status: active`
> says "this roadmap is the live prioritization surface," not "this exact ranking is frozen.
> **If the body is stale against today's `--ledger`/`--focus`, that is drift; close it.**

---

## The vision axis (what ranks everything) — RE-MINED, UNCHANGED

The household is the protocol's **living core** — the seed and the driver, not one of four equals
(`project_household_living_core_lived_contrast_diffusion`). Re-mined this cycle against the
manifesto ("lived contrast is the diffusion mechanism... get the household real, and the rest
follows") and the architecture `MAP.md` ("if you read one path first, read the household path").
The apex axis did **not** shift. The ranking rule that falls out of it:

- **Rank UP** items that make the **single household** coherent-and-computable — care made visible
  as REA, the grandma-standard recovery that gets her back in, the comet-shape memory that keeps a
  small node from melting, the one elohim per node.
- **Rank DOWN** network-scale / institution-subsumption work. The vision says the seed "composes
  outward without re-architecture," so breadth-first picks (collective coordination, cross-tenant
  proofs) are the *diffusion payoff* — explicitly later than the seed.

This axis is the **default reading entry** for the whole roadmap and for the canonical walk.

---

## What closed since the last regeneration (do not re-rank these)

- **The 2026-06-02 §4 recommendation is DONE.** It named "run ci-investigator on the dwelling-hub /
  mutual-storage-replication first instance." That verification **completed 2026-06-06** — the plan
  now reads `status: landed`, `landed_commit: a169ab72c`, `verified_by: ci-investigator 2026-06-06 —
  19/20 tasks verified on disk`. The sole gap (`replicates_dwelling_integration.rs`) was aspirational
  and never task-assigned. **Closed.**
- **`iroh-recovery-e2e` is also complete** (`status: complete`, `landed_commit: d5e29fb67`,
  `verified_by: ci-investigator 2026-06-06`). The old §3 held it "unverifiable until shem"; shem
  returned and the verdict landed. **Closed — removed from §3.**
- **The legibility premise of the old Sprint 5 partly closed**: the Developer-Paths index the old
  body asked for now exists as `genesis/docs/content/elohim-protocol/architecture/MAP.md`.

**The cost of the drift, stated plainly.** For ~8 weeks §4 pointed at a completed action, and every
sprint it gated drained **zero** OPEN items. A roadmap that names a finished move is worse than no
roadmap: it reads as a live gate. The structural correction in this regeneration is that **§4 is now
a forward move and the verification discipline is a named parallel lane (§2) with its own owner** —
never again a gate that parks the seed.

---

## The ledger in one breath (measured 2026-07-30)

- **4,164 OPEN decomposed gaps across 212 docs** (was ~3,528 / ~147 at the 2026-06-02 regeneration).
- **651 CLAIMED gaps** (checked ≠ done — the verify surface).
- **23 BLOCKED-BY-ENV gaps**: 22 in the held `iroh-delivery-master`, 1 in `sdk-promise-substrate-program-plan`.
- **Pressure queue: 203 files** — MEM-UNLINKED 108 · UNKNOWN-STATUS 51 · NEEDS-TRIAGE 28 ·
  CLAIMED-ONLY 10 · SUPERSEDED 6.
- **All four `genesis/docs/_state/{blockers,regression,unverified,needs-triage}` pressure dirs are empty.**

**Read the growth honestly — it is mostly measurement, not debt.** The doc count nearly doubled
(147→212) because decomposition coverage widened, and a large share of the OPEN total is
**landed-but-unchecked** work, not work to do. Two proven instances: the dwelling-hub plan carries
**77 OPEN** and `iroh-recovery-e2e` carries **27 OPEN** — 104 phantom OPEN from two plans that are
both `verified_by: ci-investigator 2026-06-06`. **The checkbox is not the verdict**; the frontmatter
`verified_by` is. Treat the OPEN total as an upper bound, never as a workload estimate.

**One comparison that is NOT apples-to-apples:** the 2026-06-02 body's "2 CLAIMED-ONLY pressure
items" counted *docs sitting in the `_state/unverified/` dir*; the 651 above counts decomposed
CLAIMED *gaps*. These are different units — there was no 2→651 explosion. What is true: the pressure
dir the old §2 pointed at is now empty, so that track's premise is **restated below**, not renumbered.

---

## §1 — The ranked sprints (by vision × readiness)

### Sprint 1 — REA rails at the household: economic-event emit + commitment graduation
- **Pillar:** shefa/elohim (core substrate) · mishpat (bounds)
- **Drains:** `2026-06-08-epr-acquisition-slice2a-rea-rails-plan.md` — **21 OPEN / 0 CLAIMED**,
  ledger-state ACTIVE, `requires_env: [household-nodes]`, IN SCOPE per `--focus`.
- **Readiness: READY (highest).** Partially landed already: Task 2 (`call_create_rea_economic_event`
  in `conductor_writes.rs`) and Task 3 (`economic_event_emit_service.rs`, commit `22dfc00db`
  2026-06-08, whose message names "slice-2a T3") are **on disk but unchecked**. Remaining real work
  is bounded: T4 commitment graduation (`proposed → active` — no `graduate` fn exists yet), T5 the
  commitment→content scorer-data stub, T6 the two-conductor sweettest
  (`tests/rea_event_emit_graduation_e2e.rs` — does not exist) that proves the whole rail.
- **Why #1:** It maxes **both** axes. It is *care made computable* in code — the notarized
  EconomicEvent bounded by a commitment, which is the substrate primitive the whole care economy
  composes from (`project_rea_compute_commitment_primitive`). It runs entirely on `household-nodes`,
  the stable floor. And it is the **direct downstream of the move that just closed**: its own `cites`
  name the dwelling-hub design as "the REA compute-commitment instance-1 design whose floor-check gap
  + replicates-commons reservation this rail unblocks." The verification gate was paid on 2026-06-06;
  this is what it unblocked. **Its first act is to reconcile its own checkboxes against T2/T3 on
  disk** — folding the §2 discipline into the forward move instead of gating behind it.

### Sprint 2 — Grandma-standard recovery completion + mutual-aid reciprocal pair
- **Pillar:** imagodei (identity/recovery) · lamad (attestation)
- **Drains:** `recovery-m4-completion-shamir-optional-plan` (**98 OPEN / 0 CLAIMED** — the bulk),
  plus near-done audit tails to verify-then-close: `recovery-m4-fast-path-revocation-kickoff` (1/9),
  `recovery-m4-stage4c-audit` (1/4), `recovery-m4-stage4d-ui-audit` (0 OPEN / 6 CLAIMED — pure
  verify), `recovery-m4-brainstorm` (2/7), `recovery-m4-stage1-audit` (2/0).
- **Readiness: MOSTLY READY.** The shamir-optional completion is household-local and pickable now.
  The reciprocal **Gertrude↔Dowell** mutual-aid-as-REA-in-kind instance is the first concrete
  composition. **The cross-node soak leg stays deferred** — `alpha-cluster-6peer` is the one
  remaining degraded resource. Note the cross-stack *harness* leg is no longer a blocker: the
  shem-gated `iroh-recovery-e2e` landed and was verified 2026-06-06.
- **Why #2:** Recovery is the vision's **own designated MVP acceptance test** — "if the substrate
  cannot get her back in, nothing else matters." Vision is near-max; readiness sits below Sprint 1
  because of the single large 98-OPEN block and the alpha-gated soak leg.

### Sprint 3 — Living-memory / records-lifecycle substrate gaps + reconciliation
- **Pillar:** elohim (core) · imagodei (agent memory)
- **Drains:** `records-lifecycle-part-d-substrate-gaps-plan` (**56 OPEN / 0 CLAIMED**), the named
  memory↔records reconciliation (submerge/surface ↔ Active/Subordinate vocabulary), and the
  governance-multi-factor-check that memory-lifecycle §3.3 defers.
- **Readiness: READY** (ACTIVE, household-local, no env leg).
- **Why #3:** Living memory is the differentiator vs surveillance capitalism, and the household
  ledger needs comet-shape consolidation to not melt small nodes
  (`project_memory_lifecycle_comet_shape`). Sequences naturally after Sprint 1 — it consolidates the
  Events Sprint 1 emits.

### Sprint 4 — Records-lifecycle **spec completion** *(demoted from #1; reclassified)*
- **Pillar:** elohim (architecture surface)
- **Drains:** `records-lifecycle-part-a-primitives-plan` (19 OPEN), `records-lifecycle-applications-plan`
  (20 OPEN), `records-lifecycle-master-orchestration` (19), `wave2-findings-addendum` (9),
  `phase2-findings-synthesis` (8).
- **Readiness: READY** (doc-tier, no env, parallelizable across agents).
- **Why demoted, in one line:** the 2026-06-02 body ranked these **#1 as "care made computable,"**
  but reading the plans shows part-a's goal is *"replace the seven stubbed primitive walkthrough
  sections... in records-lifecycle-design.md"* — **spec authoring, not runtime code.** The vision
  claim belonged to the REA rails (now Sprint 1); these are the canonical-surface completion behind
  it. High value, honestly re-labelled. Good parallel-lane work for a Sonnet fleet.

### Sprint 5 — Thin edge-elohim: DevContext-stub → first real inference
- **Pillar:** elohim (agent) · imagodei (representation)
- **Drains:** the elohim-agent subsystem (crate / service / sdk / gate-client / specialists /
  mcp-servers) — code exists, no `tier: architecture` seed of its own, no decomposed plan. Bridges
  `project_elohim_active_observed_not_flagged` (Phase::ElohimActive from real inference; stub =
  DevContext).
- **Readiness: PARTIALLY READY — still spec-orphaned** (re-checked 2026-07-30: `elohim-agent` is
  referenced across the architecture corpus but owns no architecture seed). Pickable as a `/shift`
  that writes the thin spec **and** wires one real inference path; not a clean checkbox-drain.
- **Why #5:** One elohim per node is the protocol's irreducible primitive — the whole
  apex-anti-capture argument rests on it. Ranks below 1–4 only because spec must precede decomposition.

### Sprint 6 — Pillar-EPR decomposition *(demoted; recharacterized)*
- **Pillar:** cross-pillar (elohim-core owns) · doorway
- **Drains:** `2026-05-25-pillar-epr-decomposition-plan` (**143 OPEN**) + its design (6 OPEN / 4 CLAIMED).
- **Readiness: READY but LARGE.**
- **Why demoted, in one line:** the 2026-06-02 body sized this from the *design*'s 6 OPEN and called
  it "low-risk legibility glue to run between heavier sprints" — the **plan** carries 143 OPEN, so it
  is a heavy sprint in its own right, not filler. Its companion legibility premise also partly closed
  (`architecture/MAP.md` now exists). Re-rank on merit, not on a mis-sized count.

### Sprint 7 *(conditional)* — Qahal collective substrate: viewer-symmetry + MVP roadmap
- **Pillar:** qahal · shefa
- **Drains:** `viewer-symmetry-reciprocity-qahal-substrate` (50 OPEN), `qahal-mvp-roadmap` (4 OPEN).
- **Readiness: READY at substrate level** (household-local primitives) but **vision-DOWN-weighted.**
- **Why last (and conditional):** real, ready OPEN surface — but ranking collective/network-scale
  coordination above the single-household seed would **invert the gospel**. Held here by design, not
  by readiness. Pick only after Sprints 1–3.

---

## §2 — Verification track, RESTATED: checkbox-vs-verdict reconciliation

The old §2 pointed at the `_state/unverified/` pressure dir. **That dir is empty** (2026-07-30), so
the old premise is gone. The premise that replaces it is sharper and better-evidenced:

**The ledger's OPEN count is inflated by landed-and-verified work whose checkboxes were never
ticked.** Proven twice this cycle — dwelling-hub (77 OPEN, `verified_by` 2026-06-06) and
`iroh-recovery-e2e` (27 OPEN, `verified_by` 2026-06-06) — and observed a third time inside Sprint 1
itself (slice-2a T2/T3 on disk since commit `22dfc00db`, unchecked). The verdict lives in
frontmatter; the checkbox lags it.

**Two bounded jobs, both on `household-nodes`:**

1. **Reconcile verified-landed plans against their checkboxes.** Start with the two proven instances
   (104 phantom OPEN between them), then sweep plans carrying `status: landed|complete` **and** a
   `verified_by:` line **and** a nonzero OPEN count. This is budget honesty with zero new code.
2. **Verify the CLAIMED-ONLY queue** — 10 docs, **56 CLAIMED / 2 OPEN** aggregate, all
   `ACTIVE:plans`: `auth-wire-contract-completion` (13 CLAIMED), `jenkins-seed-bearer-gate` (15),
   `sdk-core-entrypoints` (11), `plural-mishpat-lenses-service-layer` (11 CLAIMED / 2 OPEN),
   `elohim-facings-crate-extraction` (6), plus `limitarian-governor-v1`, `frontend-eyes-sprint`,
   `graphos-look`, `closure-posture-axis-card`, `third-party-gate-closure`.

**Ownership and discipline.** This track is a **parallel lane, not a gate** — dispatch it to
ci-investigator / the librarian's `memory-stasis-loop`, and do **not** park §1 behind it. That
inversion is exactly what cost the last 8 weeks. It runs on the AVAILABLE surface only; anything
needing `alpha-cluster-6peer` belongs in §3.

**Also cheap, also parallel:** NEEDS-TRIAGE is 28 docs but carries **zero decomposed gaps** — it is
pure classification (17 of the 28 are one coherent family: the 2026-06-13/14 self-healing / dataplane
/ federation plans). Low leverage per unit effort but near-zero cost; librarian lane. SUPERSEDED is 6
(5 of them the conductor-leak RCA arc) — distill to `history/`, which is a **grow**-target museum,
never force-shrunk.

---

## §3 — BLOCKED-BY-ENV — do NOT pick now

Per `--focus` against `cluster-state.yaml` (schema `updated: 2026-06-04`), read 2026-07-30:
**AVAILABLE = harbor-registry, household-nodes, observability, shem. UNAVAILABLE =
alpha-cluster-6peer (degraded).**

**This section shrank sharply and that is the headline.** The 2026-06-02 body held three resources
unavailable (harbor false, shem false, alpha degraded); two have since been restored with
evidence-backed flips (harbor 2026-06-03, shem operator-confirmed 2026-06-20 — the earlier "offline"
note had been stale prose contradicting `available: true`). Only one hold remains:

- **`held/plans/2026-05-10-iroh-delivery-master.md`** — 22 gaps needing
  `['harbor-registry','alpha-cluster-6peer']`. **HOLD** (alpha is the blocker; harbor returned).
- **`2026-07-25-sdk-promise-substrate-program-plan.md`** — 1 gap on `alpha-cluster-6peer`. The other
  40 OPEN gaps in that plan are testable; **hold the gap, not the plan** (scope is gap-granular).
- **Alpha-cluster 6-peer** (degraded, 10/13 peers CrashLooping): the cross-node soak legs of
  Sprint 2. Keep those legs out of scope; single-household + intimate-circle legs stay in.

**Vocabulary drift to reconcile:** `2026-06-29-p2p-dataplane-validation-suite-plan.md` declares
`@requires:multi-node` on 3 gaps; `multi-node` is not a resource name in `cluster-state.yaml`, and
an unknown cap conservatively blocks escape. **Reconcile it to `household-nodes`, not `shem`.** The
plan's own §Env-scope says household M/J/J *is* multi-node and instructs "do NOT tag those
`@requires:shem`"; `shem` is for *cross-tenant* scenarios. Mapping it to `shem` would over-constrain
household-testable work off the plate — the precise failure the stable-floor rule exists to prevent
(a 3-node household is itself a live multi-peer mesh). Reconcile per-gap where a scenario genuinely
spans doorways rather than nodes.

**Rule:** nothing here enters a sprint until `cluster-state.yaml` moves. Scope cascades immediately
on edit — re-run `--focus` and regenerate this roadmap after any operator topology change.

---

## §4 — Single highest-leverage next move

**Finish and prove Sprint 1 — the REA rails: `2026-06-08-epr-acquisition-slice2a-rea-rails-plan.md`
(21 OPEN / 0 CLAIMED) on `household-nodes`.**

*Pre-authored Objective (drop-in for `/shift`):* Reconcile the slice-2a plan's checkboxes against
what is already on disk (T2 `call_create_rea_economic_event`, T3 `economic_event_emit_service` —
commit `22dfc00db`), then implement the residual rail: T4 commitment graduation (`proposed → active`),
T5 the commitment→content scorer-data stub, and T6 the two-conductor sweettest
`elohim/elohim-storage/tests/rea_event_emit_graduation_e2e.rs` that proves a bounds-validated
EconomicEvent emits, notarizes, and projects end-to-end. Estimated 2–3 cycles.

**Grounding (why this is real, and satisfiable now):**
- It is **actually open**: 21 OPEN / 0 CLAIMED in the live ledger, state ACTIVE, and the three
  headline residuals were each checked against the tree today — no `graduate` fn in
  `rea_commitment_service.rs`, no `rea_event_emit_graduation_e2e.rs` on disk.
- It is **satisfiable on the available substrate**: `requires_env: [household-nodes]`, which is
  AVAILABLE and is the stable floor (`feedback_household_nodes_is_the_stable_floor`). No
  alpha-cluster dependency, no operator unblock.
- Its **precondition is already paid**: it cites the dwelling-hub design as the instance-1 whose
  floor-check gap it unblocks, and that instance was verified 2026-06-06. This is literally the move
  the previous §4 was sequencing toward.

**Why it beats the runners-up:**
- *vs. Sprint 2 (recovery, 98 OPEN):* higher vision-per-cycle and far better readiness — recovery is
  one undifferentiated 98-item block with an alpha-gated leg; the rails are a 3-task residual on a
  partially-landed foundation.
- *vs. §2 verification (56 CLAIMED, cheap):* the verification lane is cheaper but it is **hygiene,
  not seed advance** — and making it §4 is precisely the failure mode of the last regeneration. It
  runs in parallel with its own owner, and Sprint 1's first act performs it locally anyway.
- *vs. Sprint 4 (records-lifecycle spec completion):* that work documents the primitives; this work
  *makes them fire*. Care made computable beats care described.

**Then** continue into Sprint 2 (recovery) as the second forward sprint — same available surface.

**This is the same move a `next-actions.md` handoff menu should name at the top of the cycle.** If a
menu and this section ever disagree, the disagreement — not the ranking — is the bug.

---

## Operator-decision items (not code-pickable by a sprint)

- **`2026-06-14-vision-gap-care-valueflows-stub.md` (O2 — Native Observed-Care → REA Valueflow
  Emitter)** is `status: GREENLIGHT-TO-EXPAND` and states in its own body: *"The value/governance
  core needs the operator's blessing before expansion."* It is the natural downstream of Sprint 1
  and cannot be ranked into a sprint until the operator greenlights it. **Decision requested.**
- **Alpha-cluster-6peer**: `cluster-state.yaml` records that whether the 10/13 CrashLooping peers are
  a code regression or env-down is **UNRESOLVED**. Until an operator/investigation verdict lands, the
  soak legs stay held and cannot earn a regression cascade.
- **Horizon scan freshness**: latest is `.claude/memory-kit/horizon-scans/2026-05-14.md` — 77 days
  old at this regeneration. Under the 90-day gate, so no scan was run; it will trip on the next
  ceremony after ~2026-08-12.

---

## Vision × readiness scoreboard (regeneration 2026-07-30)

| # | Sprint | Pillar | Vision | Readiness | OPEN drained | Env |
|---|--------|--------|:------:|:---------:|--------------|-----|
| 1 | **REA rails — emit + graduation** | shefa/elohim/mishpat | 10 | 9 | 21 (T2/T3 already on disk) | none |
| 2 | Grandma recovery + mutual-aid pair | imagodei/lamad | 9 | 7 | 98 + audit tails | partial (soak leg alpha-gated) |
| 3 | Living-memory / records substrate gaps | elohim/imagodei | 9 | 9 | 56 | none |
| 4 | Records-lifecycle **spec** completion *(demoted)* | elohim | 8 | 9 | 75 (doc-tier) | none |
| 5 | Thin edge-elohim (stub→real inference) | elohim/imagodei | 9 | 5 | spec-orphaned (write seed first) | none |
| 6 | Pillar-EPR decomposition *(demoted, re-sized)* | cross-pillar/doorway | 5 | 7 | 143 | none |
| 7 | Qahal collective substrate *(cond.)* | qahal/shefa | 5 | 8 | 50 + 4 | none (vision-deferred) |
| V | Verification lane *(parallel, not a gate)* | — | — | — | 56 CLAIMED + 104 phantom OPEN | none |
| ⛔ | iroh-delivery-master / sdk-promise 1 gap | infra | — | — | HELD (23 gaps) | alpha-cluster-6peer |
| ✅ | ~~dwelling-hub verification~~ · ~~iroh-recovery-e2e~~ | — | — | — | CLOSED 2026-06-06 | — |

---

## Regeneration contract (how the cartographer keeps this current)

Each memory ceremony (or `/converge` pass), the cartographer rewrites the body above from:

1. **`placement-audit.py --ledger`** → refresh the pressure queue, CLAIMED-ONLY count, and per-plan
   OPEN/CLAIMED counts (read from `gap-items/*.json` `state` fields, **never estimated**).
2. **`placement-audit.py --focus`** → refresh §3 from current `cluster-state.yaml`; move any newly-
   AVAILABLE surface's work *into* a sprint, move any newly-degraded surface's work *out*.
3. **Vision re-mine** → re-query the gospel-tier #1 axis (`mempalace_search` + the manifesto Part II
   principles). If the apex priority shifts, re-rank — UP for single-household-coherence, DOWN for
   network-scale breadth.

**Two disciplines this regeneration added, learned the hard way:**
- **§4 must be re-checked for completion first.** A next-move that has already landed is the most
  expensive kind of staleness — it reads as a live gate and parks everything behind it.
- **Size a sprint from the plan, not from its design.** Sprint 6 was mis-ranked for 8 weeks because
  the 2026-06-02 body read 6 OPEN off the design while the plan carried 143.

The frontmatter stays `status: active`; only the operator retires this entry. A regeneration that
finds the rankings unchanged still **re-stamps the dated regeneration header** so the next reader
knows it was checked, not merely stale. When a sprint fully drains (its plans hit 0 OPEN), the
cartographer drops it from §1 and the historian writes a `chronicle/` entry recording the moment —
two entries, one moment (CONVENTIONS open-question 2).

## Related
- `genesis/docs/content/elohim-protocol/architecture/MAP.md` — the household-first canonical walk this ranking honors
- `genesis/docs/content/elohim-protocol/architecture/INDEX.md` — the `realizes:` graph
- `.claude/memory-kit/Q1-canonical-organization.md` — the canonical-walk axis (household-led reading entry)
- `genesis/data/timeline/roadmap/memory-team-as-triadic-os.md` — the "what's next in <60s" capability this artifact realizes
- `.claude/agents/cartographer.md` — the owner; §"ROADMAP-CURRENCY mandate"
- `genesis/docs/PLACEMENT.md` — the placement contract the ledger/focus inputs derive from
