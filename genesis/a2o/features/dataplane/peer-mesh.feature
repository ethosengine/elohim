@e2e @dataplane @concern:peer-mesh @requires:multi-node @act:i
Feature: Peer mesh connectivity
  The alpha deployment forms a connected P2P mesh: peers announce themselves via the DHT,
  establish direct libp2p connections, and the storage projector reconciles anchor divergence
  within a finite window. These scenarios lock in the live baseline as regression proofs —
  if they pass the mesh is healthy enough to serve the gap-first suites.

  The @requires:multi-node tag signals that these scenarios require at least two live peers.
  It is a fixture precondition (not a cluster-state gate) so the scope reconciler ignores it;
  the scenarios run whenever the alpha endpoints are reachable. Alpha confirmed 13 connected
  peers on 2026-06-29.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: alpha-A is connected to the mesh and projection is caught up
    # peerCount >= 2 confirms DHT-mediated peer discovery has run and at least one
    # additional peer has established a live libp2p connection.
    # p2p.caughtUp true confirms the storage projector has reconciled its anchor queue.
    Then peer "alpha-A" /health peerCount >= 2
    And peer "alpha-A" /health p2p.caughtUp is true

  Scenario: elohim.host is connected to the mesh and projection is caught up
    # elohim.host is the alpha-b federation peer sharing the same P2P network as alpha-A.
    # Both doorways must be independently healthy for the mesh to be considered live.
    Then peer "elohim.host" /health peerCount >= 2
    And peer "elohim.host" /health p2p.caughtUp is true

  Scenario: alpha-A reconcile anchor divergence is within a tolerated bound
    # divergentAnchor counts DHT anchors the gossip reconciler found diverged on the last
    # reconcile pass. A value <= 100 confirms the mesh is converging and there is no
    # runaway divergence. Observed values on 2026-06-29: 6 (alpha-A), 0 (elohim.host).
    Then peer "alpha-A" /health divergentAnchor <= 100
