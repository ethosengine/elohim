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
    Given peer "matthew" at "matthew"

  Scenario: Matthew's peer uses iroh to participate in the household mesh
    Then metric "elohim_sync_rounds_total" on peer "matthew" >= 1
    And metric "elohim_iroh_peers_known" on peer "matthew" >= 2
    And labeled metric "elohim_iroh_gossip_neighbor_events_total" with label "direction" "up" on peer "matthew" >= 1
    And metric "elohim_iroh_sync_rounds_total" on peer "matthew" >= 1
    And labeled metric "elohim_iroh_sync_requests_total" with label "result" "ok" on peer "matthew" >= 1
    And labeled metric "elohim_iroh_gossip_received_total" with label "topic" "elohim/transport/manifest" on peer "matthew" >= 1
