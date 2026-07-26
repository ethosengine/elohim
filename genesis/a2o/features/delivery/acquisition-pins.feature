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

  # --- Acquisition constraint regressions -----------------------------------
  # Harvested from the 2026-07-26 resiliency-saga overnight cure sprint, where
  # both defects below sat INSIDE the eight-deep chapter-5 deadlock chain: each
  # was invisible until the one above it was cured, and neither announced itself
  # — acquisition simply went quiet. These scenarios name the constraints so the
  # quiet has a witness.
  #
  # The BINDING regressions are the Rust unit tests (same convention as the
  # two-node scenarios above — cucumber cannot yet stand up a 6-peer fabric):
  #   * rotation: elohim/elohim-storage/src/p2p/mod.rs, mod acquisition_rotation_tests
  #     (successive_retries_of_a_stable_position_walk_distinct_peers)
  #   * budget:   elohim/elohim-storage/src/p2p/reconcile_rails.rs
  #     (dispatch_budget_caps_inflight) + the ShardResponse::Error release arm in
  #     p2p/mod.rs
  # These cucumber scenarios are the human-facing statement of the same
  # constraints, scaffolded pending until a household-scale acquisition fixture
  # exists.

  @requires:household-nodes @wip @regression
  Scenario: Retries of one item probe distinct peers, never one peer three times
    Given an acquisition fabric of 6 connected peers
    And "epr:elohim-host-landing" sits at a stable position in the acquisition batch
    When its acquisition retries until the retry budget is exhausted
    Then the item was probed on 3 distinct peers
    # THE DEFECT: peer choice was `batch_position % peer_count`. The acquisition
    # queue is rebuilt every 60s reconcile from list_active_pins in stable DB
    # order, so an item's batch position is STABLE — meaning all 3 retries went to
    # the SAME peer, and the item exhausted its budget having probed 1/6 of the
    # fabric. Live alpha read pull:{total:36, fetched:0, failed:36}.
    # Operational parameters: MAX_RETRIES=3 (p2p/acquisition.rs), 6-peer fabric,
    # 60s reconcile cycle (retry-on-NEXT-cycle, not immediate re-queue).
    # Informs: minimum peer diversity for a pin to be considered fairly probed —
    # a fabric with fewer peers than MAX_RETRIES cannot rotate through distinct
    # providers, which is a real operator-facing floor.
    # Review after: any change to MAX_RETRIES, the reconcile cadence, or the
    # queue's ordering guarantee (stable DB order is load-bearing here).

  @requires:household-nodes @wip @regression
  Scenario: An error-class shard response releases its in-flight dispatch slot
    Given an acquisition fabric of 6 connected peers
    When 25 acquisition dispatches are answered with an error-class shard response
    Then the available dispatch budget returns to 25
    And a subsequent acquisition drain dispatches at least one request
    # THE DEFECT: only the success arm removed the entry from
    # pending_acquisition_fetches; a ShardResponse::Error leaked its slot. After
    # exactly MAX_ACQUISITION_INFLIGHT errors the budget's available() returned 0
    # and drain_acquisition_queue returned early on EVERY subsequent tick —
    # acquisition wedged permanently, silently, with no error and no log line.
    # Operational parameters: MAX_ACQUISITION_INFLIGHT=25 (p2p/acquisition.rs),
    # 6-peer fabric, 25 error responses = full wedge.
    # Informs: the in-flight ceiling is a HARD wedge boundary, not a soft
    # throttle — any new response arm on this path must release its slot, and any
    # future budget tuning changes how many failures it takes to wedge.
    # Review after: new ShardResponse variants, changes to
    # MAX_ACQUISITION_INFLIGHT, or any new caller of DispatchBudget.
