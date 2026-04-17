Feature: EPR relationship navigation boxes
  Learners see the typed relationships declared in a concept's EPR Head
  as navigable cards with trust signals (reach + stewardship resilience).
  Relationships are grouped by type so prerequisites, sub-topics, and
  references are legible at a glance.

  Background:
    Given a learner has signed in
    And the concept "systems-thinking" has been seeded with an EPR Head
      | type         | target                    |
      | PREREQUISITE | feedback-loops            |
      | TEACHES      | mental-models             |
      | REFERENCES   | donella-meadows-essay     |

  Scenario: Learner sees typed relationship cards beneath a concept
    When the learner opens the "systems-thinking" concept
    Then a "Prerequisites" section is visible
    And it contains a card linking to "feedback-loops"
    And the card shows a reach badge and a resilience badge
    And a "This teaches" section is visible
    And a "References" section is visible

  Scenario: Clicking a relationship card navigates to the target concept
    Given the learner is viewing the "systems-thinking" concept
    When the learner clicks the "feedback-loops" prerequisite card
    Then the "feedback-loops" concept is displayed
    And the URL reflects the "feedback-loops" concept
