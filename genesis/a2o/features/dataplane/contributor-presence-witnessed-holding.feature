# Phase 0 of the contributor-presence commons-stewardship re-grounding
# (genesis/docs/superpowers/specs/2026-07-21-contributor-presence-commons-stewardship-design.md).
# The 11 conductor-less fixture residents currently seeded as bare humans rows
# (household_id set, agent_pub_key NULL) are re-grounded as ContributorPresences:
# commons-stewarded, witnessed by real steward agents via
# attestation:witnessed-ascription, counted as WITNESSED (never verified) holders
# in the household-resilience relation. The sibling invariant in
# resilience-identity-coherence.feature (household_id => agent_pub_key) evolves
# here to household_id => (agent_pub_key OR witnessed-presence), which makes the
# orphan shape a detectable defect class again instead of the fixture default.
#
# CI DISCIPLINE: these scenarios are forcing functions for spec Stages 1-2 and
# land PENDING (step definitions in steps/contributor-presence.steps.ts return
# 'pending'), never failed. They un-pend when the seeder bootstrap (Stage 1) and
# the verified-vs-witnessed holder fold (Stage 2) land.
@e2e @dataplane @concern:contributor-presence @wip @act:i
Feature: Fixture residents hold their households as witnessed presences, not bare humans
  A substrate resident without a conductor is a person not yet in the network —
  a ContributorPresence, not a humans row claiming an agent it does not have.
  The witness attests and custodies; the elohim-commons stewards the value in
  trust for the eventual claimant; the household-resilience holder relation
  counts such residents as witnessed holders. Zero rows may remain in the
  ambiguous orphan shape: household-placed with neither an agent key nor a
  backing witnessed presence.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: Every fixture resident exists as a commons-stewarded contributor presence
    # Stage 1 forcing function: the 11 fixture residents are seeded through the
    # existing presences pipeline (presences.json -> seed-presences.ts), state
    # "stewarded" — held by the elohim-commons in trust, claimable later.
    Then every fixture resident on peer "alpha-A" exists as a contributor presence in state "stewarded"
    And no fixture resident on peer "alpha-A" remains a bare human row without a backing presence

  Scenario: Steward-authored witnessed ascriptions cover fixture-resident household residency
    # A real agent authors the ascription ABOUT the presence, signed with their
    # own key — mediated agency, never a synthetic self-attestation. The
    # ascription is the attestation:witnessed-ascription kind (Content-entry
    # convention; zero new DHT entry types).
    Then each fixture-resident presence on peer "alpha-A" carries a steward-authored witnessed ascription for its household residency

  Scenario: The holder relation counts fixture residents as witnessed, never verified
    # Verified = agent_pub_key present (a live key the resilience join can reach).
    # Witnessed = presence-backed, ascribed by a steward. A fixture resident has
    # no key, so counting one as verified would be a false-green on the card.
    Then the household holder relation on peer "alpha-A" counts every fixture resident as a witnessed holder
    And the household holder relation on peer "alpha-A" counts zero fixture residents as verified holders

  Scenario: No household-placed row is orphaned under the evolved invariant
    # The evolved invariant: household_id => (agent_pub_key OR witnessed-presence).
    # With fixture residents re-grounded as presences, a row with neither is once
    # again exactly one bug — the membership-projection agent-key drop — and
    # never the fixture default. Zero matching rows on a peer with no households
    # is an honest pass; the invariant is an implication.
    Then no household-placed human on peer "alpha-A" lacks both an agent key and a witnessed presence
