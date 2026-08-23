@e2e @dataplane @regression @requires:household-nodes @concern:iroh-dual-plane @act:i
Feature: Matthew's household actually exchanges data on its iroh plane
  Matthew should not have to know which network transport keeps his household's
  shared writings available. When his family nodes run in dual mode, iroh is
  meant to carry real discovery, gossip, and document-sync traffic alongside
  libp2p. A node merely opening an iroh listener is not resilience: the second
  plane must learn the other household nodes, form gossip neighborhoods, receive
  a message, and complete a sync request.

  The counters below are cumulative since Matthew's storage peer started. The
  ordinary sync-round counter is libp2p-only; every iroh-prefixed counter is
  advanced only by a live iroh action. Seeing both on the same peer proves dual
  participation and distinguishes a working second path from an idle endpoint
  that only reports an iroh NodeId.

  Background:
    Given peer "matthew" at "E2E_STORAGE_MATTHEW"

  # Temporal precondition: these counters are cumulative and are read after the
  # mesh lane's readiness gate — the peer has been running long enough for natural
  # discovery + at least one 60 s sync round; nothing here triggers traffic.
  Scenario: Matthew's peer shows live iroh traffic on every layer of the ladder
    Then metric "elohim_sync_rounds_total" on peer "matthew" >= 1
    # the household is 3 nodes; knowing the other 2 is full discovery, not a floor
    And metric "elohim_iroh_peers_known" on peer "matthew" >= 2
    # direction=up means a neighbor JOINED the iroh gossip neighborhood
    And labeled metric "elohim_iroh_gossip_neighbor_events_total" with label "direction" "up" on peer "matthew" >= 1
    And metric "elohim_iroh_sync_rounds_total" on peer "matthew" >= 1
    And labeled metric "elohim_iroh_sync_requests_total" with label "result" "ok" on peer "matthew" >= 1
    # transport-manifest gossip is how peers learn each other's iroh addresses —
    # receiving one proves another peer's announcement crossed the iroh plane
    And labeled metric "elohim_iroh_gossip_received_total" with label "topic" "elohim/transport/manifest" on peer "matthew" >= 1
