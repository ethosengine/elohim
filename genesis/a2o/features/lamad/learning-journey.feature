@e2e @lamad @browser-only
Feature: Lamad Learning Journey
  As a learner (Traveler)
  I want to follow a curated path through the Elohim Protocol
  So that I can build understanding systematically without overwhelm

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" with device
    And the "Elohim Protocol" path exists

  # @wip 2026-06-04 shakeout: path-navigator does not render .step-list-item[data-index]
  # (structural — step list UI never wired); un-wip when the navigator family lands.
  @wip
  Scenario: Starting a Journey
    When I start the "Elohim Protocol" path
    Then I should be on step 0 "The Manifesto"
    And the content for "The Manifesto" should be visible
    But step 1 "Core Concepts" should be visible as a preview
    And step 5 "Advanced Governance" should be hidden (Fog of War)

  # @wip 2026-06-04 shakeout: AffinityCircleComponent is not imported/rendered by
  # path-navigator (structural — planned component per lamad CLAUDE.md); the step
  # waited 30s on a selector that cannot exist. Un-wip when affinity-circle is wired.
  @wip
  Scenario: Earning Affinity through Navigation
    Given I am on step 0 "The Manifesto"
    When I read the content
    Then my affinity for "The Manifesto" should increase
    And my progress on "Elohim Protocol" should update

  # @wip 2026-06-04 shakeout: attestation-gate setup steps are empty stubs AND
  # path-navigator has no locked-state UI branch (verified template read) — the
  # scenario specs an unbuilt feature. Un-wip when the attestation gate lands.
  @wip
  Scenario: Restricted Access (Attestations)
    Given step 3 "Deep Dive" requires the "Basic Understanding" attestation
    And I do not have the "Basic Understanding" attestation
    When I try to access step 3
    Then I should see a "Locked" message
    And I should be guided to earn "Basic Understanding"
