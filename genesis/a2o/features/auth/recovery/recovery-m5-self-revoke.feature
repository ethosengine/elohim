@recovery-m5 @account-pillar @revocation
Feature: Self-revocation through the account-management surface
  As a steward concerned my key may be compromised
  I want to revoke my current key
  So that peers stop trusting it immediately

  # ─────────────────────────────────────────────────────────
  # M5 reality — POST mutations return 503 until Phase-11 bridge
  # ─────────────────────────────────────────────────────────

  Scenario: Self-revocation mutation returns 503 PHASE_11_PENDING in M5
    Given I am authenticated with an active key
    And I navigate to /account/security
    When I click "Revoke this key"
    And I click "Yes, revoke"
    Then the POST to /api/v1/account/self-revocation returns 503
    And the response body contains errorCode "PHASE_11_PENDING"
    And the UI displays an informative error message rather than crashing

  # ─────────────────────────────────────────────────────────
  # Success path — will work once the Phase-11 bridge exists
  # ─────────────────────────────────────────────────────────

  Scenario: Successful self-revocation
    Given I am authenticated with an active key
    And I navigate to /account/security
    When I click "Revoke this key"
    And I click "Yes, revoke"
    Then a KeyRevocation entry is committed with triggerType "voluntary"
    And the KeyRevocation entry contains a revokedKey field
    And the KeyRevocation entry contains a createdAt field
    And the AccountView refreshes to show the key as revoked
    And my Security & sign-in pane reflects the revocation

  # ─────────────────────────────────────────────────────────
  # Cancel path — no network call issued
  # ─────────────────────────────────────────────────────────

  Scenario: Cancel before confirming does not revoke
    Given I am authenticated with an active key
    When I click "Revoke this key"
    And I click "Cancel"
    Then no KeyRevocation entry is committed
