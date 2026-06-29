@e2e @resilience @chaos @concern:blob-durability @dataplane
Feature: Chaos peer churn — the dataplane proves itself by surviving us
  As a steward of a network that promises durability through people
  I want peers killed, flapped, and cascaded on purpose, during reads
  So that resilience is something we demonstrate on demand, not a claim
  we hope holds the night a real disk dies

  # D9 of the resilience dimensions proof suite (spec:
  # 2026-06-12-resilience-dimensions-proof-suite-design.md).
  #
  # Layer split:
  # - The protocol-level chaos primitives (dead provider fails fast, failover
  #   to survivor, churn rebirth, bounded mid-transfer kill) are pinned
  #   deterministically with REAL libp2p nodes in
  #   elohim-storage/tests/chaos_dataplane.rs — they run in CI today.
  # - CRDT-plane offline/merge churn: tests/sync_integration.rs (offline merge).
  # - The rows below are the LIVE-CLUSTER drills: they kill actual pods.
  #   Actuation rail: drill bash lives in genesis/scripts/ci/ (genesis
  #   Jenkinsfile is CPS-capped — one thin stage call), and destructive
  #   pod-ops are operator-ratified per the EPR durability arc plan; verify
  #   the current ratification before wiring a new drill.
  # - Static single-loss shapes (one peer down, returning re-sync, adjacency,
  #   single-device floor) are federation/peer-loss-failover.feature — not
  #   duplicated here. This file owns the DYNAMIC shapes: flapping, cascade,
  #   mid-read kill, simultaneous loss.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @wip
  Scenario: A flapping peer never corrupts what the mesh believes
    Given the manifesto blob is replicated under custody across the household mesh
    When Jessica's storage peer is killed and restarted 3 times in succession
    Then after the final return Jessica's peer meets its connected-peers floor
    And Jessica's peer inventory parity matches the mesh
    And no duplicate or phantom custody rows exist for the manifesto blob
    # Flapping is the realistic failure (sleepy laptops, flaky power) —
    # churn must be idempotent, not merely survivable once.

  @wip
  Scenario: Cascading peer loss degrades the protection status honestly, step by step
    Given content "chaos-ladder" is stewarded by 3 households with 2 peers online
    And "/api/v1/resilience/chaos-ladder/household" reports "protected"
    When one stewarding household's peers are killed
    Then the protection status degrades to "partial" within the status window
    And fetching "chaos-ladder" through the doorway still returns the content
    When all remaining stewarding peers but one are killed
    Then the protection status reports "at-risk" — not "protected", not silence
    And the surviving peer still serves the content
    # The D1 ladder, traversed live and downward: the felt icon must tell
    # the truth at every step of a real cascade, and reads outlive the label.

  @wip @browser-only
  Scenario: A read in flight when its serving peer dies still completes
    Given a household member is loading a large content blob through the doorway
    When the peer serving that read is killed mid-transfer
    Then the load completes from a surviving custody peer without user action
    And the rescue is visible as a serve-blob economic event for the survivor
    # The protocol-level bound (bytes-or-error, never a hang, survivor
    # completes) is pinned in chaos_dataplane.rs; this row proves the full
    # stack — doorway, race-fetch, REA recognition — keeps the promise.

  @wip
  Scenario: Simultaneous loss of two peers reconverges without an operator
    Given the household mesh is at its connected-peers floor with custody in place
    When two storage peers are killed at the same moment
    And both peers return after the mesh has re-stabilized
    Then within the re-sync window every returning peer meets its connected-peers floor
    And the custody sweep reports reconvergence with no manual kicks
    And mesh adjacency is bidirectional for every household peer again
    # Correlated failure (a breaker trips, a router reboots) is the household
    # norm — recovery must be a property of the substrate, not a runbook.
