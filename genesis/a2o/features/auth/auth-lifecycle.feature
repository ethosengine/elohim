@e2e @auth @hosted-human @requires:doorway
Feature: Authentication Lifecycle
  As a hosted human on a doorway
  I want to register, login, verify my session, and prove identity
  So that I have a secure session before doing anything else

  This is the first gate of agency: you exist on someone else's doorway.
  Matthew is the primary human moving through all agency phases.
  Other humans validate that the platform works for different personas.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- Registration (new human arrives) ---

  Scenario: Register a new human on a doorway
    When a new human "Newcomer" registers on doorway "alpha"
    Then the auth response should include a token
    And the auth response should include a humanId
    And the auth response should include an agentPubKey

  # --- Login (existing humans return) ---

  Scenario: Matthew logs in as admin
    Given human "Matthew" is logged in on doorway "alpha"
    Then Matthew should have a valid token
    And Matthew's identifier should be "matthew.dowell@alpha.elohim.host"

  Scenario: Susan logs in as a family member
    Given human "Susan" is logged in on doorway "alpha"
    Then Susan should have a valid token
    And Susan's identifier should be "susan@test.elohim.host"

  Scenario: Login with wrong password fails
    When human "Matthew" attempts login on doorway "alpha" with password "wrong-password"
    Then the login should fail

  # --- Session verification ---

  Scenario: Verify doorway health after login
    Given human "Matthew" is logged in on doorway "alpha"
    When Matthew checks doorway health
    Then the doorway should be healthy

  Scenario: Verify identity with /auth/me
    Given human "Matthew" is logged in on doorway "alpha"
    When Matthew checks their identity
    Then the identity should match Matthew's credentials

  # --- Full account lifecycle ---

  Scenario: Register, login, logout, login again
    When a new human "Lifecycle" registers on doorway "alpha"
    Then the auth response should include a token
    When Lifecycle checks their identity
    Then the identity check should succeed
    When Lifecycle logs out of doorway "alpha"
    Then Lifecycle's session should be cleared
    When Lifecycle checks their identity
    Then the identity check should fail with unauthorized
    When Lifecycle logs back in on doorway "alpha"
    Then Lifecycle should have a valid token
    When Lifecycle checks their identity
    Then the identity check should succeed

  # --- Concurrent sessions (the family is online) ---

  Scenario: Multiple humans logged in simultaneously
    Given human "Matthew" is logged in on doorway "alpha"
    And human "Susan" is logged in on doorway "alpha"
    And human "Timothy" is logged in on doorway "alpha"
    Then all 3 humans should have distinct tokens
    And all 3 humans should have distinct humanIds
