@e2e @qahal @browser-only @feedback-gate @regression
Feature: Feedback dialogue panel — accountable peer surface
  As a learner viewing protocol content,
  I want to flag, challenge, share feedback, or report issues from the content view,
  so that the elohim's decisions and the content's standing remain accountable to me.

  This feature operationalizes P4 from
  genesis/docs/superpowers/specs/2026-04-19-gate-challenge-and-indemnification-design.md
  — accountability is an architectural property surfaced through the dialogue panel,
  not a post-hoc audit mechanism. The panel must be reachable from any content view,
  visually above all page chrome (TOC sidebars, omnibars, focused-view overlays),
  and the artifact card must be selectable for the learner to articulate the grievance.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Terrance" is logged in on doorway "alpha" with device

  @happy-path @reach
  Scenario: Learner reaches the feedback dialogue from a content page
    When Terrance navigates to a markdown content page
    Then the feedback trigger should be visible in the content actions
    And the feedback trigger should have an accessible label

  @happy-path @menu
  Scenario: Trigger reveals the four governance feedback choices
    Given Terrance is viewing a markdown content page
    When Terrance opens the feedback trigger menu
    Then the feedback menu should list "Flag"
    And the feedback menu should list "Challenge"
    And the feedback menu should list "Feedback"
    And the feedback menu should list "Report Issue"

  @happy-path @modal-stack
  Scenario: Selecting a category opens a readable, selectable card above page chrome
    Given Terrance is viewing a markdown content page
    And Terrance has opened the feedback trigger menu
    When Terrance selects "Feedback" from the menu
    Then the feedback dialogue panel should be visible
    And the feedback dialogue panel should stack above any table-of-contents sidebar
    And the artifact textarea should be selectable and focusable
    And the artifact textarea placeholder should be "Share your thoughts..."

  @category-vocabulary
  Scenario Outline: Each category renders the spec's grievance vocabulary
    Given Terrance is viewing a markdown content page
    And Terrance has opened the feedback trigger menu
    When Terrance selects "<menu-label>" from the menu
    Then the feedback dialogue panel title should read "<panel-title>"
    And the artifact textarea placeholder should be "<placeholder>"

    Examples:
      | menu-label   | panel-title       | placeholder            |
      | Flag         | Flag Content      | Describe the issue...  |
      | Challenge    | Challenge Content | State your case...     |
      | Feedback     | Share Feedback    | Share your thoughts... |
      | Report Issue | Report Issue      | What happened?         |

  @dismiss
  Scenario: Learner can close the dialogue without submitting
    Given Terrance has the feedback dialogue panel open with "Feedback" selected
    When Terrance clicks the dialogue close button
    Then the feedback dialogue panel should not be visible

  @dismiss @backdrop
  Scenario: Clicking outside the card dismisses the dialogue
    Given Terrance has the feedback dialogue panel open with "Feedback" selected
    When Terrance clicks the dialogue backdrop
    Then the feedback dialogue panel should not be visible

  @dismiss @keyboard
  Scenario: Pressing Escape dismisses the dialogue
    Given Terrance has the feedback dialogue panel open with "Feedback" selected
    When Terrance presses the Escape key
    Then the feedback dialogue panel should not be visible
