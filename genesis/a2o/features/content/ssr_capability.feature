Feature: SSR capability is advertised, honored, and accountable
  As a substrate operator
  I want each doorway to declare its SSR capability honestly
  So that peers can see which doorways carry which bundles, support which auth modes, and have what concurrency budget — and authenticated requests never silently downgrade to anonymous renders.

  # The substrate's compute-shape claim for server rendering. The doorway is
  # the honest source: it derives its claim from on-disk bundles intersected
  # with elohim-storage's manifest of SSR-eligible routes, and elohim-storage
  # pulls the claim into PeerStatusView for substrate visibility. CSR fallback
  # is the floor on every miss, mismatch, or overflow.
  #
  # Spec: genesis/docs/superpowers/specs/2026-05-08-ssr-capability-design.md
  # Plan: genesis/docs/superpowers/plans/2026-05-08-ssr-capability-implementation.md

  Background:
    Given a doorway is running with bundle "lamad-app@1.0.3" present on disk
    And elohim-storage manifest declares "/lamad/concept/{id}" with render="angular-ssr"
    And the doorway's render capability has been derived at startup

  Scenario: doorway exposes its derived capability at /admin/capability
    When I GET "/admin/capability" on the doorway
    Then the response status is 200
    And the response body's "bundles[0].name" is "lamad-app"
    And the response body's "authModes" includes "anonymous"
    And the response body's "renderers" includes "angular-ssr"

  Scenario: doorway returns null when no SSR claim is published
    Given the doorway has no SSR_BUNDLES_DIR configured
    When I GET "/admin/capability" on the doorway
    Then the response status is 200
    And the response body is exactly the JSON literal null

  Scenario: storage layers the capability into the peer-status view
    Given storage is started with DOORWAY_CAPABILITY_URL set to the doorway's /admin/capability
    When I GET "/api/peers/<storage_peer_id>" on storage
    Then the response body's "renderCapability.bundles[0].name" is "lamad-app"
    And the response body's "renderCapability.authModes" includes "anonymous"

  Scenario: storage degrades honestly when DOORWAY_CAPABILITY_URL is unset
    Given storage is started with DOORWAY_CAPABILITY_URL unset
    When I GET "/api/peers/<storage_peer_id>" on storage
    Then the response body's "renderCapability" is null

  Scenario: storage degrades honestly when doorway is unreachable
    Given storage is started with DOORWAY_CAPABILITY_URL pointing at a closed port
    When I GET "/api/peers/<storage_peer_id>" on storage
    Then the response body's "renderCapability" is null

  Scenario: authenticated request to anonymous-only doorway falls back to CSR
    Given the doorway's render capability is restricted to authModes=["anonymous"]
    When I GET "/lamad/concept/abc" on the doorway with header "Authorization: Bearer user-token"
    Then the response status is 200
    And the response header "x-ssr-rendered" is "0"
    And the response header "x-ssr-skipped" is "auth-mode-not-supported"

  Scenario: authenticated request renders when the doorway honors the auth mode
    Given the doorway's render capability includes authModes=["anonymous","doorway-hosted"]
    When I GET "/lamad/concept/abc" on the doorway with header "Authorization: Bearer user-token"
    Then the response status is 200
    And the response header "x-ssr-rendered" is "1"
    And the response header "x-ssr-renderer" is "angular-ssr"
    And the response header "x-ssr-bundle-version" is "lamad-app@1.0.3"

  Scenario: anonymous request renders against the always-present anonymous mode
    Given the doorway's render capability includes authModes=["anonymous"]
    When I GET "/lamad/concept/abc" on the doorway with no auth headers
    Then the response status is 200
    And the response header "x-ssr-rendered" is "1"

  Scenario: V8 fetch shim forwards the user's auth header to outbound storage fetches
    Given the doorway's render capability includes authModes=["anonymous","doorway-hosted"]
    And elohim-storage will reject anonymous reads of "/lamad/concept/abc/private-section"
    When I GET "/lamad/concept/abc" on the doorway with header "Authorization: Bearer user-token"
    Then storage receives the bundle's outbound fetch with header "Authorization: Bearer user-token"
    And the rendered HTML contains the authenticated content (not the anonymous placeholder)

  Scenario: capacity overflow falls back to CSR rather than queueing
    Given the doorway's render capability sets maxConcurrentRenders=1
    And a render is already in flight occupying the only permit
    When I issue a second concurrent GET to "/lamad/concept/def" on the doorway
    Then the second response has header "x-ssr-skipped" equal to "overflow"
    And the second response header "x-ssr-rendered" is "0"

  Scenario: bundle absence falls back to CSR with bundle-not-loaded reason
    Given the doorway has SSR_BUNDLES_DIR unset (no bundle on disk)
    And elohim-storage manifest declares "/lamad/concept/{id}" with render="angular-ssr"
    When I GET "/lamad/concept/abc" on the doorway
    Then the response status is 200
    And the response is the SPA shell (not a server-rendered document)
    # Note: the shell does NOT carry x-ssr-rendered: 0 today because the
    # current dispatch falls through to the storage-proxy path before the
    # observability headers are set. Tracked as a follow-up: every SSR-
    # eligible exit should carry machine-parseable provenance.

  Scenario: peer A inspecting peer B sees B's render capability
    Given peer A's storage knows peer B via libp2p gossip
    And peer B's doorway publishes a render capability with bundle "lamad-app@1.0.3"
    When peer A GETs "/api/peers/<peer_b_id>" on its own storage
    Then the response body's "renderCapability.bundles[0].name" is "lamad-app"
    And the response body's "renderCapability.bundles[0].version" is "1.0.3"

  Scenario: operator override reduces (never inflates) the derived claim
    Given the doorway has bundles "lamad-app@1.0.3" and "qahal-app@0.2.0" on disk
    And the doorway's override config is [render]\n bundles_hidden = ["qahal-app"]\n max_concurrent = 2
    When I GET "/admin/capability" on the doorway
    Then the response body's "bundles" has 1 entry
    And the response body's "bundles[0].name" is "lamad-app"
    And the response body's "maxConcurrentRenders" is 2

  Scenario: anonymous mode is always present even when override forgets it
    Given the doorway has bundle "lamad-app@1.0.3" on disk
    And the doorway's override config is [render]\n auth_modes = ["doorway-hosted"]
    When I GET "/admin/capability" on the doorway
    Then the response body's "authModes" includes "anonymous"
    And the response body's "authModes" includes "doorway-hosted"
