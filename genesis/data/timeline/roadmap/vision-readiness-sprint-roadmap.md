---
id: "roadmap-vision-readiness-sprint-roadmap"
kind: "roadmap"
contentType: "roadmap-item"
contentFormat: "markdown"
title: "The vision × readiness sprint roadmap (the maintained prioritization home)"
slug: "vision-readiness-sprint-roadmap"
written: "2026-06-02"
author: "cartographer"
status: "active"
target_window: "open-ended"
themes: [prioritization, household-living-core, vision-readiness, dwelling-hub-verification, sprint-ranking, gap-ledger]
relatedNodeIds:
  - "memory:project_household_living_core_lived_contrast_diffusion"
  - "memory:project_dwelling_hub_replication_pattern"
  - "memory:project_rea_compute_commitment_primitive"
  - "memory:project_recovery_grandma_standard"
  - "memory:project_memory_lifecycle_comet_shape"
  - "memory:project_elohim_active_observed_not_flagged"
  - "memory:feedback_check_existing_compute_foundation"
  - "epic:value_scanner"
  - "epic:living_memory"
  - ".claude/memory-kit/Q2-emerging-roadmap.md"
  - ".claude/memory-kit/Q1-canonical-organization.md"
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
>    `project_household_living_core_lived_contrast_diffusion` @ cosine 0.96).
>
> The dates, rankings, OPEN counts, and env-holds below are the **current regeneration**
> (2026-06-02, from `.claude/memory-kit/Q2-emerging-roadmap.md`). When any of the three inputs
> moves — a sprint drains, the operator flips an env flag in `cluster-state.yaml`, or the vision
> re-mine shifts the #1 axis — re-run the cartographer pass and rewrite the body. The frontmatter
> `status: active` says "this roadmap is the live prioritization surface," not "this exact ranking
> is frozen." **If the body is stale against today's `--ledger`/`--focus`, that is drift; close it.**

---

## The vision axis (what ranks everything)

The household is the protocol's **living core** — the seed and the driver, not one of four equals
(`project_household_living_core_lived_contrast_diffusion`). The ranking rule that falls out of this:

- **Rank UP** items that make the **single household** coherent-and-computable — care made visible
  as REA, the grandma-standard recovery that gets her back in, the comet-shape memory that keeps a
  small node from melting, the one elohim per node.
- **Rank DOWN** network-scale / institution-subsumption work. The vision says the seed "composes
  outward without re-architecture," so breadth-first picks (collective coordination, cross-tenant
  proofs) are the *diffusion payoff* — explicitly later than the seed. The architecture INDEX's own
  `realizes:` edges agree: records-lifecycle (the seed cluster) is in-flight; breadth is quarantined.

This axis is the **default reading entry** for the whole roadmap and for the canonical walk
(Q1: the household-led walk is the recommended entry, not chronological-first or breadth-first).

---

## The ledger in one breath (current regeneration)

- **2 CLAIMED-ONLY pressure items** = the dwelling-hub / compute-commitment first instance.
  Built in code, plan-tasks unchecked. → **VERIFICATION track** (below).
- **2 BLOCKED-BY-ENV held items** = `iroh-delivery-master`, `iroh-recovery-e2e`. Need harbor +
  alpha-cluster-6peer + shem. → **DO NOT PICK.**
- **~3,528 OPEN decomposed tasks** across ~147 plans — the implement surface the sprints drain.

---

## §1 — The ranked sprints (1–6, by vision × readiness)

Each sprint cites the plans it *drains* (with live OPEN/CLAIMED counts), its readiness verdict, and
**why it ranks where it does** against the household-living-core axis.

### Sprint 1 — Care-REA observation→Event loop at the household
- **Pillar:** elohim/shefa (core substrate) · lamad (observation vocabulary)
- **Drains:** `records-lifecycle-part-a-primitives-plan` (19 OPEN / 0 CLAIMED),
  `records-lifecycle-applications-plan` (20 OPEN / 0 CLAIMED), partial `value-scanner-content-audit`
  (2 OPEN / 5 CLAIMED). Part-a/applications are ledger-state ACTIVE.
- **Readiness: READY.** Household-local (no cluster dep); the canonical seed
  `2026-05-24-records-lifecycle-design` is in-flight and these plans are its decomposition;
  value-scanner archetypes (~1,700 scenarios / 21 life-stage) already on disk.
- **Why #1:** The one item that maxes **both** axes. It is *care made computable* — the substrate
  primitive that proves the thesis ("care is primary value"), it runs entirely on the one AVAILABLE
  surface (`household-nodes`), and it drains the most-cited canonical spec's OPEN decomposition.

### Sprint 2 — Grandma-standard recovery completion + mutual-aid reciprocal pair
- **Pillar:** imagodei (identity/recovery) · lamad (attestation)
- **Drains:** `recovery-m4-completion-shamir-optional-plan` (98 OPEN / 0 CLAIMED — the bulk), plus
  near-done audit tails to verify-then-close: `recovery-m4-fast-path-revocation` (1 OPEN / 9 CLAIMED),
  `recovery-m4-stage4c-audit` (1/4), `recovery-m4-stage4d-ui-audit` (0 OPEN / 6 CLAIMED — pure verify),
  `recovery-m4-stage1-audit` (2/0).
- **Readiness: MOSTLY READY.** The shamir-optional completion (98 OPEN) is household-local and
  pickable now. The reciprocal **Gertrude↔Dowell** mutual-aid-as-REA-in-kind instance is the first
  concrete composition; the single-household + intimate-circle legs are in scope. **Defer the
  cross-node two-node recovery rehearsal leg** (alpha-cluster-gated).
- **Why #2:** Recovery is the vision's **own designated MVP acceptance test** — "if the substrate
  cannot get her back in, nothing else matters." High vision; readiness just below Sprint 1 (large
  98-OPEN block, one env-gated leg). Sequence household + intimate-circle first; qahal-witness waits.

### Sprint 3 — Living-memory / records-lifecycle substrate gaps + reconciliation
- **Pillar:** elohim (core) · imagodei (agent memory)
- **Drains:** `records-lifecycle-part-d-substrate-gaps-plan` (56 OPEN / 0 CLAIMED), the named
  memory↔records reconciliation (submerge/surface ↔ Active/Subordinate vocabulary; INDEX flags it
  "reconciliation incomplete"), and the governance-multi-factor-check that memory-lifecycle §3.3 defers.
- **Readiness: READY** (ACTIVE, household-local; 56 clean OPEN, no env leg).
- **Why #3:** Living memory is the differentiator vs surveillance capitalism, and the household
  ledger needs comet-shape consolidation to not melt small nodes
  (`project_memory_lifecycle_comet_shape`). Slightly behind Sprint 2 on vision-urgency (recovery is
  the named gate) but cleaner on readiness. Naturally sequences after Sprint 1 (consolidates the
  Events Sprint 1 emits).

### Sprint 4 — Thin edge-elohim: DevContext-stub → first real inference
- **Pillar:** elohim (agent) · imagodei (representation)
- **Drains:** the UNDOCUMENTED-architecture elohim-agent subsystem (code exists — crate / service /
  sdk / gate-client / specialists / mcp-servers — but no `tier: architecture` seed and no decomposed
  plan). Bridges `project_elohim_active_observed_not_flagged` (Phase::ElohimActive from real
  inference; stub = DevContext).
- **Readiness: PARTIALLY READY.** Code exists but is **spec-orphaned** — needs an architecture seed
  authored *before* a clean plan can be decomposed. Pickable as a `/shift` that writes the thin spec
  + wires one real inference path; not yet a clean checkbox-drain.
- **Why #4:** One elohim per node is the protocol's irreducible primitive — the whole apex-anti-capture
  argument rests on it. MVP needs only the *thinnest real* version. Ranks below 1–3 because it lacks a
  decomposed plan (spec must precede implementation) and the care-loop + recovery + memory are the
  harder-load-bearing seed.

### Sprint 5 — Pillar-EPR decomposition + canonical-surface legibility glue
- **Pillar:** cross-pillar (elohim-core owns) · doorway
- **Drains:** `pillar-epr-decomposition-design` (6 OPEN), the legibility gap (5/6 pillars lack a
  CLAUDE.md guide; no pillar↔architecture cross-reference; no Developer-Paths index — Q1's
  recommended `architecture/MAP.md`), `app-manifest-staged-intents-design` (normalize its
  UNKNOWN-STATUS string), and the compositional spec gaps (Pillar-Service spec, Bridge-governance-gate
  spec).
- **Readiness: READY** (all doc/spec-level, household-irrelevant, no env). Low-risk, high-legibility.
- **Why #5:** Pure budget-lowering + onboarding legibility. No new vision advance, but it un-orphans
  built code and writes the missing pillar guides every future sprint reads — the LEGIBILITY/PATH
  discipline's own backlog. *Enabling*, not *seed-advancing*. Run between heavier sprints or in a
  parallel Haiku/Sonnet lane.

### Sprint 6 *(optional / conditional)* — Qahal collective substrate: viewer-symmetry + MVP roadmap
- **Pillar:** qahal · shefa
- **Drains:** `viewer-symmetry-reciprocity-qahal-substrate` (50 OPEN), `qahal-mvp-roadmap` (4 OPEN).
- **Readiness: READY at substrate level** (household-local primitives) but **vision-DOWN-weighted.**
- **Why #6 (and conditional):** Real, ready OPEN surface — but ranking collective/network-scale
  coordination above the single-household seed would **invert the gospel**. The diffusion payoff
  comes *after* the seed is coherent. Held at #6 by design, not by readiness. Pick only after
  Sprints 1–3.

---

## §2 — Verification track (highest-leverage), led by the dwelling-hub first instance

This is the **highest-leverage budget move**: the pressure queue's 2 CLAIMED-ONLY items are
*built-but-unverified*. Verifying them either drops the budget by 2 with zero new code, or surfaces a
real regression early — and it de-risks Sprints 1–3 that build on the same REA compute-commitment
primitive (`project_rea_compute_commitment_primitive`, `project_dwelling_hub_replication_pattern`).

**Lead item — Dwelling-hub mutual storage replication (REA compute-commitment first instance):**
- Plan `2026-05-28-mutual-storage-replication-dwelling-hub-plan.md` — ledger CLAIMED-ONLY (doc
  claims done); decomposed items **77 OPEN / 0 CLAIMED** (checkboxes never ticked). Corroborated by
  `sprint3-storage-replication-implementation-notes.md` (CLAIMED-ONLY, 5 OPEN).
- **It really landed in code** (git, since 2026-05-25): `replicates-dwelling` commitment writer,
  mishpat integrity defense-in-depth, two-conductor sweettest, `replication_prioritizer` wired
  end-to-end, capacity/mutuality-audit routes, per-scale `mutuality_audit_service`. Anchors in
  `elohim/holochain/dna/mishpat/zomes/`.
- **But explicit stubs remain** (commit messages self-report): `commitmentBackedReplication` and
  `replication_commitments` are "Sprint-3 stubs"; `find_counter/emit` stubbed in
  `mutuality_audit_service`.
- **ci-investigator job** (on `household-nodes` only): (1) run the two-conductor `replicates-dwelling`
  sweettest and confirm green; (2) walk the 77 plan steps against actual code, flip genuinely-landed
  ones to verified, leave the named stubs as honest OPEN; (3) report which "Sprint-3 stub" gaps are
  real remaining work vs already-filled. Output drops the CLAIMED-ONLY count and produces an honest
  residual-OPEN list for a follow-on sprint.

**Second verify item — recovery audit tails** (fold into Sprint 2's open): `recovery-m4-stage4d-ui-audit`
(0 OPEN / 6 CLAIMED — pure verify), `recovery-m4-fast-path-revocation` (1/9), `recovery-m4-stage4c-audit`
(1/4), `value-scanner-content-audit` (2/5). Mostly-CLAIMED audits whose verification closes them cheaply.

**Discipline:** the verification track runs on the AVAILABLE surface (`household-nodes`) only. Anything
whose verification needs harbor/alpha/shem belongs in §3, not here.

---

## §3 — BLOCKED-BY-ENV — do NOT pick now

Per `--focus` @ cluster-state 2026-06-01: **UNAVAILABLE = alpha-cluster-6peer (degraded),
harbor-registry (false), shem (false).** These are HELD, *not regressed* — they cannot be validated,
so they must not be ranked.

- **`iroh-delivery-master.md`** — needs `['harbor','alpha-cluster-6peer']`, both missing. **HOLD.**
- **`iroh-recovery-e2e.md`** — needs `['shem']`, missing (27 OPEN / 6 CLAIMED, unverifiable). **HOLD.**
- **Harbor-registry SPOF** (backlog, HIGH, recurring 2×): blocks all CI + SSR delivery.
  **Operator-domain topology decision** (HA multi-replica or pull-through mirror), not code-pickable
  by a dev sprint. The SSR-deploy leg is BLOCKED here.
- **Alpha-cluster 6-peer** (degraded): the cross-node legs of Sprint 2's recovery rehearsal and any
  two-node soak. Keep these legs out of scope; the single-household + intimate-circle legs stay in.
- **Shem** (false): multi-tenant canvas node — gates `iroh-recovery-e2e` and any cross-tenant proof.

**Rule:** none of these enter a sprint until the operator flips `cluster-state.yaml`. The scope
cascades immediately on edit — re-run `--focus` and regenerate this roadmap after any operator
topology change.

---

## §4 — Single highest-leverage next move

**Run ci-investigator on the dwelling-hub / mutual-storage-replication first instance (§2 lead item)
on `household-nodes`.**

Why this beats starting Sprint 1:
- It is the **only pressure that lowers the budget with zero new code** — the 2 CLAIMED-ONLY items
  are the entire "needs-action-but-built" surface. Verifying them is pure stasis progress.
- It is the **REA compute-commitment first proving instance** — the primitive the whole household
  care-economy (Sprint 1) and dwelling resilience rest on. Confirming it green de-risks Sprints 1–3
  before they build on it; finding the stubs are load-bearing reshapes them.
- It runs entirely on the AVAILABLE surface — no env unblock, no operator dependency.
- The grounding is unambiguous: code landed (10+ commits), plan unchecked, commit messages name the
  exact residual stubs. ci-investigator has a precise, bounded job and a clean honest-residual output.

**Then** start **Sprint 1 (care-REA observation→Event loop)** as the first *forward* sprint — it maxes
both axes and runs on the same available surface.

---

## Vision × readiness scoreboard (current regeneration)

| # | Sprint | Pillar | Vision | Readiness | OPEN drained | Env |
|---|--------|--------|:------:|:---------:|--------------|-----|
| 1 | Care-REA observation→Event loop | elohim/shefa/lamad | 10 | 9 | 39 (part-a 19 + apps 20) | none |
| 2 | Grandma recovery + mutual-aid pair | imagodei/lamad | 9 | 7 | 98 + audit tails | partial (cross-node leg gated) |
| 3 | Living-memory / records substrate gaps | elohim/imagodei | 9 | 9 | 56 | none |
| 4 | Thin edge-elohim (stub→real inference) | elohim/imagodei | 9 | 5 | spec-orphaned (write seed first) | none |
| 5 | Pillar-EPR + legibility glue | cross-pillar/doorway | 5 | 9 | 6 + doc/spec gaps | none |
| 6 | Qahal collective substrate *(cond.)* | qahal/shefa | 5 | 8 | 50 + 4 | none (vision-deferred) |
| V | **Verify dwelling-hub (highest-leverage)** | shefa/elohim | — | — | 77 plan-steps + 5 notes (verify) | none |
| ⛔ | iroh-delivery / iroh-recovery-e2e | infra | — | — | HELD | harbor / alpha / shem |

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

The frontmatter stays `status: active`; only the operator retires this entry. A regeneration that
finds the rankings unchanged still **re-stamps the dated regeneration header** so the next reader
knows it was checked, not merely stale. When a sprint fully drains (its plans hit 0 OPEN), the
cartographer drops it from §1 and the historian writes a `chronicle/` entry recording the moment —
two entries, one moment (CONVENTIONS open-question 2).

## Related
- `.claude/memory-kit/Q2-emerging-roadmap.md` — the cartographer scratch-synthesis this regeneration draws from
- `.claude/memory-kit/Q1-canonical-organization.md` — the canonical-walk axis (household-led reading entry)
- `genesis/docs/content/elohim-protocol/architecture/INDEX.md` — the `realizes:` graph this ranking honors
- `genesis/data/timeline/roadmap/memory-team-as-triadic-os.md` — the "what's next in <60s" capability this artifact realizes
- `.claude/agents/cartographer.md` — the owner; §"Output discipline" #3 (persistent roadmap entries)
- `genesis/docs/PLACEMENT.md` — the placement contract the ledger/focus inputs derive from
