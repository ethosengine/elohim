@e2e @deployment @browser-only @epic:elohim-public-observer-epic
Feature: Staging Site Validation
  As a deployment pipeline
  I want to validate that the staging site is working correctly
  So that I can safely proceed to production deployment

  Background:
    Given doorway "alpha" is healthy at env "E2E_DOORWAY_ALPHA"

  Scenario: Staging site loads successfully
    Given human "Validator" is on doorway "alpha" with device
    And I navigate to the staging site
    When the page loads
    Then the site should be accessible
    And the page should display the main content

  Scenario: Essential page elements are present
    Given human "Validator" is on doorway "alpha" with device
    And I navigate to the staging site
    When the page loads
    Then the page title should contain "elohim.host"
    And the hero section should be displayed
    And the footer should be present

  Scenario: Git hash validation
    Given human "Validator" is on doorway "alpha" with device
    And I navigate to the staging site
    When the page loads
    Then the footer should display the expected git hash
    And the git hash should match the deployed version
