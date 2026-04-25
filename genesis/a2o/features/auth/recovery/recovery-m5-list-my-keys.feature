@recovery-m5 @account-pillar
Feature: Listing my keys in the account-management surface
  As a steward
  I want to see my active key and revocation history
  So that I can verify my account's security posture

  Background:
    Given I am authenticated as a hosted human with a graduated steward presence
    And my AccountView includes one active KeyRotation and zero recent KeyRevocations

  Scenario: Active key is visible on the Security & sign-in pane
    When I navigate to /account/security
    Then I see the "Security & sign-in" pane title
    And I see my active key listed under "My keys"
    And the active key shows its rotation date

  Scenario: Revocation history is visible
    Given my AccountView has one revoked key with triggerType "voluntary"
    When I navigate to /account/security
    Then I see the revoked key under "My keys"
    And the revoked key is labelled with triggerType "voluntary"

  Scenario: AccountView active key uses the newAgentPubkey field
    Given my AccountView includes an active KeyRotation
    When I navigate to /account/security
    Then the active key display reads from the newAgentPubkey field of the KeyRotation entry
    And no field named newPubKey is referenced
