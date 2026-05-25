@e2e @auth @browser-only @peer-oauth-portal @wip
Feature: Mode B — Peer-conductor login
  My conductor on my own storage instance is the auth authority.
  Doorway is at most a transparent ingress, or absent entirely (Tauri).

  Scenario: Sign-in via doorway-routed peer-conductor
    Given matthew has graduated to running his own conductor
    And alpha.elohim.host is configured to route auth requests for matthew to his conductor
    When matthew opens "https://alpha.elohim.host/auth/portal"
    And the portal queries /auth/me
    Then the response reports trustMode "peer-conductor" and authority "matthew's conductor"
    And the trust-indicator reads "Your conductor — alpha.elohim.host is helping with ingress"

  @manual @tauri-only @skip
  Scenario: Sign-in via Tauri direct (no doorway)
    # Requires a Tauri webview test harness, which the project does not
    # provide. Per the peer OAuth portal spec §6.2, Tauri is operator-owned
    # parallel work; this scenario is preserved here for documentation and
    # manual dogfood verification (Task G1). It is excluded from automated
    # cucumber runs via the @manual @tauri-only @skip tags.
    Given matthew has the Tauri app installed with his conductor running locally
    When matthew opens the portal inside the Tauri webview
    And the portal queries /auth/me against localhost:8090
    Then the trust-indicator reads "Your conductor on this device"
    And no doorway-ingress label appears
