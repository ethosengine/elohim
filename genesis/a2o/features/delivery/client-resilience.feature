@e2e @content @delivery @requires:doorway @requires:seeded-content
Feature: Client Resilience — Service Worker and Capability Negotiation
  As a learner
  I want HTML5 apps to work offline after first load
  So that my learning is not interrupted by network conditions

  The Service Worker makes every browser and Tauri instance a self-sufficient
  peer. Once content is loaded, it stays cached locally. Capability negotiation
  lets the client choose the best delivery strategy: fetch individual files from
  a peer with warm extraction, or fetch the ZIP and extract locally.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And content "evolution-of-trust" has been seeded as html5-app
    And human "Timothy" is logged in on doorway "alpha" with device

  # --- Service Worker Registration ---

  @wip @browser-only
  Scenario: Service Worker registers in browser
    When Timothy visits the app for the first time
    Then the Service Worker is registered and active
    And the SW intercepts requests matching "/apps/"

  @wip @tauri-only
  Scenario: Service Worker registers in Tauri WebView
    When Timothy launches the Tauri desktop app
    Then the Service Worker is registered in the WebView
    And the SW intercepts requests matching "/apps/"

  # --- Offline Capability ---

  @wip @browser-only
  Scenario: Cached app works offline
    Given Timothy has loaded "evolution-of-trust" while online
    And the Service Worker has cached all app files
    When Timothy goes offline
    And Timothy reloads "evolution-of-trust"
    Then the app loads and functions normally
    And zero network requests are attempted

  @wip @tauri-only
  Scenario: Cached app survives storage pod restart
    Given Timothy has loaded "evolution-of-trust" on the Tauri client
    And the Service Worker has cached all app files
    When the local elohim-storage pod restarts
    And Timothy reloads "evolution-of-trust"
    Then the app loads from the Service Worker cache
    And Timothy's experience is uninterrupted

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
    When Timothy loads "evolution-of-trust"
    Then the SW sends a HEAD capability probe to the serving peer
    And the probe response includes the delivery mode and blob_hash
    And the SW uses the indicated delivery mode for subsequent file requests

  # --- Delivery Mode: Extracted Files ---

  @wip @browser-only
  Scenario: SW fetches individual files from a peer with warm extraction
    Given the serving peer advertises serves_extracted: true for "evolution-of-trust"
    When Timothy loads "evolution-of-trust"
    Then the SW fetches each file individually
    And each file is cached in SW CacheStorage
    And no ZIP download occurs

  # --- Delivery Mode: Client-Side Extraction ---

  @wip @browser-only
  Scenario: SW extracts ZIP locally when peer only serves compressed
    Given the serving peer advertises serves_compressed: true but not serves_extracted
    When Timothy loads "evolution-of-trust"
    Then the SW downloads the ZIP blob once
    And the SW extracts all files from the ZIP into CacheStorage
    And the requested file is returned from the local extraction
    And subsequent requests for the same app are served from CacheStorage

  # --- Cache Invalidation ---

  @wip @browser-only
  Scenario: SW invalidates cache when content is re-seeded
    Given the Service Worker has cached "evolution-of-trust" with blob_hash "sha256-old"
    When a content update signal arrives with blob_hash "sha256-new" for "evolution-of-trust"
    Then the SW evicts all cached files for "evolution-of-trust"
    And the next request triggers a fresh fetch
