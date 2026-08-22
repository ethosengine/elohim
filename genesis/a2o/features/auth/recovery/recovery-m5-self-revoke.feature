# @browser: every scenario navigates and clicks the account surface — Playwright is the
# driving mechanism (steps/ui/account-m5.steps.ts), so the browser lane runs it; in the
# HTTP lane the steps hold at the first navigation.
@recovery-m5 @account-pillar @revocation @act:i @e2e @browser
Feature: Self-revocation through the account-management surface
  As a steward concerned my key may be compromised
  I want to revoke my current key
  So that peers stop trusting it immediately

  # ─────────────────────────────────────────────────────────
  # Success path
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
