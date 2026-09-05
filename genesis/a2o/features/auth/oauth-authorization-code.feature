@e2e @auth @oauth @requires:doorway @concern:oauth-authorization-code
Feature: OAuth authorization-code flow — the doorway delegates, and bounds what it delegates
  As a human with a session on a doorway
  I want an application to receive a code that only my doorway could have issued
  So that delegating access to an app never becomes a way to hand my identity to a stranger

  A DOORWAY is the gateway a hosted human reaches the protocol through, and it acts as
  their OAuth authorization server: it holds the session, and it decides which application
  may act on their behalf. `GET /auth/authorize` mints a short-lived single-use code for an
  already-authenticated human, and `POST /auth/token` exchanges it for an access token.
  This feature is the first assertion of that flow at any layer.

  Applications are registered with the doorway ahead of time, each bounded to the callbacks
  it may receive a code at. "elohim-app" is registered and its elohim.host callbacks are
  bounded to `https://*.elohim.host/*`; "graphos-designer" is not registered at all. The
  Background asserts the first fact rather than assuming it; the second is what the opening
  scenario proves.

  `state` is an opaque value the application sends and the doorway must hand back unchanged,
  so the application can tell that the response answers the request it actually made rather
  than one forged by someone else. Every refusal below must preserve it too — an error that
  drops `state` strands the application with a failure it cannot attribute.

  A signed-out application may add `prompt=create` when the human asked to make an account.
  That changes only which doorway portal receives them; the application, callback, response
  type, and state still describe the same authorization request and must travel with them.

  The redirect_uri is the whole security boundary here. A code is a bearer credential for
  the human's identity and redirect_uri decides who receives it, so a registered
  application's bound must hold the SCHEME, HOST and PORT — not merely the shape of the
  string. Scenario "A hostile callback is refused" is the regression for a live
  authorization-code interception (proved on the local mesh 2026-08-28): the matcher
  compared only the text before the first `*` and after the last one, so
  `https://*.elohim.host/*` accepted EVERY https URI and the doorway issued a real code to
  an attacker-controlled callback.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And application "elohim-app" is registered on doorway "alpha" with callbacks bounded to "https://*.elohim.host/*"
    And "Miriam" holds an open session on doorway "alpha"

  # Miriam is provisioned fresh for each scenario. A code is minted against the human who
  # authorized it, so a session shared between scenarios would let one scenario redeem
  # another's code and the single-use assertion would stop meaning anything.
  #
  # "A signed-out human is redirected to log in" deliberately does NOT use Miriam's session —
  # it asks the same doorway as an anonymous browser. Her session is set up and left unused
  # there, which is the point: the doorway must decide on the request in front of it.

  @act:i
  Scenario: An unregistered application is refused before anything is minted
    When Miriam's doorway is asked to authorize "graphos-designer" with callback "https://graphos-designer.elohim.host/callback"
    Then the authorization response is refused with error "invalid_client"
    And the authorization error preserves the state parameter

  @act:i
  Scenario: A signed-out human is redirected to log in without losing the request
    When a signed-out browser asks doorway "alpha" to authorize "elohim-app" with callback "https://elohim.host/cb"
    Then the authorization response redirects to the login surface
    And the login redirect preserves the client_id, redirect_uri, response_type and state

  @act:i
  Scenario: A signed-out human asking to create an account is sent to registration without losing the request
    When a signed-out browser asks doorway "alpha" to authorize "elohim-app" with callback "https://elohim.host/cb", response type "code", state "create-account-request" and prompt "create"
    Then the authorization response redirects to the registration surface
    And the registration redirect preserves the client_id, redirect_uri, response_type and state

  @act:i
  Scenario Outline: A hostile callback is refused even for a registered application
    When Miriam's doorway is asked to authorize "elohim-app" with callback "<hostile>"
    Then the authorization response is refused with error "invalid_redirect_uri"
    And no authorization code is present in the response

    Examples: callbacks outside the bound — four host evasions and one scheme downgrade
      | hostile                          |
      | https://attacker.tld/steal       |
      | https://a.b.c.evil.co.uk/x       |
      | https://elohim.host.evil.tld/cb  |
      | https://evil.tld/.elohim.host/cb |
      | http://app.elohim.host/callback  |

  @act:i
  Scenario: A bounded callback receives a code that spends exactly once
    When Miriam's doorway is asked to authorize "elohim-app" with callback "https://app.elohim.host/callback"
    Then an authorization code is issued to "https://app.elohim.host/callback"
    And the authorization response preserves the state parameter

    When the authorization code is exchanged at doorway "alpha" for client "elohim-app"
    Then the token response carries an access token belonging to Miriam

    When the same authorization code is exchanged again at doorway "alpha" for client "elohim-app"
    Then the token response is refused with error "invalid_grant"
