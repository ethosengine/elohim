@recovery-m5 @account-pillar @portal-host
Feature: Adding and listing portal hosts
  As a steward
  I want to declare which URLs may render my auth portal
  So that doorway and trusted peers know where to redirect

  # ─────────────────────────────────────────────────────────
  # Success path
  # ─────────────────────────────────────────────────────────

  Scenario: Add a portal host
    Given I am authenticated as a steward
    When I POST {"hostUrl": "https://matthew.steward.example/account", "label": "main"} to /api/v1/account/portal-hosts
    Then the response is 200 with the new PortalHostView
    And /api/v1/account/portal-hosts returns the host in the list

  # ─────────────────────────────────────────────────────────
  # Validator gate — http URLs rejected (validator runs before bridge)
  # ─────────────────────────────────────────────────────────

  Scenario: Validator rejects http URL
    When I POST {"hostUrl": "http://insecure.example/account"} to /api/v1/account/portal-hosts
    Then the response is 400 with an error mentioning https
