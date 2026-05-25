@e2e @auth @browser-only @peer-oauth-portal @wip
Feature: RP consent — external app requests user claims
  Per RFC-6749, when an external relying party redirects the user to the
  authorization endpoint, the portal renders a consent surface that names
  the requesting app and lists the claims it wants.

  Scenario: User approves a per-claim consent
    Given matthew is signed in to alpha.elohim.host (Mode A)
    And graphos-designer.elohim.host is a registered OAuth client
    When matthew is redirected to "/auth/portal?client_id=graphos-designer&claims=imagodei.displayName,qahal.standing&redirect_uri=...&state=abc"
    Then the consent-card renders with graphos-designer as the requesting client
    And both claims are listed with toggles
    And "imagodei.displayName" is required (locked on)

    When matthew approves
    Then a 5-min single-use OAuth code is issued
    And matthew is redirected to graphos-designer's redirect_uri with code + state preserved

  Scenario: User declines consent
    Given matthew is on the consent-card for graphos-designer
    When matthew clicks Decline
    Then matthew is redirected to graphos-designer's redirect_uri with error=access_denied + state preserved
