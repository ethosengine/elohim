@e2e @delivery @requires:doorway
Feature: Acquisition pins — the device pin and the pull queue (slice 1)
  The device pin is the airplane-mode floor (spec §1.1): declarable with no
  hub, no conductor, no peers. The pull queue satisfies pins by byte-arrival,
  never inventory-arrival (R-A).

  These scenarios exercise the /api/v1/pins own-node API on elohim-storage
  directly (E2E_STORAGE_URL, default localhost:8090). The @requires:doorway
  tag signals that a running elohim-storage instance is needed; the doorway
  background step is omitted because the pin API is not proxied through the
  doorway — it is always own-node.

  The binding two-node byte-arrival regression is the Rust integration test
  elohim/elohim-storage/tests/acquisition_pull_e2e.rs — see spec §11.

  Scenario: A pin is creatable and durable with no network at all
    When I POST a pin for "epr:strawberry-guide" to /api/v1/pins
    Then the pin response status is 201
    And GET /api/v1/pins lists one active pin for "epr:strawberry-guide"

  Scenario: Cluster pins are honestly refused until the closure resolver lands
    When I POST a pin with kind "cluster" for "epr:cluster-target" to /api/v1/pins
    Then the pin response status is 501
    And the pin response body mentions "slice-3"

  @requires:household-nodes @wip
  Scenario: A pin completes only when verified bytes land on disk
    Given two connected storage peers where only peer A holds "epr:strawberry-guide"
    When peer B pins "epr:strawberry-guide"
    And the pull queue drains
    Then peer B's pull status shows fetched 1 of total 1
    And the content row exists in peer B's local projection

  # --- Provide loop (slice 2b, rung 4): pin-as-peer + per-EPR pull rollup ---
  # The provide pin (provide:true) makes a caught-up commons pin a serveable
  # peer provider. The own-node pull-rollup route GET /api/v1/pins/{eprId}/pull
  # backs the PinProgressComponent. Both legs are own-node and runnable on a
  # single household node — no second peer needed.

  Scenario: A provide pin is accepted and reports a pull rollup
    When I POST a provide pin for "epr:strawberry-guide" to /api/v1/pins
    Then the pin response status is 201
    And GET /api/v1/pins/strawberry-guide/pull reports a pull rollup

  Scenario: A provide pin is refused on a browser-only context
    When I POST a provide pin with a forced browser context for "epr:strawberry-guide"
    Then the pin response status is 400
    And the pin response body mentions "peer"

  @requires:household-nodes @wip
  Scenario: A peer that pins-as-peer serves the bytes to a second node
    Given two connected storage peers where only peer A holds "epr:strawberry-guide"
    When peer A pins "epr:strawberry-guide" as a peer
    And peer B pins "epr:strawberry-guide"
    And the pull queue drains
    Then peer B's pull status shows fetched 1 of total 1
    And peer B fetched the bytes from peer A
