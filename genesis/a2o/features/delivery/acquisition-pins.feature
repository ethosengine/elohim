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
