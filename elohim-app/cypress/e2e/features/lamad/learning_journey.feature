Feature: Lamad Learning Journey
  As a learner (Traveler)
  I want to follow a curated path through the Elohim Protocol
  So that I can build understanding systematically without overwhelm

  Background:
    Given I am a new traveler "Alice"
    And the "Elohim Protocol" path exists

  Scenario: Starting a Journey
    When I start the "Elohim Protocol" path
    Then I should be on step 0 "The Manifesto"
    And the content for "The Manifesto" should be visible
    But step 1 "Core Concepts" should be visible as a preview
    And step 5 "Advanced Governance" should be hidden (Fog of War)

  Scenario: Earning Affinity through Navigation
    Given I am on step 0 "The Manifesto"
    When I read the content
    Then my affinity for "The Manifesto" should increase
    And my progress on "Elohim Protocol" should update

  Scenario: Restricted Access (Attestations)
    Given step 3 "Deep Dive" requires the "Basic Understanding" attestation
    And I do not have the "Basic Understanding" attestation
    When I try to access step 3
    Then I should see a "Locked" message
    And I should be guided to earn "Basic Understanding"
