@epr-decomposition @b23 @epr-link @hypercard @browser @requires:doorway
Feature: EPR-links flip cards in place, preserving context
  Inside a mounted pillar bundle, clicking an EPR-link to another EPR
  resolves content inline (HyperCard flip) rather than triggering a
  browser navigation. The user keeps their session, scroll, and state.

  The home surface ("/") renders a real <elohim-epr-link epr="epr:manifesto">
  via the call-to-action component — the one place in the app today where the
  Lit primitive (not the markdown <a data-epr> shim) is mounted. The chip
  resolves against the seeded "manifesto" content node (reach=commons), so
  these scenarios run against live seeded data, not a fixture page.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" with device

  @browser-only
  Scenario: EPR-link with display=card resolves content inline
    Given the user is viewing "/" in a mounted pillar bundle
    And the view contains <elohim-epr-link epr="epr:manifesto" display="card">
    When the chip resolves
    Then the chip renders with the resolved EPR's title and metadata
    And no browser navigation occurs
    And the Angular app remains mounted

  @browser-only
  Scenario: EPR-link to unreachable target renders the preview, not an error
    Given an EPR-link points to an EPR that is currently unreachable
    And the EPR has a previewEprRef declared
    When the link resolves
    Then the chip renders the preview EPR's content
    And the chip displays the offline/unreachable marker

  @browser-only
  Scenario: EPR-link right-click opens the context menu
    Given the user is viewing a page containing an <elohim-epr-link>
    When the user right-clicks the link
    Then a context menu opens including Open, About this EPR, and Copy EPR link
    And the menu can be navigated by keyboard (arrows, Enter, Escape)
