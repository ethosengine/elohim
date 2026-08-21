@e2e @federation @resilience @act:i
Feature: Doorway EPR router degrades through the storage pool
  As a household member visiting elohim.host
  I want the doorway to serve the landing and lamad apps even when its
  primary storage peer is missing my doorway's projection rows
  So that one peer's gap never makes the whole front door go dark —
  the federation pair is mutual aid at the gateway layer, not a diagram.

  # Evidence anchor (2026-06-09 /deliver iter-0): doorway-B's EPR refresh
  # loop read 0 rows from its primary (adam) every 30s at DEBUG — invisible —
  # while the same pod's /db proxy was serving 3 rows from a pool peer
  # (matthew). elohim.host 302'd to /threshold for ten days.
  # Journal: .claude/deliver/journal-resilient-dual-doorway.md

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Router populates from a pool peer when the primary returns no rows
    Given the doorway's primary storage returns zero "project-epr" rows for its doorwayId
    And a configured pool peer holds the doorway's "project-epr" rows
    When the EPR router refresh runs
    Then the router table contains the pool peer's projections
    And a WARN log names the degraded primary and the serving pool peer

  Scenario: Empty everywhere is a genuine empty state, not a silent wipe
    Given the doorway's primary storage and all pool peers return zero "project-epr" rows
    When the EPR router refresh runs
    Then the router table is empty
    And the empty state is logged at INFO with the consulted peer list

  Scenario: The apex front door serves through the degraded primary
    Given doorway "apex" at "E2E_DOORWAY_APEX"
    And the apex doorway's primary storage is missing its projection rows
    When I GET "/" from the doorway
    Then the doorway response status is 200
    And the doorway response Content-Type contains "text/html"
    When I GET "/lamad" from the doorway
    Then the doorway response status is 200
