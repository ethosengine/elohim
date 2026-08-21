@e2e @content @stewardship @act:i
Feature: Content Stewardship Allocation
  As a content ecosystem,
  stewardship should be distributed by affinity
  so that content is tended by those with natural connection to it.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    # Jessica reads as a member of the commons. She is here because the
    # stewarded corpus is reach:community — a passer-by with no membership can
    # read the commons slice, but not the body of seeded content these scenarios
    # make claims about. The same query returns 5 value-scanner items to an
    # anonymous reader and 1870 to Jessica, so without her these scenarios judge
    # the affinity graph from a handful of items that were never in it.
    #
    # Jessica specifically, for two reasons. She is AUTHENTICATED and not an
    # admin, so membership — not privilege — is what lifts the reach gate, and a
    # future regression that let reach leak to admins only would still fail here.
    # And she is a household persona, so this feature does not become pending
    # whenever the remote pool is down; stewardship allocation has nothing to do
    # with remote compute and should not inherit its outages.
    And human "Jessica" is logged in on doorway "alpha"
    And content has been seeded with affinity-based stewardship allocations

  # ============================================================================
  # Allocation Distribution
  # ============================================================================

  @distribution
  Scenario: Value-scanner content has multiple stewards
    When I query stewardship allocations for value-scanner content
    Then Adam should be listed as a steward with the highest ratio
    # Jessica, not Susan: the seeder's CATEGORY_STEWARD_MAP allocates
    # jessica-spouse for value-scanner; asserting Susan could never pass
    # (she has no allocation and no presence mapping). If Susan was the
    # original intent, that is a seed-map change with allocation-supersede
    # implications — tracked, not assumed here.
    And Jessica should be listed as a steward
    And Matthew should be listed as a steward
    And no single steward should have 100% allocation ratio

  @affinity
  Scenario: Stewardship reflects human affinities
    When I query stewardship allocations for public-observer content
    Then Eve should have the highest allocation ratio
    And her allocation method should be "computed"
    And her contribution type should be "curator"

  @affinity
  Scenario: Faith content stewarded by pastoral affinity
    When I query stewardship allocations for fct content
    Then Pastor Pete should be listed as a steward
    And Pete's allocation ratio should be approximately 0.50

  # ============================================================================
  # Allocation Integrity
  # ============================================================================

  @integrity
  Scenario: Allocation ratios sum to approximately 1.0
    When I query stewardship allocations for any content category
    Then the sum of allocation ratios for each content item should be approximately 1.0

  @fallback
  Scenario: Uncategorized content falls back to bootstrap steward
    When I query stewardship allocations for content with no matching category
    Then Matthew should be the sole steward with ratio 1.0
    And the allocation method should be "computed"
    And the contribution type should be "curator"

  # ============================================================================
  # No one is a creator
  # ============================================================================

  @philosophy
  Scenario: No steward has exclusive ownership
    Given content has been seeded with affinity-based allocations
    When I query all stewardship allocations
    Then the average number of stewards per content item should be greater than 1
    And the manifesto principle holds: content is stewarded, not owned
