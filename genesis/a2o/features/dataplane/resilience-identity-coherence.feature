# Landed with the on_membership_projected identity-coherence stamp
# (elohim-storage/src/reconcile/controller.rs). Unit-anchored in elohim-storage
# (reconcile::controller::tests::n1d_membership_projection_stamps_null_agent_pub_key
# through n1h_non_agent_cid_member_key_skips_agent_pub_key_stamp_and_is_observed —
# NULL-only, HOUSEHOLD-gated, matched by humans::id so it never matches on the
# column it writes, namespace-guarded via identity_namespace::is_agent_cid so a
# transport id is observed rather than written). This feature pins the
# LIVE-OBSERVABLE consequence of that unit-level guard: a human already placed in
# a household (household_id set) must never be left without the agent_pub_key the
# household-resilience join depends on — see elohim-storage/CLAUDE.md "Identity &
# Transport-Identity Coherence" and
# genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md
# §3.4 "stopgap".
@e2e @dataplane @concern:identity-coherence @regression
Feature: Membership projection stamps the member's agent key — household resilience joins light
  Before this guard, on_membership_projected stamped humans.household_id from a DHT
  MembershipProjected signal but dropped the agent_pub_key already carried in the
  same signal (member_cid). A human left with household_id set and agent_pub_key
  NULL is exactly the shape that silently empties the household-resilience
  snapshot's `humans.agent_pub_key = peer_id` join — the documented all-zeros
  resilience card. This scenario pins the invariant every federation peer must
  hold: no human that has been placed in a household is ever left without the
  agent key that join needs.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: No household-placed human on alpha-A is missing its agent_pub_key
    # If this ever regresses, it is exactly the all-zeros-resilience-card bug:
    # a human's household_id got stamped while its agent_pub_key stayed NULL, so
    # the household-resilience snapshot's join silently drops that human's
    # devices/peer_statuses row instead of lighting the card. Zero household-
    # placed humans on the peer is an honest pass, not a fabricated one — the
    # invariant is an implication (household_id set => agent_pub_key set).
    Then no HOUSEHOLD-member human on peer "alpha-A" is missing its agentPubKey
