@e2e @content @delivery @federation @requires:doorway @requires:seeded-content
Feature: Peer Mesh — P2P App Delivery
  As a learner in a household
  I want my device to fetch content from the nearest peer
  So that content delivery is fast, resilient, and independent of any single server

  Peers serve peers. Doorway becomes one source among many, not the mandatory
  funnel. The client discovers, scores, and selects the best delivery peer —
  preferring LAN over WAN, warm extraction over compressed, and falling back
  gracefully when peers are unavailable.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And content "evolution-of-trust" has been seeded as html5-app

  # --- LAN Peer Discovery ---

  @wip @tauri-only @requires:multi-node
  Scenario: Tauri node serves app files to household peer over LAN
    Given human "Matthew" has loaded "evolution-of-trust" on Tauri node "matthew-laptop"
    And Matthew's ExtractionCache has "evolution-of-trust" warm
    And human "Jessica" is on Tauri node "jessica-laptop" on the same LAN
    When Jessica loads "evolution-of-trust"
    Then Jessica's SW discovers Matthew's node via mDNS
    And Jessica's app files are served from Matthew's node
    And the requests never leave the local network

  @wip @tauri-only @requires:multi-node
  Scenario: LAN delivery uses direct HTTP between Tauri nodes
    Given Tauri nodes "matthew-laptop" and "jessica-laptop" are on the same LAN
    And "matthew-laptop" advertises serves_extracted: true for "evolution-of-trust"
    When "jessica-laptop" requests files from "matthew-laptop"
    Then files are delivered via direct HTTP to Matthew's elohim-storage
    And no relay or doorway is involved

  # --- Multi-Peer Resolution ---

  @wip @browser-only
  Scenario: Client resolves multiple delivery peers via EPR
    Given "evolution-of-trust" has knownLocations including doorway and 2 peers
    When Timothy's SW needs to fetch "evolution-of-trust"
    Then the SW queries EPR for knownLocations
    And receives a scored list of peers with delivery capabilities

  @wip @browser-only
  Scenario: Peer scoring prefers LAN over doorway over remote
    Given the following peers are available for "evolution-of-trust":
      | peer             | network  | capability        | cache_tier  |
      | matthew-laptop   | LAN      | serves_extracted  | extraction  |
      | doorway-alpha    | WAN      | serves_extracted  | projection  |
      | timothy-remote   | relay    | serves_compressed | blob-only   |
    When the SW scores and ranks delivery peers
    Then the preference order is:
      | rank | peer             | reason                     |
      | 1    | matthew-laptop   | LAN + warm extraction      |
      | 2    | doorway-alpha    | WAN + projection cache     |
      | 3    | timothy-remote   | relay + client must extract |

  # --- Fallback Chain ---

  @wip @browser-only
  Scenario: Fallback chain degrades gracefully
    Given the SW has scored peers for "evolution-of-trust"
    And the best peer (LAN, extracted) goes offline
    When Timothy loads "evolution-of-trust"
    Then the SW tries the LAN peer and detects failure
    And the SW falls back to doorway projection cache
    And the app loads successfully from doorway

  @wip @browser-only
  Scenario: When all extraction peers fail, client extracts ZIP
    Given all peers that serve extracted files are unavailable
    And one peer with serves_compressed: true is reachable
    When Timothy loads "evolution-of-trust"
    Then the SW downloads the ZIP from the compressed-only peer
    And the SW extracts files locally
    And the app loads with a slightly longer initial delay

  @wip @browser-only
  Scenario: Doorway is not required when peers are available
    Given doorway "alpha" is offline
    And human "Matthew" on Tauri node "matthew-laptop" is reachable via relay
    And Matthew has "evolution-of-trust" warm
    When Timothy loads "evolution-of-trust"
    Then the SW discovers Matthew's node via EPR knownLocations
    And the app loads from Matthew's node
    And Timothy's experience is not degraded

  # --- libp2p QueryDelivery ---

  @wip
  Scenario: QueryDelivery protocol returns delivery info
    Given elohim-storage has "evolution-of-trust" extracted
    When a peer sends a QueryDelivery request with the blob_hash
    Then the response includes serves_extracted: true
    And the response includes cache_tier: extraction
    And the response includes warm: true

  @wip @regression
  Scenario: Old peers handle QueryDelivery gracefully
    Given an older peer that does not support the QueryDelivery variant
    When the SW sends a QueryDelivery message to the old peer
    Then the old peer returns an error response
    And the SW skips that peer and tries the next in the ranked list

  # --- Cross-Cutting: EPR Agreement ---

  @wip
  Scenario: Doorway caching authority is backed by EPR agreement
    Given doorway "alpha" caches content for "evolution-of-trust"
    Then the cache entry carries an agreement_id
    And the agreement references a self-negotiated EPR caching agreement
    And the agreement grants doorway permission to project and serve the content
