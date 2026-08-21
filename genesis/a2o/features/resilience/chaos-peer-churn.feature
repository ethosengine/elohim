@e2e @resilience @chaos @concern:blob-durability @dataplane @act:i
Feature: Chaos peer churn — the dataplane proves itself by surviving us
  As a steward of a network that promises durability through people
  I want peers killed, flapped, and cascaded on purpose, during reads
  So that resilience is something we demonstrate on demand, not a claim
  we hope holds the night a real disk dies

  # D9 of the resilience dimensions proof suite (spec:
  # 2026-06-12-resilience-dimensions-proof-suite-design.md).
  #
  # Blast radius, before anything else: every peer named here is a dedicated
  # drill fixture on a mesh this suite owns. Running this file destroys no
  # participant's custody. Jessica is a fixture household human, not a person
  # whose disk we are gambling with.
  #
  # Layer split:
  # - The protocol-level chaos primitives (dead provider fails fast, failover
  #   to survivor, churn rebirth, bounded mid-transfer kill) are pinned
  #   deterministically with REAL libp2p nodes in
  #   elohim-storage/tests/chaos_dataplane.rs, which run in CI.
  # - CRDT-plane offline/merge churn: tests/sync_integration.rs (offline merge).
  # - The rows below are the LIVE drills: they kill actual running peers.
  #   Actuation rail: drill bash lives in genesis/scripts/ci/ (genesis
  #   Jenkinsfile is CPS-capped — one thin stage call), and destructive
  #   peer-ops are operator-ratified per the EPR durability arc plan; verify
  #   the current ratification before wiring a new drill.
  # - Static single-loss shapes (one peer down, returning re-sync, adjacency,
  #   single-device floor) are federation/peer-loss-failover.feature — not
  #   duplicated here. This file owns the DYNAMIC shapes: flapping, cascade,
  #   mid-read kill, simultaneous loss.

  # The words the assertions below lean on, fixed here so every rung is
  # checkable without leaving the file:
  #   UNDER CUSTODY — a peer has promised to hold a copy of something, and the
  #     mesh RECORDS that promise. What the mesh believes about who holds what
  #     is exactly what churn must not corrupt.
  #   SERVING CREDIT — a separate ledger from the custody record: when a peer
  #     actually hands over bytes to rescue someone else's read, that work is
  #     booked to it. Custody is a promise kept on disk; serving credit is
  #     labour recognized. A file about durability through people has to prove
  #     both, and they are not the same entry.
  #   NEIGHBOURS    — the household mesh is three peers, so a joined peer sees
  #     2 of them. That is a health threshold: falling below it is the harm,
  #     never the goal. When a scenario says a peer "sees its 2 neighbours" it
  #     is claiming health; when it says a peer "reports no connected peers" it
  #     is claiming honest degradation.
  #
  # What this mesh's "independent" is, honestly: the three peers are separate
  # storage processes with separate stores, not separate hosts or disks. So a
  # copy count here proves the mesh's BOOKKEEPING and its FAILOVER, and it does
  # not prove survival of one machine's power supply. Physical independence is
  # a property of a deployed fleet, and the rungs below are careful not to
  # claim it.
  #
  # And the doorway is not a fourth copy: it holds no bytes of its own and must
  # fetch from a custody peer, which is what makes "still returns the content"
  # after a kill mean anything at all.
  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the household mesh is three storage peers, each a dedicated drill fixture
    And a household peer counts as joined only while it sees 2 neighbours
    And the household protection status reads "protected" at 3 custody copies, "partial" at 2, and "at-risk" at 1

  @requires:owned-substrate
  Scenario: A flapping peer never corrupts what the mesh believes
    Given content "manifesto" is under custody on every household peer
    And every household peer sees its 2 neighbours
    And the mesh's custody record for "manifesto" is settled
    When Jessica's storage peer is killed and restarted 3 times, 30 seconds apart
    Then within 90 seconds of the final return Jessica's peer sees its 2 neighbours again
    And Jessica's peer holds the same inventory as the rest of the mesh
    And the mesh records exactly one custody copy of "manifesto" per household peer
    And no custody record names a peer that no longer holds the bytes
    # Flapping is the realistic failure (sleepy laptops, flaky power) — churn
    # must be idempotent, not merely survivable once. The settled-record Given
    # is a quiescence precondition: it makes sure the mesh had finished making
    # up its mind about "manifesto" BEFORE the churn, so a duplicate or a
    # phantom found afterwards is attributable to the churn and not to a sweep
    # that was still mid-flight when the drill started.

  @requires:owned-substrate
  Scenario: Cascading peer loss degrades the protection status honestly, step by step
    # The ladder, stated once so each rung can fail for the right reason:
    # protected = 3 independent peers hold a copy, partial = 2, at-risk = 1.
    Given content "chaos-ladder" is under custody on all 3 household peers
    And the household protection status for "chaos-ladder" reports "protected"
    When one custody peer is killed
    Then within 60 seconds the household protection status for "chaos-ladder" reports "partial"
    And fetching "chaos-ladder" through the doorway still returns the content
    When a second custody peer is killed, leaving one
    Then the household protection status for "chaos-ladder" reports "at-risk", not "protected" and not silence
    And the surviving peer still serves the content
    # The D1 ladder, traversed live and downward: what a household is told
    # must stay true at every step of a real cascade, and reads outlive the
    # label — service degrades later than the badge does, and saying so is the
    # point.

  @browser-only @requires:owned-substrate
  Scenario: A read in flight when its serving peer dies still completes
    Given content "manifesto" is under custody on every household peer
    And a household member is loading "manifesto" through the doorway, large enough that the transfer is still in flight a second later
    When the peer serving that read is killed mid-transfer
    Then the load completes from another custody peer without the member doing anything
    And the household peer that supplied the rescue bytes is credited for serving them
    # The protocol-level bound (bytes-or-error, never a hang, survivor
    # completes) is pinned in chaos_dataplane.rs; this row proves the full
    # stack — doorway, race-fetch, and the credit a peer earns for the rescue —
    # keeps the promise. Durability through people only works if the people
    # who do the rescuing are recognized for it.

  # RE-AIMED (2026-08-21, act layering). This row used to end at "reconverges
  # without an operator". Three peers minus two leaves ONE, and a lone survivor
  # cannot tell reconvergence apart from never having had a neighbour: every
  # post-return assertion would pass on a mesh that healed nothing. So the row
  # now asserts what three peers CAN discriminate — honest degradation. The
  # reconvergence half needs a fourth peer before it can fail for the right
  # reason; it is left unasserted here rather than asserted vacuously.
  @requires:owned-substrate
  Scenario: Simultaneous loss of two peers leaves the survivor degrading honestly
    Given content "manifesto" is under custody on every household peer
    And every household peer sees its 2 neighbours
    When two storage peers are killed at the same moment
    Then the surviving peer reports no connected peers rather than a stale count
    And the surviving peer still serves "manifesto" from its own store
    And the household protection status for "manifesto" reports "at-risk"
    # Putting the two peers back is TEARDOWN, and it is deliberately not a rung
    # here: an "everyone sees 2 neighbours again" line at the end of this
    # scenario is exactly the post-return assertion the note above says three
    # peers cannot make honestly, and a reader of the run report would take it
    # for one. The restore happens in the step file's After hook instead, so
    # the mesh is handed back intact without the drill claiming it healed.
