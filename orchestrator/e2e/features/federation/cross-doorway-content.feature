@e2e @federation
Feature: Cross-Doorway Content Discovery

  Two humans on different doorways can create and discover each other's content
  through the federated network. This validates that the alpha↔staging federation
  link propagates content between doorway instances.

  Background:
    Given doorway "alpha" is healthy at env "E2E_DOORWAY_ALPHA"
    And doorway "staging" is healthy at env "E2E_DOORWAY_STAGING"

  Scenario: Content created on alpha is discoverable from staging
    Given human "Alice" is registered on doorway "alpha"
    And human "Bob" is registered on doorway "staging"
    When Alice creates content "e2e-test-article" on doorway "alpha"
    Then Bob should see content "e2e-test-article" on doorway "staging" within 60 seconds

  Scenario: Content created on staging is discoverable from alpha
    Given human "Alice" is registered on doorway "alpha"
    And human "Bob" is registered on doorway "staging"
    When Bob creates content "e2e-test-reverse" on doorway "staging"
    Then Alice should see content "e2e-test-reverse" on doorway "alpha" within 60 seconds
