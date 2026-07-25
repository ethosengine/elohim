# Chapter 8 of the resiliency-saga: capacity is reported. Each custodian reports its
# free and stewarded capacity so the mesh's aggregate posture is visible, not
# assumed — the operational-weave facing adapter
# (elohim-storage/src/services/operational_weave_facing.rs) sets both gauges from
# the SAME aggregate_capacity fold that backs the WeaveView.cluster_capacity JSON
# projection (one fold, two projections).
#
# Proof signal:
#   elohim_custodian_free_bytes > 0 — cluster-aggregate FREE capacity (0 = none
#     reported); this is basic operational capacity reporting, not a new feature,
#     so it is expected to already be non-zero on a reporting alpha.
#   elohim_custodian_stewarded_bytes >= 0 — cluster-aggregate STEWARDED bytes
#     (custody-blob commitment quantities); this bound is intentionally loose
#     (>= 0, not > 0) because stewarded_bytes needs the RESOLVED agent_cid identity
#     chapter 2 (household-forms) proves — until that lands, 0 is an honest
#     "nothing resolved yet", not a code failure.
#
# Status today: live infrastructure — the free-bytes assertion is expected to
# already hold; the stewarded-bytes assertion is deliberately weak until chapter 2's
# identity-fill cure lands.
@e2e @dataplane @concern:saga-08-capacity-reported
Feature: Chapter 8 — capacity is reported
  A mesh's resilience is only as legible as the capacity its custodians report.
  This chapter proves the cluster-aggregate free and stewarded capacity gauges are
  live and readable — the numbers the resilience card ultimately draws on.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: The cluster reports non-zero free custodian capacity
    Then metric "elohim_custodian_free_bytes" on peer "alpha-A" > 0

  Scenario: The cluster reports a non-negative stewarded-bytes aggregate
    Then metric "elohim_custodian_stewarded_bytes" on peer "alpha-A" >= 0
