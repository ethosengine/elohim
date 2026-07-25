# Chapter 2 of the resiliency-saga: matthew's device discovers jessica and james as
# household members. The periodic identity-fill sweep
# (elohim-storage/src/services/identity_fill.rs) discovers the household's collective
# cid on the DHT and fills missing humans.agent_pub_key rows from membership truth —
# the household FORMS as a legible entity. Before this sweep completes, agent_pub_key
# is NULL and every downstream custody/resilience join silently empties (the
# documented all-zeros-resilience-card root cause; see
# feedback_household_nodes_is_the_stable_floor and the identity-coherence dataplane
# concern in ../resilience-identity-coherence.feature).
#
# Proof signal:
#   elohim_identity_fill_discovered_cids >= 1 — a household collective cid was found
#     on the last sweep (a plain, unlabeled IntGauge — the bare "metric ... on peer"
#     step applies directly).
#   elohim_identity_fill_total{action="created"} >= 1 — a member row was actually
#     WRITTEN from that discovery (an IntCounterVec; the bare-name step would collapse
#     across labels and read whichever action happened to scrape first, so this uses
#     the label-aware step to pin action="created" specifically).
#
# Status today: RED — the periodic sweep exists and is unit-tested, but the ceremony
# that drives it to completion on alpha has not yet deployed. Born red; do not weaken.
@e2e @dataplane @concern:saga-02-household-forms
Feature: Chapter 2 — the household forms
  matthew's household (jessica, james) must be discoverable and legible before any
  co-stewardship agreement can be witnessed: identity-fill finds their household
  collective cid on the DHT and fills the missing agent_pub_key rows. Until this
  sweep completes on alpha, the household is invisible to every resilience join
  downstream — this chapter is born red until that ceremony cure deploys.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: Identity-fill discovers the household and writes its member rows
    Then metric "elohim_identity_fill_discovered_cids" on peer "alpha-A" >= 1
    And labeled metric "elohim_identity_fill_total" with label "action" "created" on peer "alpha-A" >= 1
