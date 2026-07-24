@e2e @resilience @federation @convergence-bar @regression @requires:household-nodes @concern:reconcile-inventory @dataplane
Feature: Federated doorways agree on the commons resilience footprint
  As a commons steward asking the mesh whether an EPR is held
  I want any two doorways to testify the same commitment-backed holder footprint
  So that convergence is a measured protocol property, not a claim about one pod

  # Guide-star convergence bar. The EPR is DHT-notarized (Category A);
  # /api/v1/resilience/{id}/household is a reconstructable operational
  # projection (Category C). The judge therefore compares the doorway
  # testimonies and never treats either response as a new source of truth.
  #
  # The bounded window admits ordinary projection/gossip propagation. Equality
  # is order-insensitive over holder kind+id and exact for
  # commitmentBackedCollectives. A fixture that is not commitment-backed, or a
  # doorway that cannot name its holders, is an honest red.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_BETA"

  Scenario: Two doorways testify the same footprint for one commons EPR
    Given the household fixture names a commitment-backed commons EPR
    Then within 60 seconds doorways "alpha" and "beta" testify the same household footprint
