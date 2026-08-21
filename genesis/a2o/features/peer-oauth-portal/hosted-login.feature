@e2e @auth @browser-only @requires:doorway @peer-oauth-portal @wip
Feature: Mode A — Doorway-hosted login
  As a new visitor to alpha.elohim.host, I sign in via the federated portal
  and the doorway hosts my conductor while I settle in.

  @act:ii
  Scenario: First-time sign-in surfaces the flywheel chrome
    Given the alpha.elohim.host doorway has a projection for the peer-oauth-portal at "/auth/portal"
    And matthew is a pre-registered imagodei on alpha.elohim.host with password "shibboleth"
    When matthew opens "https://alpha.elohim.host/auth/portal?returnTo=/lamad"
    And types "matthew@alpha.elohim.host" into the federated-resolver
    Then the portal advances to the login-card step
    And the trust-indicator reads "Hosted via alpha.elohim.host" with the flywheel hint visible

    When matthew submits the password "shibboleth"
    Then the doorway sets the elohim_session cookie
    And the trust-indicator updates to show matthew's humanId
    And the browser navigates to "/lamad"

  @act:i
  Scenario: Wrong password preserves trust-indicator chrome
    Given matthew is on the login-card step at alpha.elohim.host
    When matthew submits an incorrect password
    Then the login-card shows the inline credentials error
    And the trust-indicator remains visible at the top of the shell
    And an "ask for help" link points to /identity/recovery
