@recovery-m5 @account-pillar @recovery-entry
Feature: Lost-key entry point routes to the right flow
  As a steward who cannot sign in
  I want a single entry point for "I lost my key"
  So that I am routed to recovery or revocation based on my state

  Scenario: Active key holder is routed to recovery
    Given I am authenticated and have an active key
    When I navigate to /account/security
    And I click "Start recovery" under "I lost my key"
    Then I am redirected to /identity/recover

  Scenario: No active key — routed to recovery anyway
    Given I am authenticated but my account has no active key
    When I navigate to /account/security
    And I click "Start recovery" under "I lost my key"
    Then I am redirected to /identity/recover
