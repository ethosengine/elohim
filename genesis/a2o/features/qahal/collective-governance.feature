Feature: Collective governance
  As a member of a small group
  I want to propose and vote on group decisions
  So that our group self-governs through consent

  Background:
    Given I am "Matthew" in the "Valley Bible Study" collective

  Scenario: Create a proposal
    When I create a proposal titled "Study Romans next quarter"
    With type "sense-check"
    And description "Romans provides foundational theology for our group's next season"
    Then the proposal appears in the collective's proposals tab
    And the proposal status is "voting"

  Scenario: Vote on a proposal
    Given a proposal "Study Romans next quarter" exists in my collective
    When I vote "agree" on the proposal
    Then my vote is recorded
    And the vote count updates

  Scenario: Block a proposal with justification
    Given a proposal "Study Romans next quarter" exists in my collective
    When I vote "block" on the proposal
    Then I must provide a written reason
    And the block is visible to other members

  Scenario: Anonymous voting
    Given a proposal with anonymous voting enabled
    When members vote on the proposal
    Then vote counts are visible
    But individual voters are not identified

  Scenario: Change a vote
    Given I have voted "agree" on a proposal
    When I change my vote to "disagree"
    Then my previous vote is replaced
    And the vote counts update accordingly
