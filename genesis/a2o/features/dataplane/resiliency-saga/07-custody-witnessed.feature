# Chapter 7 of the resiliency-saga: custody is witnessed. A shard's custody state
# is not real until a peer OBSERVES and classifies it, not merely intends it. The
# operational-weave fold (elohim/elohim-facings/src/folds/operational_weave.rs
# CustodyClassCounts: none/shelved/stocked/stocked_warm/unknown/observed_lost,
# folded by observed_custody_class_counts()) already computes this classification
# and is unit-tested. The Prometheus gauge is also registered and emitted:
# `emit_custody_class_gauges` (elohim-storage/src/api/mod.rs) publishes all six
# `elohim_custody_class_count{class=...}` series and rides the GET /api/v1/weave
# sweep — but it is gated on an active local session (an observer's agent_cid
# must be resolvable) and emits nothing rather than an unattributed count when
# one isn't. A red here means the weave sweep hasn't run with an active session
# on alpha yet, not that the gauge is unwired.
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
# Status today: RED/PENDING. The fold exists and is unit-tested; the gauge is
# registered and wired to the weave sweep. This chapter's work queue entry is
# ensuring the sweep runs with an active local session on alpha so the series is
# actually populated in the live scrape — not building the gauge from scratch.
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
    Then labeled metric "elohim_custody_class_count" with label "class" "stocked" on peer "alpha-A" >= 1
