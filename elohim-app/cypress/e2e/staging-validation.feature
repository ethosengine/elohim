Feature: Staging Site Validation
  As a deployment pipeline
  I want to validate that the staging site is working correctly
  So that I can safely proceed to production deployment

  Scenario: Staging site loads successfully
    Given I navigate to the staging site
    When the page loads
    Then the site should be accessible
    And the page should display the main content
    And there should be no critical errors in the console

  Scenario: Essential page elements are present
    Given I navigate to the staging site  
    When the page loads
    Then the page title should contain "elohim.host"
    And the hero section should be displayed
    And the footer should be present