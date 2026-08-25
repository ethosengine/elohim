@e2e @epr-decomposition @b23 @epr-link @hypercard @browser @requires:doorway @act:i
Feature: EPR-links flip cards in place, preserving context
  An EPR (Elohim Protocol Reference) is the protocol's content-addressed name
  for a piece of content: "epr:manifesto" names the Manifesto wherever it is
  stored. An EPR-link is the element that renders such a reference; the
  steps below call the rendered element a card or a chip interchangeably —
  one element, one behaviour, display="card" is its preview size. The card
  RESOLVES in place: on mount it fetches the referenced content's title and
  metadata and shows them without leaving the page (a HyperCard flip, not a
  browser navigation), so the reader keeps their session, scroll, and state.
  FOLLOWING the card — clicking it — is the second act: it opens the content
  at its universal address, the "/epr/{id}" path every doorway serves for any
  EPR.

  A pillar bundle is one of the app's independently served front-ends;
  "mounted" means it has finished loading at the given path. The home surface
  ("/") renders a real <elohim-epr-link epr="epr:manifesto"> via the
  call-to-action component — the place where the Lit primitive (not the
  markdown <a data-epr> shim) is mounted. The card resolves against the seeded
  "manifesto" content node (reach=commons, readable by anyone), so these
  scenarios run against live seeded data, not a fixture page.

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

  # Regression (2026-08-25): the card RESOLVED fine (the scenario above stayed
  # green) but FOLLOWING it stranded the reader on "Loading content..." at
  # /epr/manifesto — the routed content viewer never re-rendered after the node
  # arrived (Angular 22 implicit-OnPush; backlog-onpush-eager-debt-inventory).
  # No scenario had ever followed the card, so nothing in CI could see it.
  @browser-only @regression
  Scenario: Following the card opens the content at its universal address, never stalling on a loading indicator
    Resolution (the scenario above) can succeed while following stalls: the
    card shows the Manifesto's title, the click lands on /epr/manifesto, and
    the page never leaves "Loading content...". This scenario guards the
    second act on its own.

    Given the user is viewing "/" in a mounted pillar bundle
    And the view contains <elohim-epr-link epr="epr:manifesto" display="card">
    When the user follows the card
    Then the universal address "/epr/manifesto" is showing
    And the resolved EPR's body is rendered, not a loading state

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
