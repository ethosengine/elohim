@e2e @shefa @governance @limitarian-governor @requires:doorway @requires:seeded-content
Feature: A community ratifies the limit it cannot set for itself
  The attention economy's externality is that every participant benefits locally
  from concentration — so the limit must be supplied by the layer that
  internalizes the harm, carried as a governed EPR (witnessed, immutable,
  renewable), and enforced as smooth relational friction with a dignity floor.
  Spec: per-substrate-limitarian-governor-design (v1 slice).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @wip
  Scenario: An out-of-wall gradient cannot be ratified
    # DNA wall validator (spec §5.2): reject-at-write — a config that exists is
    # in-wall by construction. @wip until a zome-call probe step exists; the
    # contract is pinned native-side by validate_ratifies_limit_gradient tests.
    When a steward proposes a limit-gradient with concentration target "0.9"
    Then the commitment is rejected naming the DNA wall

  @wip
  Scenario: A passed ratification writes the governed limit
    # The dead seam (spec §1), closed: propose → M-of-N vote → tally pass →
    # the responsibility-demand config row carries ratified_by/ratified_at/
    # dht_anchor_hash. @wip until the governance-action proposal step for the
    # ratify-limit-gradient kind is wired into the step library.
    Given a community governance action "ratify-limit-gradient" with an in-wall gradient
    When the action passes its approval tally
    Then the responsibility demand config for "community" shows a ratification anchor

  @wip
  Scenario: Concentration friction relaxes only at the governed target
    # The governor extinguishes at C_target (a deliberate setpoint), never at
    # median's drifting attractor; below the dignity floor decay is OFF.
    # Native proof: continuous_governor_restores_toward_target_under_rich_get_richer_inflow.
    Given a community whose attention substrate is concentrated above its target
    When the demurrage tick applies the ratified gradient
    Then holders above the mean experience super-linear friction
    And holders below the dignity floor experience none
