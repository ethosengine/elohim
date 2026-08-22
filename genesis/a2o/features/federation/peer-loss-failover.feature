@e2e @federation @resilience @act:i
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

  # "Parity" is each peer's gossip view agreeing with its own filesystem, and
  # Jessica holding no fewer blobs than before she went dark. Household peers
  # legitimately hold DIFFERENT blob sets (the seeding peer carries the corpus,
  # the others hold what custody placed on them), so equal counts across the
  # mesh is not the claim — it was never true, outage or not.
  Scenario: A returning peer re-syncs without operator help
    Given Jessica's storage peer was down while the mesh kept serving
    When Jessica's storage peer comes back
    Then within the re-sync window Jessica's peer meets its connected-peers floor again
    And Jessica's peer inventory parity matches the mesh again

  Scenario: Household adjacency is bidirectional
    Given the household peers matthew, jessica, and james are up
    When each peer's live peer set is inspected
    Then every household peer lists every other household peer as connected

  # A paused peer sends no FIN; what makes it leave Jessica's connected set is
  # a failed libp2p ping, on which elohim-storage closes the connection (ping
  # interval 15 s + timeout 20 s by default, so ≤ ~35 s). Before that close
  # landed (2026-08-22) a silent peer stayed "connected" for the 300 s idle
  # timeout and this status read a full floor with the whole mesh paused.
  Scenario: A single device still functions without the mesh
    Given Jessica's storage peer holds content it stewards locally
    When every other household peer is unreachable
    Then Jessica's peer still serves its locally stewarded content
    And the degraded mesh state is visible in her peer status, not hidden
