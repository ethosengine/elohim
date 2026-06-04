@epr-decomposition @b23 @epr-link @hypercard @browser
Feature: EPR-links flip cards in place, preserving context
  Inside a mounted pillar bundle, clicking an EPR-link to another EPR
  resolves content inline (HyperCard flip) rather than triggering a
  browser navigation. The user keeps their session, scroll, and state.

  @wip @browser-only
  Scenario: EPR-link with display=chip resolves content inline
    Given the user is viewing "/lamad/concept/fair-exchange" in a mounted lamad bundle
    And the concept view contains <elohim-epr-link epr="epr:elohim-host-landing" display="chip">
    When the chip resolves
    Then the chip renders with the landing EPR's title and metadata
    And no browser navigation occurs
    And the lamad Angular app remains mounted

  @wip @browser-only
  Scenario: EPR-link to unreachable target renders the preview, not an error
    Given an EPR-link points to an EPR that is currently unreachable
    And the EPR has a previewEprRef declared
    When the link resolves
    Then the chip renders the preview EPR's content
    And the chip displays the offline/unreachable marker

  @wip @browser-only
  Scenario: EPR-link right-click opens the context menu
    Given the user is viewing a page containing an <elohim-epr-link>
    When the user right-clicks the link
    Then a context menu opens including Open, About this EPR, and Copy EPR link
    And the menu can be navigated by keyboard (arrows, Enter, Escape)
