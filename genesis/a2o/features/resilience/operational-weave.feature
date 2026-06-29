@e2e @resilience @resilience-p1 @local @concern:keyspace-coverage @dataplane
Feature: Operational weave lens — cluster-scoped capacity eyes
  As an operator running a household mesh
  I want a single cluster-wide weave view (placement gaps, RS coverage, capacity)
  So I can plan capacity and trust the numbers degrade honestly, never falsely

  # Surfaced 2026-06-20 by story-harvest on the operational-weave lens branch finish
  # (Wave A of the Weave Epic). The lens is a read-only Operational-C facing:
  # GET /api/v1/weave + Prometheus gauges, folded once from existing tables.
  # Charter: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md
  # These are @wip scaffolds — step definitions follow (story-harvest captures the
  # discovered CONSTRAINTS, not finished acceptance tests).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  # --- Baseline: the lens lights -------------------------------------------

  @wip
  Scenario: The weave view reports cluster health from existing relations
    When the operator requests "/api/v1/weave"
    Then the response includes "placementGapCount" as an integer
    And the response includes "rsCoverage" between 0 and 1
    And the response includes "clusterCapacity" with "free", "used", "stewarded"
    And the response includes "measuredAt"

  # --- #1 Cluster-vs-app scoping (the undercount regression) ----------------

  @wip @regression
  Scenario: The weave gap count is cluster-wide, not scoped to one app
    Given placement gaps exist for content in more than one app/reach scope
    When the operator requests "/api/v1/weave"
    Then "placementGapCount" equals the TOTAL gap rows across all scopes
    And it is strictly greater than the count for any single app scope
    # Constraint: a cluster/node-scoped projection MUST load gaps unscoped
    # (load_all_placement_gaps, no h_app_id filter). Scoping to one app
    # (the app-scoped list_gaps) silently UNDERCOUNTS the node-wide gauge/view.
    # Regression anchor: if this ever passes with an app-scoped query, the
    # cluster eyes are lying by omission.

  # --- #2 Unsampled-node-doesn't-zero (graceful degradation) ----------------

  @wip @regression
  Scenario: An unsampled custodian does not zero the cluster capacity
    Given at least one custodian has reported a storage sample
    And at least one other custodian has NO current system_metrics sample
    When the operator requests "/api/v1/weave"
    Then "clusterCapacity.free" reflects the sampled custodian(s)
    And it is NOT zero solely because an unsampled custodian was present
    # Constraint: aggregate_capacity does per-field Option-sum — a node with
    # None for a field is SKIPPED, never summed as zero. An unsampled custodian
    # degrades the aggregate gracefully (smaller denominator), never falsely zeros it.
    # Operational parameter: cluster capacity reads are sample-population-relative;
    # informs how operators read "free" during partial-mesh / cold-start windows.

  # --- #3 Not-selected-field contract: absent != zero ----------------------

  @wip
  Scenario: A deferred lens field is absent from the response, not zero
    When the operator requests "/api/v1/weave"
    Then the response does NOT contain the key "tierOccupancy"
    And the response does NOT contain the key "regionOccupancy"
    # Constraint: a facing carries ONLY the lenses it selected. tierOccupancy and
    # regionOccupancy are deferred (backlog operational-weave-tier-region-occupancy.md);
    # they are OMITTED on the wire (skip_serializing_if + ts(optional)), never serialized
    # as 0 or {}. A reader must distinguish "not computed" from "measured zero".

  # --- #4 Dual-projection: the gauge and the view cannot disagree ----------

  @wip
  Scenario: The Prometheus gauge equals the view's placement-gap count
    When the operator requests "/api/v1/weave"
    And the operator scrapes "/metrics"
    Then "elohim_placement_gap_count" in /metrics equals "placementGapCount" in the weave view
    # Constraint: one pure fold -> two wire shapes. The gauge is set in the adapter
    # (build_weave_view / emit_placement_gap_gauge) from the SAME fold output that
    # builds the view, so the JSON and the gauge cannot drift apart.

  # --- #5 agent_cid join key (inherited guard; see existing coverage) -------
  # The lens joins custodian_metrics.custodian_id == rea_commitments.provider == agent_cid
  # (never raw-string-compare across identity namespaces — the all-zeros-card trap).
  # This guard is already exercised by the commitment-backed / observable-distribution
  # coverage (the card-zeros regressions); the weave lens inherits it. No new scenario —
  # noted here so a future identity-namespace change re-checks those features too.
