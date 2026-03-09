@e2e @auth @operator-onboarding @requires:doorway
Feature: Operator Onboarding
  As Matthew, the first steward bootstrapping the alpha doorway
  I want to register as admin using the bootstrap key,
  configure federation peers, and see the agency pipeline funnel
  So that I can bring the doorway online and start serving humans

  The bootstrap key is the genesis moment — the first admin claims the
  doorway by proving they hold the deployment secret. After that,
  Matthew configures which other doorways to federate with and monitors
  the pipeline: how many humans are registered, hosted, graduating, steward.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- Bootstrap registration (genesis moment) ---

  Scenario: First admin registers with bootstrap key
    When a new operator "GenesisAdmin" registers on doorway "alpha" with the bootstrap key
    Then the auth response should include a token
    And the auth response should include a humanId
    And GenesisAdmin should have admin permission level

  Scenario: Registration without bootstrap key gets default permissions
    When a new human "RegularUser" registers on doorway "alpha"
    Then the auth response should include a token
    And RegularUser should not have admin permission level

  # --- Federation peer configuration ---

  Scenario: Matthew configures a federation peer
    Given human "Matthew" is logged in on doorway "alpha"
    When Matthew lists federation peers
    Then the federation peers response should succeed
    When Matthew adds a federation peer "https://beta.example.elohim.host"
    Then the peer mutation should succeed
    When Matthew lists federation peers
    Then the federation peers list should include "https://beta.example.elohim.host"
    When Matthew removes the federation peer "https://beta.example.elohim.host"
    Then the peer mutation should succeed

  Scenario: Matthew refreshes the federation peer cache
    Given human "Matthew" is logged in on doorway "alpha"
    When Matthew refreshes federation peers
    Then the peer mutation should succeed

  # --- Agency pipeline funnel ---

  Scenario: Matthew views the agency pipeline
    Given human "Matthew" is logged in on doorway "alpha"
    And human "Susan" is logged in on doorway "alpha"
    When Matthew queries the agency pipeline
    Then the pipeline should show at least 2 registered users
    And the pipeline response should include all funnel stages
