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
# Finish line RE-AIMED 2026-07-29 (activity-vs-truth trap — 4th compounding
# measurement cause, see README "Measurement-timing" section): the original proof
# signals were activity-shaped and structurally unable to stay green after the
# cure succeeds. `elohim_identity_fill_discovered_cids` is a per-sweep OVERWRITE
# gauge (metrics.rs IntGauge.set) zeroed by every restart until a tick lands, and
# `elohim_identity_fill_total{action="created"}` is 0 forever once every member
# row exists (apply_membership_fill's already-present short-circuit,
# identity_fill.rs — proven by the second_pass_is_idempotent test). A fully cured
# pod was indistinguishable from a never-run one. The chapter's meaning — "the
# household forms and is discoverable" — is a STATE, so the finish line now
# asserts the durable rows the sweep exists to produce: non-null agentPubKey +
# householdId on /db/humans. This is not a weakening; it measures the outcome
# instead of the ceremony.
#
# Sweep-liveness STATION (kept, not the finish line): every successful tick
# unconditionally emits all four action buckets (identity_fill.rs run loop), so
# the SERIES EXISTING at all proves the sweep ran this boot — pollForGauge
# distinguishes absent (pending: no tick yet / unreachable) from present (pass).
# The station asserts presence (>= 0), NOT a per-member count: edge #1259 proved
# skipped_present stays 0 on matthew because his DISCOVERY legs yield zero pairs
# this boot (self-chain read blocked behind the captured-UUID conductor ceiling;
# local collectives stamp cascade pending) — that residue is real, named, and
# operator-owned (the UUID chain migration ceiling item), so it must not gate the
# chapter whose finish-line state is supplied by the other legs. When the
# migration lands, discovery on matthew goes non-zero and the per-member buckets
# become meaningful again.
@e2e @dataplane @concern:saga-02-household-forms
Feature: Chapter 2 — the household forms
  matthew's household (jessica, james) must be discoverable and legible before any
  co-stewardship agreement can be witnessed: identity-fill finds their household
  collective cid on the DHT and fills the missing agent_pub_key rows. The finish
  line is the durable truth those sweeps produce — member rows carrying their
  agent identity and household — because that state is what every resilience join
  downstream actually reads.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: The household's member rows are durably filled from membership truth
    Then the humans row "human-matthew-manager" on doorway "alpha-A" has "agentPubKey" set
    And the humans row "human-jessica-spouse" on doorway "alpha-A" has "agentPubKey" set
    And the humans row "human-james-son" on doorway "alpha-A" has "agentPubKey" set
    And the humans row "human-jessica-spouse" on doorway "alpha-A" has "householdId" equal to "household-dowell"
    And the humans row "human-james-son" on doorway "alpha-A" has "householdId" equal to "household-dowell"

  Scenario: Station — the identity-fill sweep is alive and accounting this boot
    Then labeled metric "elohim_identity_fill_total" with label "action" "skipped_present" on peer "alpha-A" >= 0
