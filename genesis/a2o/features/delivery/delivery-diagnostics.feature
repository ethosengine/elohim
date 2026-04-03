@e2e @content @delivery @requires:doorway @requires:seeded-content
Feature: Delivery Diagnostics — Observability and Controlled Degradation
  As a doorway operator
  I want to understand which layer is serving content and deliberately disable layers
  So that I can diagnose delivery issues, verify fallback behavior, and understand
  what my network is actually capable of

  This epic exists because Matthew discovered the need for projection caching by
  observing Evolution of Trust cause storage OOM under browser load. The ability
  to see what each layer does — and turn layers off to observe the result — is
  how the architecture was prompted. That diagnostic capability must survive into
  production, not just live in a developer's memory.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And content "evolution-of-trust" has been seeded as html5-app
    And human "Matthew" is logged in on doorway "alpha" as operator

  # --- The Original Failure (regression anchor) ---

  @wip @regression
  Scenario: Without projection cache, browser load overwhelms storage
    Given the doorway projection cache is disabled
    When 10 browsers simultaneously load "evolution-of-trust"
    Then all requests proxy directly to elohim-storage
    And elohim-storage receives 300+ concurrent requests
    And some requests fail or storage resource usage spikes dangerously
    # This is the scenario that motivated the entire Resilient Delivery epic.
    # It must remain as a regression anchor — if this stops being true,
    # the projection cache is no longer load-bearing and the architecture
    # should be revisited.

  @wip @regression
  Scenario: With projection cache enabled, same load is absorbed
    Given the doorway projection cache is enabled and warm for "evolution-of-trust"
    When 10 browsers simultaneously load "evolution-of-trust"
    Then zero requests reach elohim-storage
    And all browsers receive complete responses
    And elohim-storage resource usage is unchanged

  # --- Layer Observability ---

  @wip
  Scenario: Response headers indicate which cache layer served the request
    Given the projection cache is warm for "evolution-of-trust"
    When Matthew requests a file from "evolution-of-trust"
    Then the response includes a header indicating the serving layer
    And the serving layer is "projection-cache"

  @wip
  Scenario: Cache miss shows proxy layer in response
    Given the projection cache is empty for "evolution-of-trust"
    When Matthew requests a file from "evolution-of-trust"
    Then the response includes a header indicating the serving layer
    And the serving layer is "storage-proxy"

  @wip @browser-only
  Scenario: Service Worker reports delivery source for each request
    Given the Service Worker is active
    When Matthew loads "evolution-of-trust"
    Then the SW logs which source served each file
    And sources are one of: "sw-cache", "peer-extracted", "peer-compressed-local-extract", "doorway-proxy"

  # --- Controlled Layer Disable ---

  @wip
  Scenario: Operator can disable the projection cache at runtime
    Given the projection cache is enabled
    When Matthew disables the projection cache via operator API
    Then subsequent requests for app files proxy directly to storage
    And the projection cache is bypassed but not cleared
    When Matthew re-enables the projection cache
    Then subsequent requests are served from cache again

  @wip @browser-only
  Scenario: Operator can bypass Service Worker cache for diagnostics
    Given the Service Worker has cached "evolution-of-trust"
    When Matthew enables SW bypass mode
    Then requests go to the network even though the SW has cached responses
    And the response headers still indicate the upstream serving layer
    When Matthew disables SW bypass mode
    Then the SW cache resumes serving

  # --- Peer Capability Introspection ---

  @wip
  Scenario: Operator can query a peer's delivery capabilities
    Given elohim-storage is running with ExtractionCache
    When Matthew queries delivery capabilities for the storage peer
    Then the response shows serves_extracted and serves_compressed status
    And the response shows cache_tier
    And the response lists ready_content hashes

  @wip
  Scenario: Operator can see all peers and their delivery capabilities
    Given doorway "alpha" is connected to 3 peers
    When Matthew requests the delivery capability summary
    Then the summary lists each peer with:
      | field              | description                                  |
      | peer_id            | libp2p peer identifier                       |
      | network            | LAN or WAN or relay                          |
      | serves_extracted   | can serve individual files                   |
      | serves_compressed  | can serve raw blob                           |
      | cache_tier         | projection, extraction, or blob-only         |
      | ready_content      | list of content hashes warm in their cache   |
      | last_seen          | when this capability was last advertised      |

  @wip
  Scenario: Capability summary explains what the network can actually deliver
    Given the following peers are available:
      | peer           | serves_extracted | cache_tier  | ready_content includes evolution-of-trust |
      | doorway-alpha  | true             | projection  | true                                      |
      | storage-local  | true             | extraction  | true                                      |
      | remote-peer    | false            | blob-only   | false                                     |
    When Matthew views the delivery capability summary
    Then the summary indicates 2 peers can serve "evolution-of-trust" file-by-file
    And the summary indicates 1 peer requires client-side extraction
    And Matthew can understand the network's delivery posture at a glance

  # --- Degradation Testing Workflow ---

  @wip
  Scenario: Operator walks the fallback chain by disabling layers
    # This is the diagnostic workflow Matthew used to discover the architecture.
    # Each step peels back a layer to reveal the next.

    # Layer 1: Full stack — projection cache absorbs everything
    Given all delivery layers are enabled
    When Matthew loads "evolution-of-trust"
    Then the serving layer is "projection-cache"

    # Layer 2: Disable projection cache — storage extraction serves
    When Matthew disables the projection cache
    And Matthew loads "evolution-of-trust"
    Then the serving layer is "storage-extraction"

    # Layer 3: Evict from extraction cache — blob decompression serves
    When Matthew evicts "evolution-of-trust" from the extraction cache
    And Matthew loads "evolution-of-trust"
    Then the serving layer is "blob-decompress"
    And the response is slower but correct

    # Layer 4: All server caches disabled — SW cache serves (if warm)
    # (Browser-side; covered in client-resilience.feature)

    # Restore
    When Matthew re-enables all delivery layers
    Then the next load re-warms the caches
    And the serving layer returns to "projection-cache"

  # --- Cold-Cache Performance (Phase 1 Validation) ---

  @wip @browser-only @regression
  Scenario: Cold-cache HTML5 app loads via SW ZIP delivery without crashing storage
    # The original OOM failure happened when projection cache was cold and every
    # browser file request hit storage directly. SW ZIP delivery is the fix:
    # one blob fetch, local extraction, zero storage fanout. This scenario
    # validates the full path and confirms storage is not overwhelmed.
    Given the projection cache for "evolution-of-trust" is empty
    And the Service Worker is registered same-origin
    And browser console logging is enabled
    When Timothy loads "evolution-of-trust"
    Then the SW capability probe returns deliveryMode "compressed"
    And the SW downloads the ZIP blob via a single GET /blob/{hash} request
    And the SW extracts all app files into CacheStorage
    And the app renders correctly within 10 seconds
    And the browser console evidence shows:
      | event                    | count    |
      | capability probes sent   | 1        |
      | ZIP blob fetches         | 1        |
      | CacheStorage puts        | 30+      |
      | direct storage requests  | 0        |
    And elohim-storage received at most 2 HTTP requests total
    And elohim-storage memory usage did not spike above baseline + 50 MB

  @wip @browser-only
  Scenario: Same-origin ingress enables SW interception for /apps/ requests
    # SW can only intercept requests on the same origin. If the iframe renderer
    # builds a cross-origin URL (e.g. storage-host/apps/...) the SW is bypassed
    # entirely, defeating the caching strategy. This scenario verifies the ingress
    # routes /apps/ and /blob/ through the main origin so SW interception fires.
    Given the iframe renderer is building asset URLs for "evolution-of-trust"
    When the renderer resolves the entry point path
    Then the URL is relative to the main origin (e.g. /apps/evolution-of-trust/index.html)
    And the SW fetch event fires for the request
    And no CORS preflight is triggered
