@e2e @auth @browser-only @requires:doorway @recovery-m5 @auth-portal-convergence
Feature: Doorway hands the login session off to the steward's portal host
  As Matthew, a graduated steward whose peer-native portal is registered
  I want doorway to recognize me as a steward and hand authentication to my portal host
  So that doorway never owns my login decision — it is only the relying party

  Doorway's role for a graduated steward is OAuth-relying-party: it presents
  the form, but it does not adjudicate the credential. When the credential
  check succeeds and the AuthResponse carries a reachable portalHostUrl,
  doorway redirects to the peer-native portal, which completes the OAuth
  code dance back at the original client_id.

  See:
    - genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md
    - .claude/memory/project_m5_reframe_auth_portal_convergence.md
    - .claude/memory/project_peer_native_account_canonical_surface.md

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is a graduated steward with portal host "https://matthew.steward.example/account"
    And the portal host responds to /healthz with 200

  @elohim-visually-validated
  Scenario: Matthew's login response carries his portal host URL
    When Matthew submits credentials at the threshold-login page
    Then the /auth/login response includes "isSteward": true
    And the /auth/login response includes a "portalHostUrl" matching his registered host

  @elohim-visually-validated
  Scenario: Doorway redirects Matthew to his portal host after auth
    When Matthew submits credentials at the threshold-login page
    Then the browser is redirected to "https://matthew.steward.example/account"
    And the redirect URL carries a session_token query parameter
    And the redirect URL preserves the OAuth client_id, redirect_uri, response_type, and state when present

  Scenario: Portal host unreachable — fall through to local auth
    Given the portal host does not respond to /healthz
    When Matthew submits credentials at the threshold-login page
    Then the /auth/login response includes "isSteward": true
    And the /auth/login response does NOT include "portalHostUrl"
    And the browser completes the OAuth dance at the doorway as relying-party-and-identity-provider

  Scenario: Hosted visitor receives no portalHostUrl
    Given human "Susan" is a hosted visitor with no portal host registered
    When Susan submits credentials at the threshold-login page
    Then the /auth/login response includes "isSteward": false
    And the /auth/login response does NOT include "portalHostUrl"
    And the browser completes the OAuth dance at the doorway normally
