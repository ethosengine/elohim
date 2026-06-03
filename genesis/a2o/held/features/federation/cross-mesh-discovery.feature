@step-zero @cross-mesh @phase-1-federation @requires:shem @requires:alpha-cluster-6peer
Feature: Cross-mesh DHT discovery survives the doorway-A / doorway-B partition

  The federation-wiring-audit Phase 1 split the alpha cluster's signaling
  mesh: matthew/jessica/james register at signal.doorway-alpha.elohim.host;
  the 11 remote personas (adam, pete, terrance, frank, gertrude, susan, caleb,
  daniel, emma, eve, nancy) register at signal.elohim.host. The substrate
  step-zero gossip (`elohim/conductor/agent-info/v1`) propagates each pod's
  Holochain AgentInfoSigned over the libp2p mesh (which is already full-mesh
  cross-doorway via P2P_BOOTSTRAP_NODES), so every conductor's peer cache
  learns about every other peer regardless of which signal server they
  registered with.

  These scenarios run against the alpha cluster's a2o pipeline once the
  ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP flag is on cluster-wide.

  Background:
    Given the alpha cluster has 14 humans deployed with per-human primary doorway routing
    And matthew, jessica, james are registered with signal.doorway-alpha.elohim.host
    And adam plus 10 others are registered with signal.elohim.host
    And the ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP feature flag is on for every pod

  @seeder @substrate-replication
  Scenario: Seeded content lands on one peer and reaches the cross-mesh half
    Given the seeder writes Matthew's AccountPackage to elohim-matthew-alpha (hash-mod primary)
    When the seeder completes the Matthew package import
    Then within 30 seconds adam's conductor can DHT-resolve Matthew's ContributorPresence
    And within 30 seconds pete's conductor can DHT-resolve Matthew's identity binding

  @signal-failure @resilience
  Scenario: Signal-server-A goes down mid-session and the cross-mesh half stays reachable
    Given the cluster is in steady state with all conductor peer caches warm
    When signal.doorway-alpha.elohim.host becomes unreachable
    Then existing inter-peer DHT operations continue to complete for at least 5 minutes
    And adam can still DHT-resolve content authored by matthew via cached peer info
