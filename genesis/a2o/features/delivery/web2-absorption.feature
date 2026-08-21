# Every scenario here runs against a single doorway in front of a single storage node.
# Nothing depends on remote multi-tenant compute, so this feature carries no tag for it.
@e2e @content @delivery @requires:doorway @requires:seeded-content
Feature: Web2 Absorption — Doorway Projection Cache
  As a learner visiting via browser
  I want HTML5 apps to load without 502 errors
  So that the protocol experience works regardless of how many learners are on at once

  An HTML5 app ships as one ZIP blob. A browser opening it does not fetch one
  thing — it fetches the entry page and then every asset that page references,
  30+ requests in a burst. Storage is a peer's node, not a CDN, and a handful of
  learners arriving at once is enough to bury it. So the doorway keeps a copy of
  the app's files and answers from that.

  It is called a PROJECTION cache because it is derived, never authoritative:
  every entry can be thrown away and rebuilt from the ZIP blob, which is the only
  source of truth. That is what makes it safe to evict aggressively.

  How a stale entry finds out it is stale: elohim-storage publishes an event
  stream, the doorway subscribes to it, and a content.created/updated/deleted
  event for a slug clears every cached file for that slug. The next request for
  it re-resolves the blob hash from storage and re-populates. Nothing polls, and
  nothing compares hashes at request time — the eviction is pushed.

  `evolution-of-trust` is the bundled HTML5 app used as the fixture throughout.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And content "evolution-of-trust" has been seeded as html5-app
    And human "Matthew" is logged in on doorway "alpha" with device

  # --- Cache Population (Cold Start) ---

  @act:i
  Scenario: First load proxies to storage and populates cache
    Given the projection cache for "evolution-of-trust" is empty
    When Matthew loads the html5-app "evolution-of-trust"
    Then all app files are served with 200 status
    And the projection cache contains entries for "evolution-of-trust"
    And each cache entry records the blob_hash from storage

  # An EPR agreement is the protocol's record of permission: the terms under which one party
  # may hold and serve another's content. "Self-negotiated" means the doorway writes its own —
  # it is caching on its own initiative, so it records the permission it is acting under rather
  # than waiting to be granted one. The point of putting that id on a cache entry is provenance:
  # every copy the doorway serves can name what entitled it to hold that copy.
  #
  # Still @wip, and for a reason worth naming: no surface exposes a cache ENTRY. The doorway
  # publishes GET /admin/cache/stats (counts only), so "each entry carries an agreement_id"
  # is unobservable from outside the process — the claim has no witness, in a test or for an
  # operator. Sheds @wip when a cache-entry read surface exists.
  @wip @act:i
  Scenario: Cache entries include EPR agreement reference
    Given the projection cache for "evolution-of-trust" is empty
    When Matthew loads the html5-app "evolution-of-trust"
    Then each cache entry has an agreement_id field
    And the agreement_id references a self-negotiated EPR agreement

  # --- Cache Hits (Warm) ---

  @act:i
  Scenario: Second load serves entirely from cache
    Given the projection cache for "evolution-of-trust" is warm
    When Matthew loads the html5-app "evolution-of-trust"
    Then all app files are served from the projection cache
    And zero requests reach elohim-storage

  # A "browser" here is the whole bundle, not one file: 30 browsers fan ~1200 requests at
  # one doorway from a single source IP, past the doorway's own rate limiter, which bans a
  # source for 900 seconds at that volume. That is an act which changes the mesh for every
  # other run on it, so the first step holds the scenario unless the operator has said go.
  @requires:owned-substrate @act:i
  Scenario: Storage remains stable under concurrent browser load
    Given this run owns the doorway's request budget
    And the projection cache for "evolution-of-trust" is warm
    When 30 browsers simultaneously load "evolution-of-trust"
    Then all browsers receive complete responses
    And elohim-storage memory usage stays within container limits

  # --- Request Coalescing ---

  @act:i
  Scenario: Concurrent cold requests coalesce into a single storage fetch
    Given the projection cache for "evolution-of-trust" is empty
    When 30 browsers simultaneously request the same file "pixi.min.js" from "evolution-of-trust"
    Then only 1 request is proxied to elohim-storage
    And all 30 browsers receive the same response

  # --- Cache Invalidation ---

  # WRITES: re-seeding moves a real content head. Held behind A2O_ALLOW_DESTRUCTIVE=1.
  # The literal "sha256-old" reads as "whatever hash is cached right now" — the contract is
  # that the cached hash is SUPERSEDED, never that it equals a particular string.
  @requires:owned-substrate @act:i
  Scenario: Re-seeded content invalidates stale cache entries
    Given the projection cache for "evolution-of-trust" is warm with blob_hash "sha256-old"
    When "evolution-of-trust" is re-seeded with a new ZIP blob
    And storage publishes the content-changed event for "evolution-of-trust"
    Then the projection cache entries for "evolution-of-trust" with hash "sha256-old" are evicted
    And the next request for "evolution-of-trust" proxies to storage
    And the new response is cached with the new blob_hash

  # --- Replica Consistency ---

  @wip @requires:multi-replica @act:ii
  Scenario: Cache is shared across doorway replicas
    Given doorway "alpha" has 2 replicas sharing MongoDB
    And replica 1 has served "evolution-of-trust" into the cache
    When Matthew's request is routed to replica 2
    Then the response is served from the shared projection cache
    And replica 2 does not proxy to storage
