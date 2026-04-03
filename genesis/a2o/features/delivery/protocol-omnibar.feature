@delivery @omnibar @provenance
Feature: Full-Browser Content Delivery with Protocol Omnibar
  As a visitor to the Elohim Protocol
  I want content delivered as full pages with an unobtrusive provenance bar
  So I can tell this came from the protocol network and drill into its governance

  The protocol omnibar is the equivalent of a browser's address bar with SSL
  padlock. At a glance it says "you're on the network." Click to inspect the
  EPR provenance — like viewing a site's SSL certificate. Drill down to the
  full governance hub. Initiate feedback or report actions.

  Background:
    Given content node "manifesto" exists with:
      | title       | The Elohim Protocol Manifesto           |
      | format      | markdown                                |
      | reach       | commons                                 |
      | stewardedBy | Genesis Collective (80%), Matthew (20%) |

  # --- Full-Page Delivery ---

  Scenario: Markdown content renders as full page
    When I visit "/deliver/manifesto"
    Then the page renders the manifesto as formatted HTML
    And there is no Angular navigation chrome
    And the protocol omnibar pill is visible in the corner

  Scenario: HTML5 app content renders as full-page iframe
    Given content node "evolution-of-trust" exists with format "html5-app"
    When I visit "/deliver/evolution-of-trust"
    Then the page shows the app in a full-viewport iframe
    And the protocol omnibar pill overlays the iframe

  Scenario: Unknown content shows 404
    When I visit "/deliver/nonexistent-slug"
    Then I see a "Content not found" page

  # --- Omnibar: At-a-Glance (Collapsed Pill) ---

  Scenario: Omnibar pill shows protocol-delivered status
    When I visit "/deliver/manifesto"
    Then a small pill appears in the top-right corner
    And the pill shows the reach icon and "E" protocol mark
    And the pill does not distract from the content

  # --- Omnibar: Expanded (Inspect Provenance) ---

  Scenario: Clicking the pill expands provenance details
    When I visit "/deliver/manifesto"
    And I click the omnibar pill
    Then it expands to show the EPR content address
    And it shows the reach level "commons"
    And it shows the stewards "Genesis Collective" and "Matthew"
    And it shows the delivery source "alpha.elohim.host"

  Scenario: EPR address is copyable
    Given the omnibar is expanded
    When I click the copy button next to the EPR address
    Then the full content address is copied to clipboard

  Scenario: Collapsing the expanded omnibar
    Given the omnibar is expanded
    When I click the collapse button
    Then it returns to the minimal pill

  # --- Omnibar: Drill-Down (Navigate to Governance Hub) ---

  Scenario: Inspect EPR navigates to governance hub
    Given the omnibar is expanded
    When I click "Inspect" (or the EPR address link)
    Then I navigate to "/resource/manifesto"
    And I see the full content viewer with Attestations, Governance, and Network tabs

  # --- Omnibar: Actions ---

  Scenario: Report action available from omnibar
    Given the omnibar is expanded
    When I click the actions menu
    Then I see options: "Report content", "Give feedback", "View stewards"

  # --- Doorway Headers ---

  Scenario: Doorway response includes provenance headers
    When I request "/deliver/manifesto"
    Then the HTTP response includes header "X-Content-Address"
    And the HTTP response includes header "X-Reach" with value "commons"

  # --- Focused View Integration ---

  Scenario: Content viewer focused mode shows omnibar pill
    Given a learner is viewing content "manifesto" in the learning app
    When they toggle focused view mode
    Then the omnibar pill appears in the top-right corner
    And the tabs and feedback sections are hidden
