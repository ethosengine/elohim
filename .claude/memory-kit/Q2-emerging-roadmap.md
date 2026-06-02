# Q2 — Emerging Prioritized Developer-Sprint Roadmap

**Cartographer synthesis · 2026-06-02 · READ-ONLY (scratch doc)**
Axis: **vision × readiness**, ranked toward the gospel-tier priority — *the household is the living core, the seed and driver, not one of four equals* (`project_household_living_core_lived_contrast_diffusion.md`, re-mined 2026-06-02 @ cosine 0.96). Rank UP items that make the **single household** coherent-and-computable; rank DOWN network-scale / institution-subsumption work that the vision says "composes outward without re-architecture."

Grounding sources: `placement-audit.py --ledger` (386 files; 3 PRESSURE, 2 HELD, 381 SETTLED), `placement-audit.py --focus` (cluster-state @ 2026-06-01: household-nodes AVAILABLE; alpha-cluster-6peer degraded, harbor false, shem false), the 149 decomposed gap-items, and B1/B2 survey. Per-doc OPEN/CLAIMED counts below are read directly from `.claude/memory-kit/gap-items/*.json` (`state` field), not estimated.

---

## The ledger in one breath

- **2 CLAIMED-ONLY pressure items** = the dwelling-hub / compute-commitment first instance. Built in code, plan-tasks unchecked. → **VERIFICATION track** (§2).
- **2 BLOCKED-BY-ENV held items** = `iroh-delivery-master`, `iroh-recovery-e2e`. Need harbor + alpha-cluster-6peer + shem. → **DO NOT PICK** (§3).
- **1 UNKNOWN-STATUS** = `app-manifest-staged-intents-design` — normalize the status string (trivial, folded into Sprint 5).
- **3,528 OPEN decomposed tasks** across 147 plans — the implement surface the sprints below drain.

---

## §1 — The next 4–6 candidate sprints (ranked by vision × readiness)

### Sprint 1 — Care-REA observation→Event loop at the household *(records-lifecycle Part A + applications)*
- **Pillar:** elohim/shefa (core substrate) · **lamad** (content/observation vocabulary)
- **Drains:** `records-lifecycle-part-a-primitives-plan` (**19 OPEN / 0 CLAIMED**), `records-lifecycle-applications-plan` (**20 OPEN / 0 CLAIMED**), partial `value-scanner-content-audit` (**2 OPEN / 5 CLAIMED**). Both part-a/applications are ledger-state **ACTIVE** (open work, not blocked).
- **Status:** **READY.** Substrate is household-local (no cluster dep); canonical seed `2026-05-24-records-lifecycle-design` is in-flight/canonical and the part-a/applications plans are its decomposition. value-scanner archetypes (1,700 scenarios / 21 life-stage) already on disk.
- **Why #1:** This is B2's #1 load-bearing theme — *care made computable*. It is the substrate primitive that proves the thesis ("care is primary value"), it runs entirely on `household-nodes` (the one AVAILABLE surface), and it lowers the budget by closing the most-cited canonical spec's OPEN decomposition. Highest vision **and** highest readiness — the only item that maxes both axes.

### Sprint 2 — Grandma-standard recovery completion + mutual-aid reciprocal pair *(Recovery M4 tail)*
- **Pillar:** imagodei (identity/recovery) · lamad (attestation)
- **Drains:** `recovery-m4-completion-shamir-optional-plan` (**98 OPEN / 0 CLAIMED** — the bulk), plus near-done audit tails to verify-then-close: `recovery-m4-fast-path-revocation` (1 OPEN / **9 CLAIMED**), `recovery-m4-stage4c-audit` (1/4), `recovery-m4-stage4d-ui-audit` (**0 OPEN / 6 CLAIMED** — pure verify), `recovery-m4-stage1-audit` (2/0).
- **Status:** **MOSTLY READY** — the *shamir-optional completion* (98 OPEN) is household-local and pickable now. The reciprocal **Gertrude↔Dowell** mutual-aid-as-REA-in-kind instance is named in `resilience/README.md` as the first concrete composition; the substrate side is household-local, but a full two-node recovery rehearsal touches alpha-cluster (defer that leg, keep the single-household + intimate-circle leg).
- **Why #2:** B2 names recovery as the vision's **own designated MVP acceptance test** — "if the substrate cannot get her back in, nothing else matters." High vision. Readiness slightly below Sprint 1 because the 98-OPEN block is large and the cross-node rehearsal leg is env-gated. Sequence the household + intimate-circle recovery first; the qahal-witness leg waits on cluster.

### Sprint 3 — Living-memory / records-lifecycle substrate gaps + reconciliation *(comet shape)*
- **Pillar:** elohim (core) · imagodei (agent memory)
- **Drains:** `records-lifecycle-part-d-substrate-gaps-plan` (**56 OPEN / 0 CLAIMED**), the named-but-unspecified memory↔records reconciliation (submerge/surface ↔ Active/Subordinate vocabulary; A1 flags INDEX.md "reconciliation incomplete"), and the governance-multi-factor-check that memory-lifecycle §3.3 defers.
- **Status:** **READY** (ACTIVE state, household-local). Part-d is substrate gaps under the canonical records-lifecycle umbrella — no cluster dependency.
- **Why #3:** B2 #3 — living memory is the differentiator vs surveillance capitalism and is load-bearing because *the household ledger needs comet-shape consolidation to not melt small nodes*. Slightly behind Sprint 2 on vision-urgency (recovery is the named acceptance gate) but ahead on readiness-cleanliness (56 clean OPEN, no env leg). Naturally sequences after Sprint 1 (it consolidates the Events Sprint 1 emits).

### Sprint 4 — Thin edge-elohim: DevContext-stub → first real inference *(autonomous entity, minimal scope)*
- **Pillar:** elohim (agent) · imagodei (representation)
- **Drains:** the **UNDOCUMENTED-architecture** elohim-agent subsystem (A3 zone 1 — code exists: elohim-agent crate/service/sdk/gate-client/specialists/mcp-servers, but no `tier: architecture` seed and no decomposed plan). Bridges to `project_elohim_active_observed_not_flagged` (Phase::ElohimActive from real inference; stub = DevContext).
- **Status:** **PARTIALLY READY** — code exists but is **spec-orphaned**; needs an architecture seed authored *before* a clean plan can be decomposed (A3's recommended Elohim-Agent spec). Pickable as a `/shift` that writes the thin spec + wires one real inference path; not yet a clean checkbox-drain.
- **Why #4:** B2 #4 — one elohim per node is the protocol's irreducible primitive and the whole apex-anti-capture argument rests on it. MVP needs only the *thinnest real* version. Ranks below 1–3 because it lacks a decomposed plan (readiness penalty: spec must precede implementation) and the household care-loop + recovery + memory are the harder-load-bearing seed.

### Sprint 5 — Pillar-EPR decomposition + canonical-surface legibility glue *(structural / hygiene)*
- **Pillar:** cross-pillar (elohim-core owns) · doorway
- **Drains:** `pillar-epr-decomposition-design` (**6 OPEN**, requirement-bullets), the A2/A3 legibility gap (5/6 pillars lack a CLAUDE.md guide; no pillar↔architecture cross-reference; no Developer-Paths index), `app-manifest-staged-intents-design` (**UNKNOWN-STATUS** → normalize), and the A3 compositional spec gaps (Pillar-Service spec, Bridge-governance-gate spec — "Gap 9").
- **Status:** **READY** (all doc/spec-level, household-irrelevant, no env). Low-risk, high-legibility-leverage.
- **Why #5:** Pure budget-lowering + onboarding legibility. No new vision advance, but it un-orphans built code (A3 zone 1/3) and writes the missing pillar guides that every future sprint reads. Ranks here because it is *enabling* not *seed-advancing* — do it between heavier sprints or in parallel with a Haiku/Sonnet lane.

### Sprint 6 *(optional / conditional)* — Qahal collective substrate: viewer-symmetry + MVP roadmap *(diffusion edge)*
- **Pillar:** qahal · shefa
- **Drains:** `viewer-symmetry-reciprocity-qahal-substrate` (**50 OPEN**), `qahal-mvp-roadmap` (4 OPEN, requirement-bullets).
- **Status:** **READY at substrate level** (household-local primitives) but **vision-DOWN-weighted**: B2 ranks collective/network-scale coordination as the *diffusion payoff*, explicitly later than the single-household seed. Pick only after Sprints 1–3 make the seed coherent.
- **Why #6 (and conditional):** Real OPEN surface, genuinely ready, but ranking it above the seed would *invert the gospel*. The architecture INDEX's own `realizes:` edges put records-lifecycle (seed cluster) in-flight and quarantine breadth — a breadth-first pick diverges from both the gospel memory and the canonical graph. Held at #6 by design, not by readiness.

---

## §2 — Verification track (CLAIMED → ci-investigator), led by the dwelling-hub / compute-commitment first instance

This is the **highest-leverage budget move** (see §4): the pressure queue's 2 CLAIMED-ONLY items are *built-but-unverified*, so verifying them either drops the budget by 2 with zero new code, or surfaces a real regression early.

**Lead item — Dwelling-hub mutual storage replication (REA compute-commitment first instance):**
- Plan: `2026-05-28-mutual-storage-replication-dwelling-hub-plan.md` — ledger **CLAIMED-ONLY** (doc claims done); decomposed items **77 OPEN / 0 CLAIMED** (checkboxes never ticked). Corroborated by `sprint3-storage-replication-implementation-notes.md` (also CLAIMED-ONLY, 5 OPEN).
- **It really landed in code** (git, since 2026-05-25): `replicates-dwelling` commitment writer (`0f346db9e`), mishpat integrity defense-in-depth validation (`87b1464d5`), two-conductor sweettest (`3464d8d15`), `replication_prioritizer` wired end-to-end (`21cb7e8b3`), capacity/mutuality-audit routes (`3a13d44be`), per-scale `mutuality_audit_service` (`cf0110c7a`). Code anchors confirmed in `elohim/holochain/dna/mishpat/zomes/`.
- **But explicit stubs remain** (commit messages self-report): `commitmentBackedReplication` and `replication_commitments` are "Sprint-3 stubs" (`8ce097757`); `find_counter/emit` stubbed in `mutuality_audit_service` (`cf0110c7a`).
- **ci-investigator job:** (1) run the two-conductor `replicates-dwelling` sweettest and confirm green on `household-nodes`; (2) walk the 77 plan steps against actual code and flip genuinely-landed ones to verified, leaving the named stubs as honest OPEN; (3) report which of the "Sprint-3 stub" gaps are real remaining work vs already-filled. Output drops the CLAIMED-ONLY count and produces an honest residual-OPEN list for a follow-on sprint.

**Second verify item — recovery audit tails** (fold into Sprint 2's open): `recovery-m4-stage4d-ui-audit` (**0 OPEN / 6 CLAIMED** — pure verify), `recovery-m4-fast-path-revocation` (1 OPEN / **9 CLAIMED**), `recovery-m4-stage4c-audit` (1/4), `value-scanner-content-audit` (2 OPEN / **5 CLAIMED**). These are mostly-CLAIMED audits whose verification closes them cheaply.

**Discipline:** ci-investigator runs on the **AVAILABLE** surface (`household-nodes`) only. Anything whose verification needs harbor/alpha/shem belongs in §3, not here.

---

## §3 — BLOCKED-BY-ENV — do NOT pick now

Per `--focus` @ cluster-state 2026-06-01: **UNAVAILABLE = alpha-cluster-6peer (degraded), harbor-registry (false), shem (false).** These are HELD, *not regressed* — they cannot be validated, so they must not be ranked.

- **`iroh-delivery-master.md`** — needs `['harbor','alpha-cluster-6peer']`, both missing. **HOLD.**
- **`iroh-recovery-e2e.md`** — needs `['shem']`, missing (27 OPEN / 6 CLAIMED, but unverifiable). **HOLD.**
- **Harbor-registry SPOF** (backlog, HIGH, recurring 2× — 2026-04-28 / 2026-05-30): blocks all CI + SSR delivery. **Operator-domain topology decision** (HA multi-replica or pull-through mirror), not code-pickable by a dev sprint. The SSR-deploy leg is BLOCKED here too (`cf53a76c2`).
- **Alpha-cluster 6-peer** (degraded): the cross-node legs of Sprint 2's recovery rehearsal and any two-node soak. Keep these legs out of the sprint scope; the single-household + intimate-circle legs stay in.
- **Shem** (false): multi-tenant canvas node — gates `iroh-recovery-e2e` and any cross-tenant proof.

**Rule:** none of these enter a sprint until the operator flips `cluster-state.yaml`. The scope cascades immediately on edit, so re-run `--focus` after any operator topology change.

---

## §4 — Single highest-leverage next move

**Run ci-investigator on the dwelling-hub / mutual-storage-replication first instance (§2 lead item) on `household-nodes`.**

Why this beats starting Sprint 1:
- It is the **only pressure that lowers the budget with zero new code** — the 2 CLAIMED-ONLY items are the entire "needs-action-but-built" surface. Verifying them is pure stasis progress.
- It is the **REA compute-commitment first proving instance** — the substrate primitive the whole household care-economy (Sprint 1) and dwelling resilience rest on. Confirming it green *de-risks Sprints 1–3* before they build on it; finding the stubs are load-bearing reshapes them.
- It runs entirely on the AVAILABLE surface — no env unblock needed, no operator dependency.
- The grounding is unambiguous: code landed (10+ commits), plan unchecked, commit messages name the exact residual stubs. ci-investigator has a precise, bounded job and a clean honest-residual output.

**Then** start **Sprint 1 (care-REA observation→Event loop)** as the first *forward* sprint — it maxes both axes and runs on the same available surface.

---

## Vision × readiness scoreboard (summary)

| # | Sprint | Pillar | Vision | Readiness | OPEN drained | Env |
|---|--------|--------|:------:|:---------:|--------------|-----|
| 1 | Care-REA observation→Event loop | elohim/shefa/lamad | 10 | 9 | 39 (part-a 19 + apps 20) | none |
| 2 | Grandma recovery + mutual-aid pair | imagodei/lamad | 9 | 7 | 98 + audit tails | partial (cross-node leg gated) |
| 3 | Living-memory / records substrate gaps | elohim/imagodei | 9 | 9 | 56 | none |
| 4 | Thin edge-elohim (stub→real inference) | elohim/imagodei | 9 | 5 | spec-orphaned (write seed first) | none |
| 5 | Pillar-EPR + legibility glue | cross-pillar/doorway | 5 | 9 | 6 + doc/spec gaps | none |
| 6 | Qahal collective substrate *(cond.)* | qahal/shefa | 5 | 8 | 50 + 4 | none (but vision-deferred) |
| V | **Verify dwelling-hub (highest-leverage)** | shefa/elohim | — | — | 77 plan-steps + 5 notes (verify) | none |
| ⛔ | iroh-delivery / iroh-recovery-e2e | infra | — | — | HELD | harbor / alpha / shem |
