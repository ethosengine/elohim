@e2e @content @stewardship
Feature: Content Stewardship Allocation
  As a content ecosystem,
  stewardship should be distributed by affinity
  so that content is tended by those with natural connection to it.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And content has been seeded with affinity-based stewardship allocations

  # ============================================================================
  # Allocation Distribution
  # ============================================================================

  @distribution
  Scenario: Value-scanner content has multiple stewards
    When I query stewardship allocations for value-scanner content
    Then Adam should be listed as a steward with the highest ratio
    And Susan should be listed as a steward
    And Matthew should be listed as a steward
    And no single steward should have 100% allocation ratio

  @affinity
  Scenario: Stewardship reflects human affinities
    When I query stewardship allocations for public-observer content
    Then Eve should have the highest allocation ratio
    And her allocation method should be "affinity"
    And her contribution type should be "steward"

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
    And the allocation method should be "affinity"
    And the contribution type should be "steward"

  # ============================================================================
  # No one is a creator
  # ============================================================================

  @philosophy
  Scenario: No steward has exclusive ownership
    Given content has been seeded with affinity-based allocations
    When I query all stewardship allocations
    Then the average number of stewards per content item should be greater than 1
    And the manifesto principle holds: content is stewarded, not owned
