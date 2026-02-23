@browser @auth @hosted-human @requires:doorway
Feature: Browser Authentication
  As a hosted human using a real browser
  I want to login through the UI and verify the app loads correctly
  So that I know the auth flow works end-to-end

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Matthew logs in through the browser UI
    Given human "Matthew" has a browser on doorway "alpha"
    When Matthew logs in through the browser on doorway "alpha"
    Then Matthew should see the authenticated shell
    And there should be no console errors after login

  Scenario: Login with wrong password shows error in browser
    Given human "Matthew" has a browser on doorway "alpha"
    When Matthew enters wrong credentials in the browser
    Then the login page should show an error message
    And there should be no uncaught JS errors

  Scenario: Page loads without console errors
    Given human "Matthew" has a browser on doorway "alpha"
    When Matthew logs in through the browser on doorway "alpha"
    And Matthew navigates to "/lamad" in the browser
    Then Matthew should see the authenticated shell
    And there should be no console errors
    And there should be no failed network requests

  Scenario: Logout through the browser UI
    Given human "Matthew" has a browser on doorway "alpha"
    When Matthew logs in through the browser on doorway "alpha"
    And Matthew logs out through the browser UI
    Then Matthew should be redirected to the login page
    And there should be no uncaught JS errors
