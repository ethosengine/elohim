@e2e @deployment @substrate-floor @resource-tunable @wip
Feature: Node resource limits are tunable, bounded, and documented
  As an operator watching runaway-resource issues recur across the stack
  I want every node resource limit to be a named, per-node tunable with a safe
  default and a documented bound
  So that I can specifically tune the parameter that is exhausting, and so the
  shape of where exhaustion happens is reportable rather than folklore

  This is the "document the limit" half of the tune→document→report loop. The
  numeric truth lives in a Rust boundary #[test] (the source of truth); this
  story references those tests by name rather than re-encoding the numbers, the
  same discipline resilience-dimensions.feature uses against
  elohim-storage/tests/household_resilience.rs. The shape report
  (genesis/a2o/scripts/build-resource-shape-report.ts) aggregates breaches of
  these limits by node and class.

  See: docs/superpowers/specs/2026-06-15-node-resource-tunables-and-exhaustion-shape-design.md

  # --- The catalogue of node resource tunables ---

  Scenario Outline: Each node resource limit is a named per-node tunable with a safe default
    Given the elohim-node container is deployed from the consolidated template
    When an operator sets <knob> via <surface>
    Then the running node applies <value-rendered>
    And leaving it unset renders the <default> (no behavior change)
    And the bound is pinned by the Rust boundary test <guarding-test>

    Examples:
      | knob                 | surface                              | env / field                 | default | guarding-test                                                  |
      | SQLite read pool     | deployments.json edgenodeDbPoolSize  | STORAGE_DB_POOL_SIZE        | 10      | db::pool_size_tests::unset_uses_the_default                    |
      | conductor arc factor | conductor-config network block       | ELOHIM_TARGET_ARC_FACTOR    | 1       | arc_actuator::render_inserts_target_arc_factor_into_network_block |
      | edgenode CPU limit   | deployments.json edgenodeCpuLimit    | (k8s resources.limits.cpu)  | per-archetype | (manifest-rendered; validate:deployments gates the field)  |
      | edgenode memory limit| deployments.json edgenodeMemoryLimit | (k8s resources.limits.memory) | per-archetype | (manifest-rendered; validate:deployments gates the field) |

  # --- Bounded behaviour: the default must never be a footgun ---

  @regression
  Scenario: A zero or garbage SQLite pool size falls back to the default, never a zero-size pool
    # Constraint (A1, spec 2026-06-15): a zero-size r2d2 pool would deadlock every
    # pool.get(); STORAGE_DB_POOL_SIZE must therefore reject 0/garbage and fall back
    # to the historical hardcoded value, so an env typo can never wedge the node.
    # Operational parameters: STORAGE_DB_POOL_SIZE default 10; 0/non-numeric → 10.
    # Guarded by: db::pool_size_tests::zero_falls_back_to_the_default,
    #             db::pool_size_tests::garbage_falls_back_to_the_default
    Given a node with STORAGE_DB_POOL_SIZE set to "0"
    When the storage pool initialises
    Then the pool size is 10, not 0
    And a valid override like "20" is honored

  @regression
  Scenario: The conductor arc factor is bounded to {0,1} — fractional is refused, not silently clamped
    # Constraint (A2, spec 2026-06-15): kitsune2 / holochain_conductor_api type
    # target_arc_factor is u32 honoring {0,1} only (full arc vs leecher). Any value
    # outside that set is refused as NotActuatable — fractional arc (<1) is blocked
    # upstream until kitsune2 sharding lands, so the knob must refuse it loudly.
    # The deploy-time wiring of ELOHIM_TARGET_ARC_FACTOR is folded into the iroh
    # transport sprint (it shares the main.rs transport-config boot path); the BOUND
    # is already pinned here.
    # Operational parameters: target_arc_factor ∈ {0,1}; default 1; genesis bootstrap
    # peers (matthew) must stay 1 — the coverage floor refuses dropping them to 0.
    # Guarded by: arc_actuator (render_conductor_arc_factor refuses value ∉ {0,1})
    Given a request to set the conductor arc factor to a fractional value
    When the arc actuator renders the conductor config
    Then the actuation is refused as NotActuatable
    And only 0 (leecher) or 1 (full arc) are accepted

  # --- The reportable shape ---

  Scenario: Breaches of these limits aggregate into the resource-shape report
    Given self-heal backlog entries record where limits were breached, by node and class
    When build-resource-shape-report runs over the backlog and the ci-findings ledger
    Then the report shows a node × resource-class matrix
    And each resource class is annotated with its documented tunable
    And any class lacking a documented tunable is flagged as a gap to close
