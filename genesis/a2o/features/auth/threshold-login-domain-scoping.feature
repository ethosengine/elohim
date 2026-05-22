@e2e @auth @browser-only @requires:doorway @auth-portal-convergence
Feature: Doorway login is gateway-scoped, not generic email
  As an operator landing at doorway-alpha.elohim.host
  I want the login form to express the doorway's actual scope
  So that I don't mis-target a different doorway by typing my whole email

  The doorway is hosting me; it is not a generic identity provider. The
  identifier field is the username portion of my account; the gateway
  suffix is enforced by the doorway and visible to me. Federation routing
  (logging in via a different doorway) lives behind a clearly-labeled link,
  not behind a free-text field that pretends to accept any domain.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @elohim-visually-validated
  Scenario: Identifier input shows the gateway domain
    When I navigate to the threshold-login page
    Then I see the identifier input followed by a domain suffix
    And the domain suffix matches the doorway's gateway hostname
    And the hint text names the doorway's domain rather than offering "full email"

  @elohim-visually-validated
  Scenario: Identifier input strips an '@' segment if I paste one
    When I navigate to the threshold-login page
    And I type "matthew.dowell@alpha.elohim.host" into the identifier input
    Then the identifier input value is "matthew.dowell"
    And the visible domain suffix is unchanged by what I typed

  Scenario: Hint links to federated login for a different doorway
    When I navigate to the threshold-login page
    Then the hint contains a "Use a different doorway" link
    And the link points at the federated doorway picker
