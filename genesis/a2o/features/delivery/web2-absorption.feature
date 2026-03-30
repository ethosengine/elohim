@e2e @content @delivery @requires:doorway @requires:seeded-content
Feature: Web2 Absorption — Doorway Projection Cache
  As a learner visiting via browser
  I want HTML5 apps to load without 502 errors
  So that the protocol experience works regardless of how many learners are on at once

  Doorway's projection cache absorbs browser traffic patterns (30+ concurrent
  asset requests per HTML5 app) before they reach storage. Storage is a P2P node,
  not a CDN. The cache is permanent architecture — the onboarding flywheel that
  protects the entire network.

  The invariant: the ZIP blob is truth. MongoDB cache is a projection that can be
  rebuilt from the blob. Hash-based invalidation propagates on re-seed.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And content "evolution-of-trust" has been seeded as html5-app
    And human "Timothy" is logged in on doorway "alpha" with device

  # --- Cache Population (Cold Start) ---

  @wip
  Scenario: First load proxies to storage and populates cache
    Given the projection cache for "evolution-of-trust" is empty
    When Timothy loads the html5-app "evolution-of-trust"
    Then all app files are served with 200 status
    And the projection cache contains entries for "evolution-of-trust"
    And each cache entry records the blob_hash from storage

  @wip
  Scenario: Cache entries include EPR agreement reference
    Given the projection cache for "evolution-of-trust" is empty
    When Timothy loads the html5-app "evolution-of-trust"
    Then each cache entry has an agreement_id field
    And the agreement_id references a self-negotiated EPR agreement

  # --- Cache Hits (Warm) ---

  @wip
  Scenario: Second load serves entirely from cache
    Given the projection cache for "evolution-of-trust" is warm
    When Timothy loads the html5-app "evolution-of-trust"
    Then all app files are served from the projection cache
    And zero requests reach elohim-storage

  @wip
  Scenario: Storage remains stable under concurrent browser load
    Given the projection cache for "evolution-of-trust" is warm
    When 30 browsers simultaneously request "evolution-of-trust"
    Then all requests complete successfully
    And elohim-storage memory usage stays within container limits

  # --- Request Coalescing ---

  @wip
  Scenario: Concurrent cold requests coalesce into a single storage fetch
    Given the projection cache for "evolution-of-trust" is empty
    When 30 browsers simultaneously request the same file "pixi.min.js" from "evolution-of-trust"
    Then only 1 request is proxied to elohim-storage
    And all 30 browsers receive the same response

  # --- Cache Invalidation ---

  @wip
  Scenario: Re-seeded content invalidates stale cache entries
    Given the projection cache for "evolution-of-trust" is warm with blob_hash "sha256-old"
    When "evolution-of-trust" is re-seeded with a new ZIP blob
    Then the projection cache entries for "evolution-of-trust" with hash "sha256-old" are evicted
    And the next request for "evolution-of-trust" proxies to storage
    And the new response is cached with the new blob_hash

  # --- Replica Consistency ---

  @wip @requires:multi-replica
  Scenario: Cache is shared across doorway replicas
    Given doorway "alpha" has 2 replicas sharing MongoDB
    And replica 1 has served "evolution-of-trust" into the cache
    When Timothy's request is routed to replica 2
    Then the response is served from the shared projection cache
    And replica 2 does not proxy to storage
