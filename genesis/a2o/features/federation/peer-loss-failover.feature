@e2e @federation @resilience
Feature: Peer-loss failover — reads keep serving while a household peer is down
  As a household member reading content during a device outage
  I want reads to be served by the surviving household peers, and the
  returning peer to re-sync on its own
  So that one device going dark never makes the household's content
  unreachable — and so that a single device alone still works, because
  hubs and meshes are conveniences, never conditions of participation.

  # Evidence anchor (2026-06-10 Phase 0, EPR durability arc): genesis #1118's
  # cross-pod fetch 404'd — jessica could not read matthew's blob with the
  # whole mesh nominally up. Failover must be proven by killing a peer on
  # purpose, not discovered when one dies on its own.
  # Plan: genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md (Workstream C)

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Reads still serve while one household peer is down
    Given the manifesto blob is replicated under custody across the household mesh
    And every household peer meets its connected-peers floor
    When Jessica's storage peer goes down
    Then fetching the manifesto blob through the doorway still returns the content
    And the serving peer is a surviving household peer

  Scenario: A returning peer re-syncs without operator help
    Given Jessica's storage peer was down while the mesh kept serving
    When Jessica's storage peer comes back
    Then within the re-sync window Jessica's peer meets its connected-peers floor again
    And Jessica's peer inventory parity matches the mesh again

  Scenario: Household adjacency is bidirectional
    Given the household peers matthew, jessica, and james are up
    When each peer's live peer set is inspected
    Then every household peer lists every other household peer as connected

  Scenario: A single device still functions without the mesh
    Given Jessica's storage peer holds content it stewards locally
    When every other household peer is unreachable
    Then Jessica's peer still serves its locally stewarded content
    And the degraded mesh state is visible in her peer status, not hidden
