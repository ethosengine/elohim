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

  Scenario: Learner sees context menu only on constitutional content
    Given content "protocol-foundations" has governance state "constitutional"
    When a learner views the content
    Then the feedback gateway renders at level 0
    And only the context menu is visible with flag, challenge, and open feedback options
    And no emotional reactions or graduated feedback are shown

  Scenario: Learner reacts to learning content with emotional response
    Given content "intro-to-ecology" has governance state "active"
    And no active proposals exist for the content
    When a learner views the content
    Then the feedback gateway renders at level 1
    And emotional reactions are available
    When the learner selects the "inspired" reaction
    Then a governance signal is recorded with type "reaction" and value "inspired"

  Scenario: Learner provides graduated feedback on discussion content
    Given content "climate-adaptation-strategies" has governance state "active"
    And the content type is "discussion"
    When a learner views the content
    Then the feedback gateway renders at level 2
    And a graduated feedback scale is shown
    When the learner rates accuracy as "mostly-inaccurate"
    Then reasoning text is required before submission
    And the signal is recorded with type "graduated" and value "accuracy:mostly-inaccurate"

  Scenario: Formal governance ballot renders via Psephos for ranked-choice proposal
    Given content "history-curriculum-options" has an active proposal
    And the proposal uses "ranked-choice" voting mechanism
    When a learner views the content
    Then the feedback gateway routes to the Psephos ballot renderer
    And the ballot displays options with election hygiene applied

  Scenario: Governance participation generates stewardship recognition
    Given a learner has submitted a vote on a proposal
    Then an REA economic event is generated with type "governance-participation"
    And the learner's steward affinity increases proportional to mechanism level

  Scenario: Learner challenges inaccurate content
    Given content "intro-to-photosynthesis" has governance state "active"
    When a learner files a challenge with grounds "factual-error"
    And provides evidence "The diagram shows CO2 being released, but photosynthesis absorbs CO2"
    And their standing is "community-member"
    Then the challenge is recorded with state "pending"
    And a response deadline is set 3 days from filing

  Scenario: Challenge list shows SLA countdown
    Given content "intro-to-photosynthesis" has a pending challenge
    And the response deadline is in 2 days
    When a learner views the challenge list
    Then the SLA indicator shows "on_time" in green
    And displays "Response due in 2d"

  Scenario: Elohim responds to challenge within SLA
    Given a pending challenge exists for content "intro-to-photosynthesis"
    When the elohim responds with outcome "upheld"
    And provides reasoning "The diagram was indeed incorrect — CO2 is absorbed during photosynthesis"
    And marks the response as precedent-setting
    Then the challenge state changes to "responded"
    And the SLA status shows "resolved"

  Scenario: Learner appeals rejected challenge
    Given a challenge was rejected with reasoning "The content is metaphorical, not literal"
    When the challenger files an appeal with grounds "disproportionate-response"
    And provides additional evidence "Students are being tested on this content literally"
    Then the appeal is recorded and escalates to the next governance level

  Scenario: SLA overdue triggers visual warning
    Given a challenge has been pending for 4 days
    And the response deadline was 3 days after filing
    When the challenge list is viewed
    Then the SLA indicator shows "overdue" in red
    And displays "OVERDUE by 1d"

  Scenario: Challenge response sets referenceable precedent
    Given a challenge response was marked as precedent-setting
    When another similar challenge is filed
    Then the precedent is visible in the challenge detail
    And the elohim can reference it in their response
