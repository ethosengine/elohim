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
    - MemPalace wing=memory: project_m5_reframe_auth_portal_convergence (graduated 2026-06-02)
    - MemPalace wing=memory: project_peer_native_account_canonical_surface (graduated 2026-06-02)

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is a graduated steward with portal host "https://matthew.steward.example/account"
    And the portal host responds to /healthz with 200

  # ---------------------------------------------------------------------------
  # @wip — the REGISTRATION leg does not exist yet (2026-08-20)
  #
  # The two positive hand-off scenarios below are NOT a regression and NOT a
  # missing doorway field. Doorway already emits both fields on /auth/login
  # (AuthResponse.isSteward + AuthResponse.portalHostUrl, populated by
  # generate_auth_response → probe_first_portal_host, landed in 6ada3ad55 /
  # 927db4bc6), and doorway-app already reads portalHostUrl and navigates
  # (threshold-login.component.ts, mints a single-use session_token and
  # preserves the OAuth params). Both ends of the hand-off are built.
  #
  # What is missing is the middle: a hosted human cannot REGISTER a portal host
  # at all today, so the probe has no row to return and portalHostUrl is
  # correctly absent. Two independent blocks, both verified 2026-08-20:
  #
  #   1. Doorway's storage proxy never injects X-Agent-Id. `forward_to_storage`
  #      (routes/storage_proxy.rs) forwards Content-Type, Authorization,
  #      X-Observation-Id and X-Agent-Cid only, while storage's account routes
  #      resolve identity from X-Agent-Id or a local Tauri session
  #      (elohim-storage/src/api/account.rs::extract_agent_key — its own doc
  #      calls X-Agent-Id "the header doorway injects after JWT validation").
  #      Probe: POST {doorway}/api/v1/account/portal-hosts with a valid bearer
  #      → 400 {"error":"missing X-Agent-Id and no active session"}.
  #   2. Even with that header, storage refuses the write: handle_add_portal_host
  #      gates on verify_caller_owns_cell and returns 503
  #      BROWSER_WRITE_PATH_PENDING — "the browser-via-doorway write path is
  #      deferred to M6 where the hosting trust model is settled".
  #
  # Live confirmation on the deployed fleet (2026-08-20): Matthew logs in with
  # isSteward:true and no portalHostUrl, and GET /auth/portal-host returns
  # {"reachable":false,"hostUrl":null,"allHosts":[]} — no row exists to probe.
  # The a2o fixture's registration POST is best-effort and silently swallows
  # this, which is why the failure surfaced only at the assertion.
  #
  # These stay written and stay bound (steps are defined; --dry-run is clean).
  # They flip to green with no edit here the moment the registration write path
  # lands. Do NOT satisfy them by staging a doorway-local fixture portal host:
  # portal_hosts is a Category-A notarized DHT entry, and inventing a second
  # source of truth for it in doorway would make these scenarios green while
  # the real path stayed broken.
  # ---------------------------------------------------------------------------
  @elohim-visually-validated @wip
  Scenario: Matthew's login response carries his portal host URL
    When Matthew submits credentials at the threshold-login page
    Then the /auth/login response includes "isSteward": true
    And the /auth/login response includes a "portalHostUrl" matching his registered host

  @elohim-visually-validated @wip
  Scenario: Doorway redirects Matthew to his portal host after auth
    When Matthew submits credentials at the threshold-login page
    Then the browser is redirected to "https://matthew.steward.example/account"
    And the redirect URL carries a session_token query parameter
    And the redirect URL preserves the OAuth client_id, redirect_uri, response_type, and state when present

  # Honest reading while the registration leg is missing: this scenario's
  # "does NOT include portalHostUrl" passes, but it cannot yet DISTINGUISH
  # "the portal host was probed and found unreachable" from "no portal host was
  # ever registered". It becomes a real fall-through proof once the two @wip
  # scenarios above can go green. Left running — the assertion is correct, and
  # the fall-through half (doorway completes auth itself, no hand-off) is
  # genuinely observed.
  Scenario: Portal host unreachable — fall through to local auth
    Given the portal host does not respond to /healthz
    When Matthew submits credentials at the threshold-login page
    Then the /auth/login response includes "isSteward": true
    And the /auth/login response does NOT include "portalHostUrl"
    And the browser completes the OAuth dance at the doorway as relying-party-and-identity-provider

  @requires:shem
  Scenario: Hosted visitor receives no portalHostUrl
    Given human "Susan" is a hosted visitor with no portal host registered
    When Susan submits credentials at the threshold-login page
    Then the /auth/login response includes "isSteward": false
    And the /auth/login response does NOT include "portalHostUrl"
    And the browser completes the OAuth dance at the doorway normally
