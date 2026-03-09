@browser @navigation @hosted-human @requires:doorway
Feature: Browser Navigation Health
  Verify that key pages load without errors in a real browser.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" with device

  Scenario: Home page loads cleanly
    When Matthew navigates to "/" in the browser
    Then the page should load successfully
    And there should be no console errors

  Scenario: Learning hub loads cleanly
    When Matthew navigates to "/lamad" in the browser
    Then the page should load successfully
    And the page should display the main content
    And there should be no console errors

  Scenario: Profile page loads cleanly
    When Matthew navigates to "/identity/profile" in the browser
    Then the page should load successfully
    And there should be no console errors
