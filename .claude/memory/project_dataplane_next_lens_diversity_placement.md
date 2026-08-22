---
index: false
name: project_dataplane_next_lens_diversity_placement
title: Dataplane next lens = diversity placement
description: Diversity-aware salvage placement (1a+1b landed) is INERT in prod — household_id NULL from identity-coherence gaps, not scope reads; degrades safely to XOR.
metadata:
  node_type: memory
  type: project
  originSessionId: a0632634-fa0a-4b7a-8546-a7e1a5d6f0ab
  modified: 2026-07-21T14:01:02.280Z
---

After the Che op-gate (governed distribution) landed, an 11-agent grounding workflow
(`wf_197318fc-f51`, 5 ground → 5 adversarial-verify → synth, all claims source-confirmed) picked the
**next dataplane lens**: the **household-diversity-aware salvage `PlacementStrategy`** (blob-custody P3-8).

**The dataplane is 3 layers at 3 maturities** (conflating them is the recurring error): SUBSTRATE
(byte movement — RS(4-7) coding + household-first ingest selector + XOR salvage all WIRED; XOR salvage
shipped `6a1070f9a`, the spec's P3-1…6 "OPEN" table is STALE) · AGGREGATION/FACING (the resilience
card — `elohim-facings` pure folds landed, card reads 0 from DATA-STARVATION not missing lens; plus
`diversity_score` is a mislabeled `…/7` coverage proxy, the real fold `compute_fault_domain_diversity`
exists at `distribution_view.rs:114` but only feeds the distribution facing) · GOVERNANCE (op-gate,
shipped; live loop held). **The ONE genuine substrate hole:** salvage re-placement is household-BLIND
(only prod strategy was `XorDistanceStrategy`; salvage pool built `household_id: None` at
`salvage_commitment_author.rs:152`, hard-selects XOR `:163`) → autonomous salvage can silently
re-converge replicas into one household, eroding the diversity ingest established, INVISIBLY to the
op-gate loop. This is the only "make diversity real" piece the held live loop will NOT deliver.

**Slice 1a LANDED** (commit `9e0f84f4b`, `feat/frontend-eyes-sprint`): `DiversityAwarePlacementStrategy`
in `elohim/elohim-storage/src/reconcile/placement.rs` — pure deterministic diversity-first multi-pass
greedy, XOR tiebreak; failure-domain = `household_id` or per-cid sentinel so a NO-household pool
degrades EXACTLY to XOR (never worse than XOR; safe to wire before 1b). 21 unit tests. **Adversarial
review caught a real agreement break** (duplicate `agent_cid` with CONFLICTING households made dedup
input-order-dependent under the stable sort) → fixed with failure_domain as a final total-order sort
tiebreak + regression test. fmt+clippy clean.

**1b LANDED** (commit `6cca8927b`, `feat/frontend-eyes-sprint`, **option B chosen** = join
`humans.household_id` at pool-build, NOT the gossip-ad fork A which would self-report households = new
trust surface + wire change). `build_salvage_candidates` mirrors the ingest selector
(`peer_selection.rs:184-203`); `select_placement_strategy` picks DiversityAware vs XOR behind
`config.salvage_diversity_placement` (default ON, env `SALVAGE_DIVERSITY_PLACEMENT`). 8 salvage tests +
config default; fmt/clippy clean. 4-lens adversarial review (`wf_c550e523-8c6`) found NO blocker.

**KEY FINDING — the join is INERT in production (a 4th, more precise dormancy than the handoff's
NULL-`agent_pub_key` one):** household-bearing `humans` rows are WRITTEN under `h_app_id="imagodei"`
(`api/identity.rs:112` register_human; `genesis_self_heal.rs:109`; membership projection
`controller.rs:1105-1115` UPDATEs household onto the imagodei row in place) but the salvage join — like
the **ingest selector** (`distribute_shards(.., "lamad")`) AND the **resilience card** (reads
`ctx.h_app_id`="lamad") — READS under `"lamad"`. So `"lamad"` returns zero household rows → every
candidate stays None → diversity degrades exactly to XOR. This is SAFE (never worse than XOR) and a
faithful mirror of ingest, so 1b ships the decision logic + plumbing fixture-verified; **production
efficacy waits on a SHARED substrate scope reconciliation** (backlog
`resilience-card-membership-humans-projection-gap-2026-06-19.md`), NOT a salvage-local fix. Do NOT
unilaterally flip salvage to `"imagodei"` (diverges from ingest; plus a 2nd dormancy — `self_cid`/pool
cids may be libp2p/iroh transport ids unless `SELF_CID` pins the agent key). The handoff's
`args.app_id`="elohim" instruction was wrong (would empty harder); `"lamad"` is the correct
ingest-mirroring scope even though it's currently dormant. Comments/tests state this honestly.

**UPDATE 2026-06-27 — the h_app_id scope-split is RESOLVED** (commit `755ade34e`): salvage now joins
the canonical `HUMANS_HAPP_ID="imagodei"` scope, superseding the "do NOT unilaterally flip to
imagodei" guidance above (the flip happened as part of a shared reconciliation, not salvage-local).
Still INERT in prod, but the cause MOVED: `household_id` stays NULL because (1) `agent_pub_key` is
unpopulated and (2) the transport-id vs agent_cid namespace mismatch — substrate-identity-coherence
gates now, not a scope-read bug.

**UPDATE 2026-07-21 — both INERT causes CLOSED on dev (`928bbb5ec..44feb0db0`, identity-coherence
sprint):** (1) `on_membership_projected` now stamps `humans.agent_pub_key = member_agent_key`
(NULL-only, HOUSEHOLD-gated, `is_agent_cid`-guarded) — the 2026-06-15 spec's stopgap shipped; (2) the
salvage/custody path resolves self to agent_cid on WRITE ([session → boot cell-key snapshot], skip+
observe when unresolvable; reconcile re-resolves the session arm PER PASS — no boot-frozen identity)
while READS match legacy transport-id rows too. Also: holder-relation dedup (last-write-wins by
humans.id) prevents duplicate-agent_pub_key diversity inflation; snapshot diversity now the real
`fault_domain_diversity` fold shared via elohim-facings (`FAULT_DOMAIN_TARGET=7`). Remaining for
live efficacy: deploy + humans rows actually carrying keys on alpha (membership signals / heal /
seeder re-author after rekey), and mesh convergence for cross-peer coverage.

**Correctly deferred (live-mesh / shem blocked):** op-gate's own live driving loop; P3-7 cross-peer
"replica count rises" a2o; stage-2 signed capacity ads / real spare_bytes probe; CoverageRollup
recursion (Wave-0 Governor lift). Runners-up: resilience-facings real diversity fold (near-free honesty
companion — route the card through `compute_fault_domain_diversity`); reach-projection vocab bug
(commons graded at lowest replica band — real ~½-day fix, projection not byte-mover). Commit-only;
integrator pushes. See [[project_che_opgate_slice1_plan_ready_held]],
[[project_inventory_exchange_not_byte_replication]], [[feedback-cleanup-toward-p2p-dataplane-trajectory]].
