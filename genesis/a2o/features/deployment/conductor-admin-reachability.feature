@e2e @deployment @regression @conductor-admin
Feature: Conductor admin WebSocket is reachable through elohim-storage
  As a protocol operator
  I want every human's conductor admin interface reachable from doorway
  So that /auth/register and admin operations don't regress to "Connection refused"

  # Regression guard for sprint-report fingerprints surfaced during the
  # 2026-04-21 consolidate-human-deployments migration:
  #   75ca9eb6e5dc  Adam admin :4444 refused (8× in #940; 0 after forwarder fix)
  #   68407ba1be5a  legacy-human admin refused via old port derivation
  #
  # Root cause was that spawn_forwarder("0.0.0.0:4444") fails with EADDRINUSE
  # because the Holochain conductor already binds 127.0.0.1:4444 in the same
  # pod netns. The fix moved the Rust forwarder to :8444/:8445 externally,
  # matching socat convention. If this test fails with "Connection refused"
  # in a register error body, the forwarder is not binding correctly.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Registration succeeds without surfacing connection-refused errors
    When a fresh human is registered on doorway "alpha"
    Then the register response is 200 with a valid agent identifier
    And the register response does not mention a refused admin socket
