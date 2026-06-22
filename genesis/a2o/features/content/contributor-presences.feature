@e2e @content @browser-only @requires:doorway @requires:seeded-content @requires:seeded-contributors
Feature: Contributor presences on a content artifact
  When a learner opens a piece of content, they can see who inspired or contributed to it.
  Recognition arrives before registration: a contributor presence is established the moment
  someone's work is cited in the content graph — no account required to be seen,
  no gate before gratitude. The "Contributors" section makes that recognition visible.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a learner has signed in
    And content "systems-thinking-intro" has been seeded with contributor presences
      | presenceId       | displayName        |
      | presence-donella | Donella Meadows    |
      | presence-peter   | Peter Senge        |

  Scenario: Learner sees contributor presences below the content
    When the learner opens the content "systems-thinking-intro"
    Then the contributors section is visible
    And the contributors list shows 2 contributor cards
    And there is a contributor card for "Donella Meadows"
    And there is a contributor card for "Peter Senge"

  Scenario: Content without contributors shows no contributors section
    Given content "no-contributors-node" has been seeded with no contributor presences
    When the learner opens the content "no-contributors-node"
    Then the contributors section is absent
