@browser @elohim-core @chrome-preferences
Feature: Chrome preferences follow the person across EPR-app boundaries
  Theme and language controls live in the protocol chrome (omnibar, navigator)
  and persist device-wide through one shared contract, so crossing a bundle
  boundary never resets how the protocol looks or speaks.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @wip @browser-only
  Scenario: Theme choice persists across the app boundary
    When Matthew navigates to "/lamad" in the browser
    And Matthew clicks the element with testid "nav-theme-inline"
    And Matthew clicks the element with testid "lamad-footer-home-link"
    Then the body data-theme attribute equals the chosen theme
    And there should be no console errors

  @wip @browser-only
  Scenario: Switching to Hebrew flips the chrome to RTL and persists
    When Matthew navigates to "/lamad" in the browser
    And Matthew clicks the element with testid "profile-bubble"
    And Matthew clicks the element with testid "nav-language"
    And Matthew clicks the element with testid "nav-language"
    Then the document dir attribute is "rtl"
