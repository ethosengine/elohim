@e2e @deployment @p2p @epic:elohim-p2p-infrastructure
Feature: P2P Peer Validation
  As a deployment pipeline
  I want to validate that P2P peers are connected and syncing
  So that I can verify the distributed data layer is operational

  Background:
    Given doorway "alpha" is healthy at env "E2E_DOORWAY_ALPHA"

  Scenario: Doorway reports connected P2P peers
    Given the doorway health endpoint is accessible
    When I check the P2P status
    Then connected_peers should be greater than 0

  Scenario: P2P sync documents are tracked
    Given the doorway health endpoint is accessible
    When I check the P2P status
    Then sync_documents count should be available

  # --- Sync Backpressure (Bulk Write Protection) --------------------------------
  #
  # Discovery: During account seeding, storage runs full-state inventory sync
  # with all peers every 60s. With 5 peers × 3400 items × ~200 bytes each,
  # that's ~6.8MB allocated per round on top of import processing. On a 256MB
  # container this caused OOM.
  #
  # Constraint: P2P sync and bulk writes compete for the same memory budget.
  # A push-full-state sync model cannot coexist with heavy writes on
  # resource-constrained peers (phones, Raspberry Pi, small containers).
  #
  # Operational parameters: 256MB memory, 5 peers, 3400+ inventory items,
  #   60s sync interval, ~6.8MB per sync round
  # Informs: peer diversity presets (home_node vs steward vs phone),
  #   operator documentation, container sizing recommendations
  # Review after: delta-sync protocol replaces full-state inventory exchange

  @wip @regression
  Scenario: Storage pauses P2P sync during account import
    Given elohim-storage on doorway "alpha" has 3 connected peers
    When a seeder POSTs an account package for "Matthew" with 200 content items
    Then the P2P status should report sync_paused as true during the import
    And sync/replication cycles should be skipped until the import completes
    And after the import response returns, sync_paused should be false
    # Guard: without backpressure, concurrent sync + import causes OOM on 256MB nodes.

  @wip
  Scenario: Storage pauses P2P sync during bulk content creation
    Given elohim-storage on doorway "alpha" has 3 connected peers
    When a seeder POSTs a bulk content batch of 100 items
    Then the P2P status should report sync_paused as true during the write
    And after the bulk response returns, sync_paused should be false
    # Threshold: batches under 50 items do not trigger the pause.

  @wip
  Scenario: Sync resumes even if bulk write fails
    Given elohim-storage on doorway "alpha" has 3 connected peers
    And P2P sync is active
    When a bulk content request fails mid-write due to invalid JSON
    Then sync_paused should be false after the error response
    # RAII guard: SyncPauseGuard resumes sync on drop, including error paths.

  @wip @regression
  Scenario: Sync auto-suppressed while drain backlog is large
    Given elohim-storage on doorway "alpha" has 5 connected peers
    And 3424 content items have been bulk-seeded
    When the drain publish queue has more than 100 items pending
    Then sync_paused should be true (auto-suppressed by drain)
    And sync/replication cycles should be skipped
    And once drain pending drops below 100, sync_paused should be false
    # Discovery: 4Gi pod OOM-killed at 73% drain (2500/3424) because
    # sync + replication + drain competed for memory simultaneously.
    # Operational parameters: 4Gi limit, 5 peers, 3424 items, 500/cycle drain
    # The drain is the priority after a seed — sync can wait.

  @wip
  Scenario: P2P status endpoint exposes sync_paused state
    Given the doorway health endpoint is accessible
    When I check the P2P status
    Then the response should include a "syncPaused" boolean field
    # Observability: operators and elohim agents can see backpressure state.
