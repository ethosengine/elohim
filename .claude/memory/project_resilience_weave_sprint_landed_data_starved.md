---
index: false
name: project_resilience_weave_sprint_landed_data_starved
title: Resilience weave sprint landed; card is data-starved
description: All 5 lens routes landed on dev 2026-06-22; the dark resilience card is DATA-STARVED (shard_locations empty, no live writer), not lens/fold-starved.
metadata: 
  node_type: memory
  type: project
  originSessionId: a0632634-fa0a-4b7a-8546-a7e1a5d6f0ab
---

The resiliency-card + P2P-weave sprint (`2026-06-21-resiliency-card-p2p-weave-sprint-plan`, Waves 0–5) **all landed on `dev` 2026-06-22** (`498acb00a`→`637ea8f6d`). **All FIVE facings/lenses now have live routes** — resiliency `/api/v1/resilience/{cid}`, operational-weave `/api/v1/weave` (`c6805597d`, lit + CoverageRollup descent `1e7bc2d89`), REA `/api/v1/commitments/facing/rea` (`ea31e0bf7`), reach/projection `/api/v1/peer-topology`, EPR `/api/v1/epr/{cid}/{raw,envelope}`. The 2026-06-21 plan's "operational-weave route 0%" was **stale when written**.

**The live resilience card reading `stewardingCollectives 0` / `diversity 0%` / `no region data` is DATA-STARVED, NOT a missing lens/fold.** All three derive from ONE root: `diversity_score = min(stewarding,max(commitment,1))/7` (`household_resilience.rs:210`) → `stewarding_hubs()` → `load_holder_relation()` INNER JOIN on `shard_locations`, which is **empty in practice** (no live writer). Folds are correct — Slice-0 proof-gate `populated_relation_lights_stewarding_regional_and_intra_hub` (`46baef5e5`) passes with coherent rows. See [[project_resilience_snapshot_humans_junction]].

**Why shard_locations is empty:** `distribute_shards` is wired + on-by-default (`p2p/mod.rs:1489`, writes `status="announced"`) but only produces rows against a healthy mesh; production lighting is **leak-gated** (conductor OOM, jemalloc swap `ed111a5cc` un-soak-verified — [[project_storage_metrics_surface_and_leak_verdict]]). `commitmentBacked=2` IS lit (adam cell-enable, `4115e01fc`).

**Next real frontier (not a new lens):**
1. **Wave 1.3 — live `distribute_shards` household e2e** (the plan's own "single highest-leverage first slice"). Household-testable NOW on the M/J/J mesh ([[feedback_household_nodes_is_the_stable_floor]]); only sustained alpha is leak-gated.
2. **`alpha-cluster-6peer` DEGRADED** (≈10/13 peers crashlooping since build #1024; code-regression-vs-env-down unresolved) blocks all alpha acceptance — operator/CI-owned.
3. Genuinely-next *lens-dimension* work = **reach-vocabulary reconciliation** (roadmap 13, [[project_reach_enum_drift_reconciliation]]) — completes the reach/projection facing; REA `compute-fulfilled` emitter is built-but-dormant on purpose (forgeable without attestation; don't wire).

Recurring trajectory check: the dark card is the [[feedback-cleanup-toward-p2p-dataplane-trajectory]] error in miniature — blobs don't auto-replicate yet, so per-host/per-row reads zero until the dataplane actually moves bytes.
