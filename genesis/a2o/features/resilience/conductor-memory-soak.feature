@e2e @resilience @conductor-memory
Feature: Conductor memory soak — a node stays alive under sustained load
  As a person trusting a peer to hold what matters to me
  I want the node serving my content to keep running under sustained mesh
  traffic, not crash and restart on a recurring cycle
  So that availability is continuous, not a sawtooth of OOM-restarts

  # Harvested 2026-06-19 from the conductor heap-leak debugging session.
  # Root cause + cure record:
  #   genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md
  #   genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-prod-changeset.md
  #
  # SCOPE NOTE: the *build mandate* that delivers this (the conductor binary must
  # link the jemalloc allocator, HC_FEATURES must include `jemalloc`, and a
  # declared cargo feature is NOT a linked allocator) is a build-system contract,
  # NOT a learner experience — it belongs in the edgenode build / a pre-push or CI
  # assertion (e.g. `strings <holochain> | grep -q jemalloc`), not in this file.
  # These scenarios capture only the human-visible availability property + the
  # operator-visible diagnostic that made the cure possible.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- Capability proof: the bridge that holds ------------------------------

  @wip
  Scenario: A busy anchor conductor sustains a multi-hour soak without OOM-restart
    Given human "James" runs a high-fanout anchor conductor on doorway "alpha"
    And the conductor is built with the jemalloc allocator
    When the node serves sustained mesh traffic over a multi-hour window
    Then the conductor's anonymous-heap working set stays flat within a bounded band
    And the conductor records zero out-of-memory restarts
    And reads for content the node holds keep being served throughout
    # Cure parameters: jemalloc (decay/munmap) holds the conductor flat
    # ~2.1-2.9 GB working_set (oscillating peak-and-decay) indefinitely; 0 OOM
    # restarts well past the old ~5h cadence. Verified live 2026-06-18/19.
    # Informs: edgenode conductor build (HC_FEATURES must include `jemalloc`);
    # household_node / anchor memory presets.

  # --- Failure regression: the constraint anchor ----------------------------

  @wip @regression
  Scenario: Without the jemalloc allocator, a busy conductor OOM-cycles under load
    Given a conductor built with the default glibc allocator
    And it serves sustained mesh traffic on a high-fanout anchor
    When it runs for several hours without restart
    Then its anonymous heap climbs monotonically toward the container memory ceiling
    And it OOM-restarts on a recurring cadence, dropping availability between cycles
    # Constraint revealed: glibc ptmalloc2 pins freed memory in chained 64MB
    # secondary arenas (the 0x77xx mmap region) under conductor churn -> monotonic
    # OOM. Measured: ~8 GB ceiling, ~5h cadence on busy anchors (~1.16 GB/h);
    # ~0.2 GB/h on quiet nodes. Arc-INDEPENDENT (arc=0 leaks the same shape).
    # If this ever STOPS reproducing on a glibc conductor, the upstream allocator
    # behavior moved — revisit whether the jemalloc requirement still holds.

  # --- Observability gate: the diagnostic that enabled the cure -------------

  @wip
  Scenario: An operator can read the conductor's leak-vs-cache memory verdict
    Given human "Matthew" is observing node health on doorway "alpha"
    When the conductor's memory is sampled
    Then the per-process anonymous-heap trajectory is readable as a metric
    And the anon-heap is distinguishable from page cache (a true leak vs benign cache)
    And the per-VMA size bands localize where the growth lives
    # Diagnostic that made the cure possible: the elohim-storage /metrics surface
    # (elohim_node_conductor_smaps_anon_bytes{class} + anon size-band buckets +
    # smaps_growth) and cadvisor rss-vs-cache. The native-malloc-leak signature is
    # a FLAT Go heap (0xc0...~52MB) while glibc-arena anon (0x77xx) grows — which
    # is what exonerated go-pion/tx5 and pointed at the allocator. READ the shipped
    # smaps localizer before source-reasoning a memory growth.
