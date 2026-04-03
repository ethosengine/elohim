@delivery @landing-page @protocol
Feature: Protocol Landing Page as SPA ContentNode
  As a visitor to elohim.host
  I want to see a fast, informative landing page
  That is itself delivered as a protocol content node

  Background:
    Given doorway is configured with ROOT_APP_SLUG "protocol-landing"
    And the protocol-landing SPA blob is extracted in the cache

  # --- Page Content ---

  Scenario: Landing page loads with hero section
    When I visit "/"
    Then I see a hero section with the protocol's vision statement
    And the page loads in under 1 second (no framework overhead)

  Scenario: Landing page shows manifesto summary
    When I visit "/"
    Then I see a summary of the manifesto's executive summary
    And there is a link to read the full manifesto

  Scenario: Landing page shows five pillars
    When I visit "/"
    Then I see five pillar cards: Lamad, ImagoDei, Qahal, Shefa, Elohim
    And each card has a title, icon, and one-sentence description

  Scenario: Landing page shows live protocol stats
    When I visit "/"
    And the doorway health endpoint reports 42 content nodes and 3 humans
    Then I see "42 content nodes" and "3 humans" in the stats section

  Scenario: Landing page has call-to-action to enter learning platform
    When I visit "/"
    Then I see a "Start Learning" button
    And clicking it navigates to "/lamad"

  # --- Protocol-Native Delivery ---

  Scenario: Landing page is a ContentNode
    When I inspect the HTTP response for "/"
    Then the response includes "X-Root-App: protocol-landing"
    And the response includes "X-Content-Address" header

  Scenario: Landing page has proper SEO meta tags
    When I visit "/"
    Then the page has og:title "Elohim Protocol"
    And the page has og:description containing "human flourishing"

  # --- Fallback ---

  Scenario: Bootstrap page shown when SPA not yet loaded
    Given the protocol-landing blob is NOT yet extracted
    When I visit "/"
    Then I see the bootstrap page "Connecting to the Elohim Protocol..."
    And the page auto-refreshes when the SPA becomes available
