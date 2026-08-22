@e2e @deployment @p2p @epic:elohim-p2p-infrastructure @act:i
Feature: P2P Peer Validation
  As a peer steward keeping a household node on modest hardware
  I want my node to prove its peers are connected and to pause sync while heavy writes run
  So that my community's content keeps flowing and my node is not killed mid-import

  A node syncs with its peers and absorbs bulk writes (account imports, content
  seeding) on the SAME small memory budget. On a 256MB container, a phone, or a
  Raspberry Pi, running both at once has crashed the process out of memory and
  taken the steward's node offline in the middle of a seed. These scenarios
  validate the protections that keep such a node alive: peer connectivity is
  observable from the outside, and sync yields to heavy writes — pausing during
  imports and bulk creation, auto-suppressing while the publish backlog drains,
  and always resuming afterward, even when a write fails.

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
  #
  # Fabric scope: the peer counts above are the INCIDENT's operational
  # parameters, not preconditions. The pause/resume semantics under test are
  # peer-count-independent, so each scenario's fabric Given asserts a live
  # floor — at least 1 connected peer — because a fabric with zero connected
  # peers has no sync traffic to suppress and cannot prove anything. The floor
  # holds at any real mesh size (a 3-node household reports 2 connected peers
  # from any member's own view; alpha reports more).

  @regression @requires:owned-substrate
  Scenario: Storage pauses P2P sync during account import
    Given elohim-storage on doorway "alpha" has at least 1 connected peer
    When a seeder POSTs an account package for "Matthew" with 200 content items
    Then the P2P status should report sync_paused as true during the import
    And sync/replication cycles should be skipped until the import completes
    And after the import response returns, sync_paused should be false
    # Guard: without backpressure, concurrent sync + import causes OOM on 256MB nodes.

  @requires:owned-substrate
  Scenario: Storage pauses P2P sync during bulk content creation
    Given elohim-storage on doorway "alpha" has at least 1 connected peer
    When a seeder POSTs a bulk content batch of 100 items
    Then the P2P status should report sync_paused as true during the write
    And after the bulk response returns, sync_paused should be false
    # Threshold: batches under 50 items do not trigger the pause.

  @requires:owned-substrate
  Scenario: Sync resumes even if bulk write fails
    Given elohim-storage on doorway "alpha" has at least 1 connected peer
    And P2P sync is active
    When a bulk content request fails mid-write due to invalid JSON
    Then sync_paused should be false after the error response
    # RAII guard: SyncPauseGuard resumes sync on drop, including error paths.

  @regression @requires:owned-substrate
  Scenario: Sync auto-suppressed while drain backlog is large
    Given elohim-storage on doorway "alpha" has at least 1 connected peer
    And 3424 content items have been bulk-seeded
    When the drain publish queue has more than 100 items pending
    Then sync_paused should be true (auto-suppressed by drain)
    And sync/replication cycles should be skipped
    And once drain pending drops below 100, sync_paused should be false
    # Discovery: 4Gi pod OOM-killed at 73% drain (2500/3424) because
    # sync + replication + drain competed for memory simultaneously.
    # Operational parameters: 4Gi limit, 5 peers, 3424 items, 500/cycle drain
    # The drain is the priority after a seed — sync can wait.

  @requires:owned-substrate
  Scenario: P2P status endpoint exposes sync_paused state
    Given the doorway health endpoint is accessible
    When I check the P2P status
    Then the response should include a "syncPaused" boolean field
    # Observability: operators and elohim agents can see backpressure state.
