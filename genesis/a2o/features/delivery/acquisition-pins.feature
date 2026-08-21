@e2e @delivery @requires:doorway @act:i
Feature: Acquisition pins — the device pin and the pull queue
  A PIN is a person saying "keep this on my device." The device pin is the
  airplane-mode floor: declarable with no hub, no conductor, no peers — just
  the device and its own disk. The PULL QUEUE is what makes the promise true:
  it goes and gets the bytes.

  The one rule everything here turns on:

    A pin is satisfied by BYTE ARRIVAL — the bytes on this device's disk,
    readable — and never by INVENTORY ARRIVAL, some record saying they are
    somewhere. A status that reports "done" from the inventory alone is the
    system telling a person it holds something it does not.

  `epr:strawberry-guide` is the fixture content this story pins: one ordinary
  piece of published content, addressed the way all published content is
  (`epr:` + its id). Nothing about it is special — that is the point.

  These scenarios exercise the /api/v1/pins own-node API on elohim-storage
  directly. The pin API is never proxied through a doorway: a pin is a promise
  a device makes about itself, so it is always answered own-node.

  Design: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md
  (the provide loop: 2026-06-08-epr-acquisition-slice2b-provide-loop-design.md).
  The binding two-node byte-arrival regression is the Rust integration test
  elohim/elohim-storage/tests/acquisition_pull_e2e.rs.

  # Test environment: E2E_STORAGE_URL is peer A (default localhost:8090) and
  # E2E_STORAGE_JESSICA (or E2E_STORAGE_URL_B) is peer B. Scenarios tagged
  # @requires:owned-substrate write to, or restart, the mesh; each such step
  # answers 'skipped' unless the operator sets A2O_ALLOW_DESTRUCTIVE=1. A @wip
  # scenario is one whose step definitions are not wired to a real observation
  # yet — it is documentary until they are.

  Scenario: A pin is creatable and durable with no network at all
    When I POST a pin for "epr:strawberry-guide" to /api/v1/pins
    Then the pin response status is 201
    And GET /api/v1/pins lists one active pin for "epr:strawberry-guide"

  # A CLUSTER pin says "keep this on the whole cluster" — one declaration standing
  # in for whatever set of devices satisfies it. Resolving that set is not built,
  # so the node refuses rather than accepting a promise it cannot keep.
  Scenario: Cluster pins are honestly refused until the closure resolver lands
    When I POST a pin with kind "cluster" for "epr:cluster-target" to /api/v1/pins
    Then the pin response status is 501
    And the pin response body mentions "slice-3"

  # WRITES: pinning on peer B enqueues a real acquisition and moves real bytes.
  # Every mutating step answers 'skipped' unless A2O_ALLOW_DESTRUCTIVE=1.
  @requires:household-nodes @requires:owned-substrate
  Scenario: A pin completes only when verified bytes land on disk
    Given two connected storage peers where only peer A holds "epr:strawberry-guide"
    When peer B pins "epr:strawberry-guide"
    And the pull queue drains
    Then peer B's pull status shows fetched 1 of total 1
    And peer B's local projection carries the row at the moment of that claim
    # BORN RED, measured 2026-08-21 on the household mesh: peer B's rollup answered
    # {total:1, fetched:1, caughtUp:true} while asking that same peer for the content
    # returned "not found"; the row appeared about 20 seconds later. So "caught up" is
    # published from the fetch ledger alone, ahead of the store it is supposed to be
    # reporting on — inventory arrival wearing byte arrival's clothes. It matters to a
    # person, not just to a test: a device that pins a guide, watches the pin go green,
    # and opens it is told the guide is not there.
    # The last step is deliberately read AT the moment of the claim, which is why its
    # wording says so. When the row lands late it still fails, and names how late — a
    # settle window here would launder the gap into a pass.

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

  @requires:household-nodes @requires:owned-substrate
  Scenario: A peer that pins-as-peer serves the bytes to a second node
    Given two connected storage peers where only peer A holds "epr:strawberry-guide"
    When peer A pins "epr:strawberry-guide" as a peer
    And peer B pins "epr:strawberry-guide"
    And the pull queue drains
    Then peer B's pull status shows fetched 1 of total 1
    And peer B fetched the bytes from peer A
    # Bounded honestly: nothing records WHICH peer supplied a given blob, so the last
    # step asserts the two things that are observable — peer B now serves the bytes
    # itself, and peer A was a connected peer able to supply them. Traced provenance
    # would need a surface that does not exist.

  # --- Acquisition constraint regressions -----------------------------------
  # Harvested from the 2026-07-26 resiliency-saga overnight cure sprint, where
  # both defects below sat inside an eight-deep chain of stacked acquisition defects: each
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

  # RESTARTS a storage peer: the only way to observe a process before its first
  # reconcile. Held behind A2O_ALLOW_DESTRUCTIVE=1, answering 'skipped'.
  @requires:owned-substrate @regression
  Scenario: Pull status distinguishes an unmeasured boot from an observed empty pin set
    Given a storage peer has restarted and no acquisition reconcile has completed
    When I GET /p2p/status
    Then the pull status is null
    When the first acquisition reconcile completes with zero active pins
    Then the pull status reports total 0, fetched 0, pending 0, and failed 0
    And the acquisition reconcile initialized metric is 1
    And the acquisition active pins metric is 0
    And the acquisition reconcile outcome "completed" was counted
    # Constraint: unreadable/unmeasured is not zero. `pull: null` is the boot and
    # early-return answer; a non-null zero rollup is evidence that the local
    # active-pin × presence census actually completed and found nothing.
    # Binding contracts: p2p/acquisition.rs::pull_is_null_until_a_reconcile_observes_the_desired_set,
    # metrics.rs::acquisition_reconcile_outcomes_are_stable_pretouched_and_incrementable,
    # tests/schema_contract.rs::p2p_status_view_with_null_drain_and_uninitialized_pull.
    # Operational parameters: first tokio interval tick at t=0, then every 60s;
    # the completed first pass refreshes /p2p/status immediately.
    # Review after: changes to acquisition cadence, status refresh, or any new
    # early-return branch in run_acquisition_reconcile.
