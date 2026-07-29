# Chapter 7 of the resiliency-saga: custody is witnessed. A shard's custody state
# is not real until a peer OBSERVES and classifies it, not merely intends it. The
# operational-weave fold (elohim/elohim-facings/src/folds/operational_weave.rs
# CustodyClassCounts: none/shelved/stocked/stocked_warm/unknown/observed_lost,
# folded by observed_custody_class_counts()) already computes this classification
# and is unit-tested. The Prometheus gauge is also registered and emitted:
# `emit_custody_class_gauges` (elohim-storage/src/api/mod.rs:782) publishes all six
# `elohim_custody_class_count{class=...}` series.
#
# Trigger truth (discovered 2026-07-29): the sweep is 100% REQUEST-triggered —
# its only call site is the GET /api/v1/weave handler (api/mod.rs:282). It is NOT
# one of the periodic background sweeps its two sibling gauges ride (identity-fill
# has run_fill_loop, custodian-capacity has run_report_loop; custody-class has no
# loop). Nothing on alpha calls /api/v1/weave — no CI scenario (the one feature
# that does, resilience/operational-weave.feature, is fully @wip and filtered out
# of Dataplane Validation) and no app consumer — so the series had ZERO samples
# in Prometheus retention until a manual probe fired it. Hence the When step
# below: issue the same GET a real WeaveView consumer would, then poll.
# The active-local-session gate is NOT a blocker on alpha: genesis self-heal
# (GENESIS_SELF_HEAL_IDENTITY=1) mints an ambient session at boot.
#
# Proof signal: elohim_custody_class_count{class="stocked"} >= 1 — matching the
# existing elohim_* custody-flow naming convention (elohim_custody_announce_total
# already exists with a "direction" label per the same custody-announcement flow
# in metrics.rs).
#
# New glue (steps/dataplane/resiliency-saga.steps.ts): the label-aware metric step
# (chapter 2 also uses it) — elohim_-prefixed metrics live on elohim-storage's own
# /metrics, not the doorway's port-8080 /metrics, so this step routes storage-owned
# metric names to the direct storage URL the same way the existing bare-metric step
# routes p2p_/reconcile_/dedup_ prefixes, extended to cover elohim_ too.
#
# Station discovered mid-flight (2026-07-29) — between sweep-fires → stocked>=1,
# missing node: shard→blob resolution + custody commitment. With the sweep firing,
# the fold honestly reports unknown=32, stocked=0 on alpha-A: derive_class
# (services/custody_facing.rs:332-364) sends a shard to `unknown` when no
# shard_manifests row maps its shard_hash to a blob_hash — true for ALL 32 of
# matthew's shard_locations rows — and `stocked` further needs an active
# rea_commitments row (action='custody-blob', state='active') naming the peer.
# Probe: /api/v1/weave then /metrics; current state: sweep fires (station green
# once the When lands), shard_manifests resolution empty (red), custody-blob
# commitment absent (red). The finish line stays exactly where it was.
@e2e @dataplane @concern:saga-07-custody-witnessed
Feature: Chapter 7 — custody is witnessed
  A shard's custody state is not real until it is OBSERVED and classified, not
  merely intended. The operational-weave fold already computes
  none/shelved/stocked/stocked_warm/unknown/observed_lost counts from custody
  observation rows; this chapter proves that classification is visible on the live
  surface as a labeled gauge. The gauge is wired (rides the /api/v1/weave sweep,
  gated on an active local session) — red here means the sweep hasn't populated
  the series on alpha yet, not that the gauge is missing.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: At least one shard is witnessed as actively stocked
    When I read "/api/v1/weave"
    Then labeled metric "elohim_custody_class_count" with label "class" "stocked" on peer "alpha-A" >= 1
