# Test Governance Scenario

**Type:** scenario

**Tags:** governance, test, decision-making

**Reach:** commons

Feature: Community Decision Making
  As a community member
  I want to participate in governance decisions
  So that I can have a voice in the protocol's direction

  Scenario: Proposing a change
    Given I am a verified community member
    When I submit a governance proposal
    Then the proposal should be visible to all members
    And I should receive acknowledgment of submission

  Scenario: Voting on a proposal
    Given there is an active governance proposal
    When I cast my vote
    Then my vote should be recorded
    And I should see updated vote tallies
