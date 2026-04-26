@recovery-m5 @auth-portal-convergence
Feature: Doorway redirects steward humans to their portal host
  As a steward who has graduated from hosted to peer-native
  I want doorway/account to point me at my own steward
  So that I manage my account from peer-native infrastructure

  Background:
    Given I am authenticated at a doorway as a graduated steward
    And my AccountView includes a portal host at "https://matthew.steward.example/account"
    And the portal host responds to /healthz with 200

  Scenario: Doorway shows the redirect link
    When I navigate to doorway/account
    Then I see a "Manage from your steward →" button

  Scenario: Click redirects with a session token
    When I click "Manage from your steward →"
    Then I am redirected to the portal host URL with a session_token query parameter
    And elohim-app's account-guard consumes the session_token
    And I land on /account/security authenticated as the same identity

  Scenario: No reachable portal host — fall through to hosted view
    Given my portal host does not respond to /healthz
    When I navigate to doorway/account
    Then I do NOT see the "Manage from your steward →" button
    And I see the existing hosted account view
