@e2e @auth @session-handoff @requires:doorway
Feature: Cross-App Session Handoff
  As Matthew, the genesis steward operating the doorway
  I want my session to persist when I navigate from elohim-app to doorway-app
  So that I don't have to log in twice when managing my doorway

  Matthew is the only real human on the alpha deployment. He logs in via
  elohim-app (lamad), clicks "Visit" on his doorway from the imagodei
  profile, and should arrive authenticated in doorway-app as the operator
  without a second login prompt.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"

  # --- Token handoff via session transfer token ---

  Scenario: Matthew obtains a session transfer token
    When Matthew requests a session transfer token
    Then the transfer token should be present
    And the transfer token should expire within 60 seconds

  Scenario: Matthew exchanges session transfer token for JWT
    Given Matthew has a session transfer token
    When Matthew exchanges the session transfer token
    Then the exchange should return a valid JWT
    And Matthew should be able to verify identity with the new JWT

  Scenario: Session transfer token is single-use
    Given Matthew has a session transfer token
    When Matthew exchanges the session transfer token
    Then the exchange should return a valid JWT
    When Matthew attempts to exchange the same transfer token again
    Then the second exchange should fail

  Scenario: Expired session transfer token is rejected
    Given Matthew has an expired session transfer token
    When Matthew attempts to exchange the expired transfer token
    Then the exchange should fail with unauthorized

  # --- End-to-end handoff (API-level simulation) ---

  Scenario: Full handoff from elohim-app to doorway-app
    When Matthew requests a session transfer token
    And Matthew opens the doorway-app with the transfer token
    Then the doorway-app account endpoint should return Matthew's account
    And the account identifier should be "matthew.dowell@alpha.elohim.host"
