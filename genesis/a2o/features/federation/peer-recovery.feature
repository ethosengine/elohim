@e2e @federation @resilience @act:i
Feature: Peer recovery — a wiped device recovers its stewarded content from the mesh
  As a household member whose device lost its local store
  I want my device to reconverge from the household mesh on its own —
  blobs through the custody sweep, commitments through the DHT,
  projections through reconciliation
  So that losing one device's disk is an inconvenience, not a loss:
  the household's mutual custody commitments are real promises that
  re-materialize my stewarded content without anyone running a script.

  # VOCABULARY. "The manifesto blob" is the Act I fixture content — one seeded
  # document the Prologue replicates across the household, standing in for any
  # stewarded content. A "custody-blob commitment" is a household peer's
  # recorded promise to hold a named blob (provider = that peer); it is notarized
  # on the DHT and each peer keeps a local PROJECTION (a database table) of the
  # commitments it has seen. The "custody sweep" is the periodic pass on each
  # peer that compares its commitments against its local store; a "kick" is the
  # sweep's recovery action — a fetch it fires for a blob it promised to hold but
  # does not have. The "recovery window" is the drill's bound on that sweep
  # (E2E_CUSTODY_SWEEP_WINDOW_MS, default 300 s; the sweep itself runs every
  # 120 s by default). A "serve-blob" economic event is the REA record of which peer
  # supplied the bytes, so mutual aid inside a household is accounted, not
  # anonymous.
  #
  # Evidence anchor (2026-06-10 Phase 0, EPR durability arc): jessica's
  # projection sat at local_total=0 / ids_discovered=0 for 24h while matthew
  # held 16 DHT-anchored custody rows — convergence existed on paper only.
  # The RESET_STORAGE wipe in the genesis pipeline IS the backup-restore
  # drill; this feature makes the reconvergence half of it a promise.
  # Plan: genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md (Workstream B)

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @requires:owned-substrate
  Scenario: A wiped device recovers its stewarded content from the mesh
    Given human "Jessica" has a storage peer in the household mesh
    And a custody-blob commitment names Jessica's peer as provider for the manifesto blob
    And Jessica's peer serves the manifesto blob
    When Jessica's peer content store is wiped while the mesh stays up
    Then within the recovery window Jessica's peer serves the manifesto blob again
    And the custody sweep on Jessica's peer reports a kick for the manifesto blob
    And a "serve-blob" economic event records which household peer aided the recovery

  # HELD (2026-08-22, run 20260822T201747Z-3bd326d6): the inventory-blind
  # starting condition cannot be CONSTRUCTED on the household mesh. The gossip
  # inventory is the persisted `peer_blob_inventory` table, so a restart hands
  # Jessica back all 77 entries, and elohim-storage has no verb that clears it
  # (nor one that holds inventory receipt). The drill's own Given measures and
  # names the gap rather than passing against a peer that was never blind.
  # Sheds @wip when a destructive-gated inventory-reset verb exists and
  # steps/mesh/household-chaos.steps.ts calls it under the owned-substrate gate.
  @wip @requires:owned-substrate
  Scenario: Recovery does not depend on gossip inventory health
    Given Jessica's peer was restarted and its gossip inventory is empty
    And a custody-blob commitment names Jessica's peer as provider for the manifesto blob
    And the manifesto blob is missing from Jessica's local store
    When the custody sweep runs on Jessica's peer
    Then the sweep falls back to racing the connected household peers
    And the sweep outcome reports the kick as an inventory-blind fallback

  # HELD (2026-08-22, same run): needs a WIPED commitment projection on Jessica
  # (she holds 84 custody-blob rows), and elohim-storage has no verb to clear
  # one — no DELETE on /db/rea_commitments, no /admin projection-reset. The
  # Given measures and names that gap. Sheds @wip when a destructive-gated
  # projection-reset verb exists and the chaos steps establish the
  # precondition with it (and restore afterwards).
  @wip @requires:owned-substrate
  Scenario: A wiped peer's commitment projection reconciles from its own conductor
    Given Jessica's peer projection holds no custody-blob commitment rows
    And another household peer's projection holds the custody-blob commitment rows
    When the projection reconcile sweep runs on Jessica's peer
    Then Jessica's peer discovers the missing commitment ids from a household peer
    And each discovered commitment is healed from Jessica's own conductor
    And Jessica's peer projection reconcile reports caught up with a non-zero local total
