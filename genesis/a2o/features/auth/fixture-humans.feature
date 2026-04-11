@e2e @auth @fixture-sweep @hosted-human @requires:doorway @requires:seeded-humans
Feature: Fixture Human Categories
  All human categories from humans.json can login successfully.
  This validates that the seeder correctly provisioned every persona.

  Each scenario represents a different community stratum — the platform
  must work for all of them before we can test agency transitions.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Core family — Matthew's household
    Given human "Matthew" is logged in on doorway "alpha"
    And human "Susan" is logged in on doorway "alpha"
    And human "James" is logged in on doorway "alpha"
    And human "Gertrude" is logged in on doorway "alpha"
    Then all 4 humans should have distinct tokens

  Scenario: Community humans
    Given human "Nancy" is logged in on doorway "alpha"
    And human "Pam" is logged in on doorway "alpha"
    Then all 2 humans should have distinct tokens

  Scenario: Affinity group — learning and faith
    Given human "Pete" is logged in on doorway "alpha"
    And human "Timothy" is logged in on doorway "alpha"
    And human "Tommy" is logged in on doorway "alpha"
    And human "Meriadoc" is logged in on doorway "alpha"
    Then all 4 humans should have distinct tokens

  Scenario: Local economy humans
    Given human "Frank" is logged in on doorway "alpha"
    And human "Georgina" is logged in on doorway "alpha"
    And human "Manny" is logged in on doorway "alpha"
    And human "Bub" is logged in on doorway "alpha"
    Then all 4 humans should have distinct tokens

  Scenario: Newcomers
    Given human "Maria" is logged in on doorway "alpha"
    And human "Ronald" is logged in on doorway "alpha"
    Then all 2 humans should have distinct tokens

  Scenario: Red-team humans can login
    Given human "Charlie" is logged in on doorway "alpha"
    And human "Sam" is logged in on doorway "alpha"
    And human "Dr. Dolittle" is logged in on doorway "alpha"
    Then all 3 humans should have distinct tokens
