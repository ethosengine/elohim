# Chapter 7 of the resiliency-saga: custody is witnessed. A shard's custody state
# is not real until a peer OBSERVES and classifies it, not merely intends it. The
# operational-weave fold (elohim/elohim-facings/src/folds/operational_weave.rs
# CustodyClassCounts: none/shelved/stocked/stocked_warm/unknown/observed_lost,
# folded by observed_custody_class_counts()) already computes this classification
# and is unit-tested — but as of this writing no Prometheus gauge exposes it on
# alpha's /metrics yet (Track 3 is wiring the gauge now).
#
# Proof signal: elohim_custody_class_count{class="stocked"} >= 1 — a provisional
# metric name matching the existing elohim_* custody-flow naming convention
# (elohim_custody_announce_total already exists with a "direction" label per the
# same custody-announcement flow in metrics.rs). If Track 3 lands under a
# different literal name, this scenario's honest failure (or pending, if the
# series is simply absent from the scrape) is exactly the signal that the name
# needs reconciling here.
#
# New glue (steps/dataplane/resiliency-saga.steps.ts): the label-aware metric step
# (chapter 2 also uses it) — elohim_-prefixed metrics live on elohim-storage's own
# /metrics, not the doorway's port-8080 /metrics, so this step routes storage-owned
# metric names to the direct storage URL the same way the existing bare-metric step
# routes p2p_/reconcile_/dedup_ prefixes, extended to cover elohim_ too.
#
# Status today: RED/PENDING — born red. The fold exists and is unit-tested; the
# gauge does not exist on alpha's /metrics yet. This chapter is the loop's work
# queue entry for wiring the gauge and deploying it.
@e2e @dataplane @concern:saga-07-custody-witnessed
Feature: Chapter 7 — custody is witnessed
  A shard's custody state is not real until it is OBSERVED and classified, not
  merely intended. The operational-weave fold already computes
  none/shelved/stocked/stocked_warm/unknown/observed_lost counts from custody
  observation rows; this chapter proves that classification is visible on the live
  surface as a labeled gauge. Born red — the gauge does not exist on alpha's
  /metrics yet.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: At least one shard is witnessed as actively stocked
    Then labeled metric "elohim_custody_class_count" with label "class" "stocked" on peer "alpha-A" >= 1
