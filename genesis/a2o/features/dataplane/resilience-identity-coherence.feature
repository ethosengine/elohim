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
@e2e @dataplane @concern:identity-coherence @regression @act:i
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
    # invariant is an implication (household_id set => agent_pub_key set),
    # SCOPED to households this peer can actually observe: a narrative/fixture
    # household with zero conductors on this mesh has no truthful key source at
    # all (seed-humans deliberately writes agent_pub_key NULL for it), so it
    # sits out of scope entirely rather than counting as an offender — mirrors
    # the sibling fossil-check scenario's "observable" scoping below.
    Then no HOUSEHOLD-member human on peer "alpha-A" is missing its agentPubKey

  # Landed with the boot-time membership-truth key-supersede pass
  # (elohim-storage/src/services/membership_identity_reconcile.rs, commit
  # 9378519cb). Unit-anchored by membership_identity_reconcile::tests::
  # single_fossil_with_one_current_member_supersedes (the realistic post-signal
  # shape: one member already on its live key, one lone orphan fossil pairs
  # unambiguously to the one unmatched live key),
  # ambiguous_multi_fossil_household_is_skipped (≥2 orphans / ≥2 unmatched keys
  # is a forced abstention — mis-pairing would attribute one human's identity
  # AND shards to another, so the pass logs + skips rather than guesses), and
  # fossil_household_supersede_cascade_lights_the_card (the supersede cascades
  # to shard_locations.peer_id and rea_commitments.provider in the SAME
  # transaction, so the resilience-card holder/commitment joins re-light
  # together instead of realigning one side while stranding the other). This
  # scenario pins the LIVE-OBSERVABLE consequence: a household-placed human's
  # agent_pub_key must never be left pointing at a dead (fossil) key once the
  # DHT membership truth for that household is observable on this peer — a
  # lone resolvable fossil surviving past boot is exactly the regression this
  # reconcile pass exists to prevent.
  Scenario: No household-member human on alpha-A carries a fossil agentPubKey
    # A fossil survives only when the boot-time reconcile pass never ran, or
    # ran but found a genuinely ambiguous (≥2-orphan) household — the
    # documented abstention, which converges as more members get re-projected
    # on later deploys. A household with NO live membership key observable at
    # all (e.g. this peer isn't hosting any of its members yet) is out of
    # scope for this check — the invariant only applies where live membership
    # truth is actually visible on this peer. Zero observable households, or
    # every observable household's members all matching their live keys, is
    # an honest pass; a household with exactly ONE member's key unresolved
    # against an otherwise-observable live set is the FAIL this scenario
    # exists to catch — that is precisely the forced-bijection case
    # single_fossil_with_one_current_member_supersedes covers, and it must
    # never survive a boot past the reconcile pass.
    Then every observable HOUSEHOLD-member human on peer "alpha-A" has a non-fossil agentPubKey
