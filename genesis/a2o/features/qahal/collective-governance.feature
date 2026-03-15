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

  Scenario: Community uses ranked-choice to pick a curriculum path
    Given a collective "homeschool-coop" has an active proposal "Which history curriculum?"
    And the proposal uses "ranked-choice" voting with 3 options
      | option            |
      | Story of the World |
      | History Odyssey    |
      | Classical Conversations |
    When member "sarah" ranks her preferences
      | rank | option            |
      | 1    | History Odyssey    |
      | 2    | Story of the World |
      | 3    | Classical Conversations |
    And member "james" ranks his preferences
      | rank | option            |
      | 1    | Story of the World |
      | 2    | History Odyssey    |
      | 3    | Classical Conversations |
    Then the tally shows round-by-round elimination results
    And the winning option is displayed with the elohim's justification

  Scenario: Stewards score competing content revisions
    Given content "intro-to-fractions" has 2 proposed revisions
    And the proposal uses "score-vote" voting with range 1 to 10
    When 3 stewards score each revision independently
    Then the revision with the highest total score is recommended
    And each steward's reasoning is visible to the others

  Scenario: Dot-voting allocates limited attention across proposals
    Given a collective has 5 pending proposals
    And each member gets 10 dots to distribute
    When member "maria" allocates dots across proposals
      | dots | proposal                    |
      | 5    | Add music theory path       |
      | 3    | Update science curriculum   |
      | 2    | Create art history module   |
    Then the proposals are ranked by total dots received
    And proposals with zero dots are deprioritized

  Scenario: Consent round with escalation on block
    Given a proposal "Restructure reading groups" is in consent round
    When all members consent except "david" who blocks
    And "david" provides justification "This eliminates the only group for struggling readers"
    Then the block triggers an elohim-facilitated conversation
    And the elohim engages with david's concern before proceeding
    And the block is recorded in the settlement log regardless of outcome

  Scenario: Elohim selects feedback mechanism based on content context
    Given content "manifesto-foundations" has governance state "constitutional"
    When a learner views the content
    Then only reasoned dissent is available via the context menu
    And no low-friction reactions are shown
    But the learner can still flag, challenge, or provide open feedback
