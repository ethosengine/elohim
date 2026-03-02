@e2e @deployment @conductor-visibility @requires:doorway
Feature: Conductor Pool Visibility
  As Matthew, the genesis steward operating the alpha doorway
  I want to see the conductor pool — which conductors are running,
  their capacity, and which agents are hosted where
  So that I can monitor resource usage and plan scaling

  The conductor pool is the heart of the hosted human experience.
  Each conductor runs a Holochain instance pool. Matthew needs to see
  how many are running, how full they are, and which users live where.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"

  Scenario: Matthew lists conductors in the pool
    When Matthew queries the conductor pool
    Then the conductor pool response should include a total count
    And the conductor pool should include total capacity metrics

  Scenario: Matthew views agents on a conductor
    When Matthew queries the conductor pool
    And Matthew views agents on the first conductor
    Then the conductor agents response should include the conductor ID
    And the agents list should be an array

  Scenario: Matthew checks which conductor hosts a user
    Given human "Susan" is logged in on doorway "alpha"
    When Matthew looks up which conductor hosts Susan
    Then the agent conductor lookup should return a conductor ID
    And the lookup should include the conductor URL

  Scenario: Non-admin cannot access conductor pool
    Given human "Susan" is logged in on doorway "alpha"
    When Susan attempts to access the conductor pool
    Then the request should be forbidden
