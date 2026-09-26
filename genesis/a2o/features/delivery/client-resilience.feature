@e2e @content @delivery @requires:doorway @requires:seeded-content @act:i
Feature: Client Resilience — Service Worker and Capability Negotiation
  As a learner
  I want HTML5 apps to work offline after first load, delivered the cheapest way the network can
  So that my learning is not interrupted by network conditions or by a small peer's limits

  Caching is this feature's resilience strategy: the Service Worker (SW) makes every
  browser a self-sufficient peer. Once content is loaded, it stays cached locally.
  Capability negotiation lets the client select the best delivery strategy: fetch
  individual files from a peer with warm extraction, or fetch the ZIP and extract
  locally when a small peer can only hand over raw bytes.

  The roles. A DOORWAY is the household's gateway to the ordinary web: a browser asks
  it for pages and files. ELOHIM-STORAGE is the storage service each peer runs; a
  SERVING PEER is whichever storage node hands over the bytes. An app's files travel as
  one ZIP blob named by its blob_hash — the sha256 of its bytes — so a browser can check
  the bytes it received against the name it asked for. The HOUSEHOLD is the set of peers
  that share this content, and its ELECTION is the rule every peer applies to decide
  which version of an app is current.

  A serving peer keeps an EXTRACTION CACHE (ExtractionCache) of apps it has already
  unzipped, and lists them as ready_content in the capability announcement it publishes;
  a peer that lists an app can hand over its files one by one. When an app gets a new
  version, a CONTENT UPDATE SIGNAL — a message to the Service Worker naming the new
  blob_hash — tells the worker its cached copy is out of date.

  What the offline promise proves today, and what it does not. When Matthew goes offline,
  his browser shows the app from bytes it checked against their blob_hash, at the
  version it last checked. Which version that is, it learned from a doorway. Checking
  that version against the household's election is the next stage, named as its own
  scenario below.

  Two local cache rings compose, innermost first: the elohim-cache-core WASM
  cache (IndexedDB, sub-5ms) is consulted before the SW CacheStorage; the SW
  is consulted before the network (doorway → elohim-storage). A miss at one
  ring falls through to the next; a hit at an inner ring never touches an
  outer one.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And content "evolution-of-trust" has been seeded as html5-app
    And human "Matthew" is logged in on doorway "alpha" with device

  # --- Service Worker Registration ---

  @wip @browser-only
  Scenario: Service Worker registers in browser
    When Matthew visits the app for the first time
    Then the Service Worker is registered and active
    And the SW intercepts requests matching "/apps/"

  @wip @tauri-only
  # HELD (2026-08-21): @tauri-only: no declared layer runs a Tauri device; no runner anywhere today.
  Scenario: Service Worker registers in Tauri WebView
    When Matthew launches the Tauri desktop app
    Then the Service Worker is registered in the WebView
    And the SW intercepts requests matching "/apps/"

  # --- Offline Capability ---

  # RED-FIRST 2026-09-26 (the record-proves check).
  @browser-only @concern:record-proves
  Scenario: Cached app works offline
    Given Matthew has loaded "evolution-of-trust" while online
    And the Service Worker has cached all app files
    When Matthew goes offline
    And Matthew reloads "evolution-of-trust"
    Then the app's page renders with visible text
    And no request is answered by the network

  @wip @browser-only @concern:record-proves
  Scenario: The version shown offline is the one the household elected, not only one a doorway named
    Given Matthew has loaded "evolution-of-trust" while online
    And the Service Worker has cached all app files
    And his browser kept the household's signed election record for "evolution-of-trust" while online
    When Matthew goes offline
    And Matthew reloads "evolution-of-trust"
    Then the blob_hash his browser shows is the one the household's election names as current
    And his browser read that election itself rather than taking the version from a doorway

  @wip @tauri-only
  # HELD (2026-08-21): @tauri-only: no declared layer runs a Tauri device; no runner anywhere today.
  Scenario: Cached app survives storage pod restart
    Given Matthew has loaded "evolution-of-trust" on the Tauri client
    And the Service Worker has cached all app files
    When the local elohim-storage pod restarts
    And Matthew reloads "evolution-of-trust"
    Then the app loads from the Service Worker cache
    And the app's page renders with visible text

  # --- Capability Negotiation ---

  @wip
  Scenario: Peer advertises delivery capabilities
    Given elohim-storage has extracted "evolution-of-trust" in its ExtractionCache
    When elohim-storage broadcasts its capabilities
    Then the capability announcement includes serves_extracted: true
    And the capability announcement includes "evolution-of-trust" in ready_content

  @wip
  Scenario: ready_content updates when extraction cache changes
    Given elohim-storage has an empty ExtractionCache
    When "evolution-of-trust" is extracted into the cache
    Then a capability update is broadcast with "evolution-of-trust" in ready_content
    When "evolution-of-trust" is evicted from the cache
    Then a capability update is broadcast without "evolution-of-trust" in ready_content

  @wip @browser-only
  Scenario: SW probes peer capability before fetching assets
    Given the Service Worker cache for "evolution-of-trust" is empty
    When Matthew loads "evolution-of-trust"
    Then the SW sends a HEAD capability probe to the serving peer
    And the probe response includes the delivery mode and blob_hash
    And the SW uses the indicated delivery mode for subsequent file requests

  # --- Delivery Mode: Extracted Files ---

  @wip @browser-only
  Scenario: SW fetches individual files from a peer with warm extraction
    Given the serving peer advertises serves_extracted: true for "evolution-of-trust"
    When Matthew loads "evolution-of-trust"
    Then the SW fetches each file individually
    And each file is cached in SW CacheStorage
    And no ZIP download occurs

  # --- Delivery Mode: Client-Side Extraction ---

  @wip @browser-only
  Scenario: SW extracts ZIP locally when peer only serves compressed
    Given the serving peer advertises serves_compressed: true but not serves_extracted
    When Matthew loads "evolution-of-trust"
    Then the SW downloads the ZIP blob once
    And the SW extracts all files from the ZIP into CacheStorage
    And the requested file is returned from the local extraction
    And subsequent requests for the same app are served from CacheStorage

  # --- Cache Invalidation ---

  @wip @browser-only
  Scenario: SW invalidates cache when an app's version changes
    Given the Service Worker has cached "evolution-of-trust" with blob_hash "sha256-old"
    When a content update signal arrives with blob_hash "sha256-new" for "evolution-of-trust"
    Then the SW evicts all cached files for "evolution-of-trust"
    And the next request triggers a fresh fetch

  # --- elohim-cache-core (WASM) Layer — the innermost cache ring (see preamble) ---

  @wip @browser-only
  Scenario: WASM cache provides sub-5ms content lookups
    # elohim-cache-core is an IndexedDB-backed WASM module that sits in front of
    # the SW → doorway → storage chain. When warm it resolves content without any
    # network round-trip. Sub-5ms lookups are the design target.
    Given elohim-cache-core WASM is loaded in the browser
    And content "evolution-of-trust" has been fetched and written into the WASM IndexedDB cache
    When Matthew navigates to a previously visited page in "evolution-of-trust"
    Then the content lookup resolves from the WASM cache
    And no network request is made for that content
    And the lookup latency is under 5 ms

  @wip @browser-only
  Scenario: WASM cache falls back to network when content not cached
    Given elohim-cache-core WASM is loaded in the browser
    And the WASM cache has no entry for "evolution-of-trust"
    When Matthew loads "evolution-of-trust"
    Then the request falls through to the SW → doorway → storage chain
    And the content is served successfully from the network
    And the fetched content is written into the WASM cache for subsequent lookups

  @wip @browser-only @regression
  Scenario: WASM cache unavailable degrades gracefully
    # elohim-cache-core WASM is built by the DNA CI pipeline and fetched from Harbor, the artifact registry.
    # If the DNA pipeline hasn't run, the WASM blob is absent and the fetch returns
    # 404. The app must not crash or block content delivery when this happens.
    # See known issue: "WASM cache 404 noise" in CLAUDE.md.
    Given elohim-cache-core WASM failed to load with a 404 response
    When Matthew loads "evolution-of-trust"
    Then content requests bypass the WASM cache and go directly to the SW → network chain
    And "evolution-of-trust" loads and functions normally
    And no errors are shown to Matthew
    And the browser console contains exactly 1 warning about WASM unavailability
