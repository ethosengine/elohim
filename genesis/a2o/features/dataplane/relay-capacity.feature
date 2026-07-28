@e2e @dataplane @concern:relay-capacity @requires:multi-node
Feature: TURN relay capacity bounds doorway-operator serviceable scale
  # HARVESTED 2026-07-28 (story-harvest, two-premises TURN shakeout).
  #
  # THE CONSTRAINT: a premises' TURN relay port pool is a hard ceiling on how many
  # concurrent relayed peer-links that premises can service. Each live allocation holds
  # one UDP port from min-port..max-port until the watchdog reaps it. When the pool is
  # exhausted, coturn answers ALLOCATE with error 508 "Cannot create socket"; the peer
  # gets no XOR-RELAYED-ADDRESS, the data channel never forms, and gossip rounds die at
  # stage Initiated — which reads downstream as "sync is broken", not "router is small".
  #
  # MEASURED PARAMETERS (operations leg, 41-port pool 49160-49200, 7 conductors + 2
  # doorways on the fleet):
  #   2026-07-28 single-legged era: 29 ALLOCATE ok vs 31 error-508 over 1h  (52% fail;
  #     an earlier pass measured 65%)
  #   2026-07-28 two-legged era (shem 3478 opened, real relay demand on both legs):
  #     104 ALLOCATE attempts, 88 error-508 over 1h (85% fail) on operations;
  #     0 error-508 on the shem leg the same hour (18 attempts)
  #   Correlated: adam gossip-round timeout ratio ~85% (213 timed out / 250 initiated / 1h)
  #
  # THE OPERATOR-DIVERSITY DIMENSION (why this is a preset parameter, not just a config):
  # pool size is bounded by what the operator's ROUTER can forward. Consumer routers
  # differ in kind: GFiber's Google-account UI cannot edit an existing forward range at
  # all (resetting the router is the only path), while prosumer gear forwards arbitrary
  # ranges. So "how many peers can this doorway premises service" is a function of
  # router class — a premises-tier capability, alongside CPU/RAM/disk, that peer
  # diversity presets and operator docs must carry.
  #
  # Informs: NodeCapabilities / operator presets (relay_port_pool size per premises
  # tier), the coturn manifests' operator port-forward blocks
  # (genesis/orchestrator/manifests/infra/alpha-coturn-{operations,shem}.yaml), and
  # the seam-map device-spectrum (router class is part of the premises tier).
  # Review after: fleet size changes, coturn lifetime tuning, arc-factor changes
  # (less relay demand), or any router swap at either premises.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  @wip
  Scenario: Relay pool absorbs the fleet's concurrent allocation demand on both legs
    # Capability proof — born red on the operations leg (85% 508s measured 2026-07-28).
    # Green means: the pool (however sized) is not the binding constraint on convergence.
    When TURN allocation outcomes are sampled on each premises leg for one hour
    Then the ALLOCATE error-508 rate on leg "operations" is below 1%
    And the ALLOCATE error-508 rate on leg "shem" is below 1%

  @wip
  Scenario: Gossip round completion is not starved by relay exhaustion
    # The downstream truth the pool serves. 85% timeouts co-occurred with 85% 508s;
    # this station separates "transport starved" from other convergence failures.
    When gossip round outcomes are sampled on peer "adam" for one hour
    Then the initiated-round timeout ratio is below 20%

  @wip
  Scenario: An operator can see per-leg relay-pool saturation
    # Observability gate — the diagnostic that made this discovery possible was log
    # archaeology (grep error-508 in coturn logs). An operator sizing a premises needs
    # the saturation signal surfaced, not excavated: allocations in use vs pool size,
    # and the 508 rate, per coturn leg.
    When Matthew inspects relay health for premises "operations"
    Then he sees the relay pool size, current allocations in use, and the ALLOCATE failure rate
