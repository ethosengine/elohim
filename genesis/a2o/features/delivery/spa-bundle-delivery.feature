@e2e @content @delivery @requires:doorway
Feature: SPA Bundle Delivery — Root App from Blob Storage
  As a doorway operator
  I want my SPA served from the content system
  So that the protocol serves its own front door — one origin, no nginx

  The SPA is a spa-bundle content node with a ZIP blob. Doorway extracts
  and caches it, serving static assets directly and falling back to
  index.html for SPA routes. The bootstrap page shows live progress
  during cold start.

  The invariant: the ZIP blob is truth. Doorway's extracted SPA cache is a
  projection that can be rebuilt on warmup. ROOT_APP_SLUG names the content
  node; the rest resolves through the same content addressing as html5-apps.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "alpha" has ROOT_APP_SLUG set to "lamad"
    And content "lamad-spa" has been seeded as spa-bundle with a valid blobHash

  # --- Root App Resolution ---

  @wip
  Scenario: Root app serves index.html at /
    Given the SPA bundle for "lamad-spa" is extracted and cached in doorway "alpha"
    When an unauthenticated request is made for /
    Then the response is the SPA index.html
    And the response status is 200

  @wip
  Scenario: Static assets are served with immutable cache headers
    Given the SPA bundle for "lamad-spa" is extracted and cached
    When a request is made for a content-hashed asset (e.g. /main.abc123.js)
    Then the response status is 200
    And the Cache-Control header is "public, max-age=31536000, immutable"

  @wip
  Scenario: index.html is served with no-cache headers
    # index.html must never be cached long-term so the browser always fetches
    # the latest entry point. Only the hashed assets get immutable caching.
    Given the SPA bundle for "lamad-spa" is extracted and cached
    When a request is made for /index.html
    Then the response status is 200
    And the Cache-Control header is "no-cache, no-store, must-revalidate"

  @wip
  Scenario: SPA routes fall back to index.html so Angular handles routing
    # Angular uses the HTML5 History API. Any deep link must return index.html
    # rather than a 404, then Angular's router takes over on the client.
    Given the SPA bundle for "lamad-spa" is extracted and cached
    When a request is made for /content/manifesto
    Then the response status is 200
    And the response body is the SPA index.html
    And Angular's router handles the /content/manifesto route on the client

  @wip
  Scenario: API routes are not caught by the root app catch-all
    # /db/ is the elohim-storage API prefix. Catch-all must not shadow it.
    Given the SPA bundle for "lamad-spa" is extracted and cached
    When a request is made for /db/content/evolution-of-trust
    Then the request is proxied to elohim-storage
    And the response is not the SPA index.html

  @wip
  Scenario: /threshold is still accessible as the operator dashboard
    Given the SPA bundle for "lamad-spa" is extracted and cached
    And human "Matthew" is logged in on doorway "alpha" as operator
    When Matthew navigates to /threshold
    Then the response is the operator dashboard
    And the SPA catch-all does not intercept the /threshold route

  @wip
  Scenario: /apps/ still serves html5-apps and is not intercepted by the root app
    Given the SPA bundle for "lamad-spa" is extracted and cached
    And content "evolution-of-trust" has been seeded as html5-app
    When Timothy navigates to /apps/evolution-of-trust/index.html
    Then the response is the evolution-of-trust html5-app entry point
    And the SPA catch-all does not intercept the /apps/ route

  # --- Bootstrap Page ---

  @wip @browser-only
  Scenario: Bootstrap page is shown when SPA blob is not yet extracted
    # On a cold-start or fresh pod, the SPA blob may not have been extracted yet.
    # Rather than a blank screen, doorway returns a lightweight bootstrap page that
    # polls /health/startup and auto-navigates when the SPA is ready.
    Given doorway "alpha" has ROOT_APP_SLUG set to "lamad"
    And the SPA blob for "lamad-spa" has not yet been extracted
    When an unauthenticated request is made for /
    Then the response is the bootstrap page (not the SPA)
    And the bootstrap page displays a loading indicator

  @wip
  Scenario: /health/startup returns live startup status as JSON
    # The bootstrap page polls this endpoint. Each subsystem reports ready/not-ready
    # so the UI can show granular progress rather than a generic spinner.
    Given doorway "alpha" is starting up
    When a request is made for /health/startup
    Then the response is JSON with status fields:
      | field      | values              |
      | identity   | ready or pending    |
      | storage    | ready or pending    |
      | projection | ready or pending    |
      | rootApp    | ready or pending    |

  @wip @browser-only
  Scenario: Bootstrap page auto-navigates when rootApp becomes ready
    Given the bootstrap page is displayed
    And /health/startup is returning rootApp: pending
    When the SPA extraction completes and /health/startup returns rootApp: ready
    Then the bootstrap page navigates to / automatically
    And Timothy sees the SPA without manually refreshing

  @wip @browser-only
  Scenario: Bootstrap page falls back to reload timer when /health/startup is unreachable
    # If health endpoint is down (doorway restarting), the page should not hang.
    # A reload timer ensures the user eventually gets the app.
    Given the bootstrap page is displayed
    And /health/startup returns network errors on every poll
    When 30 seconds have elapsed
    Then the bootstrap page reloads the browser
    And a message is shown explaining the delay

  # --- No Root App Configured ---

  @wip
  Scenario: Without ROOT_APP_SLUG, / redirects to /threshold
    Given doorway "alpha" has no ROOT_APP_SLUG configured
    When an unauthenticated request is made for /
    Then the response is a redirect to /threshold

  @wip
  Scenario: Without ROOT_APP_SLUG, an unknown path returns 404 (not the bootstrap page)
    # The bootstrap page catch-all must not activate when no root app is configured,
    # otherwise every 404 becomes a confusing spinner.
    Given doorway "alpha" has no ROOT_APP_SLUG configured
    When a request is made for /some/unknown/path
    Then the response status is 404
    And the response is not the bootstrap page

  # --- Warmup Recovery ---

  @wip @regression
  Scenario: Projection cache self-heals after storage pod restart
    # After a pod restart, the projection cache (MongoDB) is warm but
    # elohim-storage may have lost in-memory state. The warmup retry mechanism
    # re-extracts the SPA blob so root app delivery recovers without operator
    # intervention. This was the failure mode that motivated the warmup retry feature.
    Given the SPA for "lamad-spa" was previously extracted and serving correctly
    And the elohim-storage pod has restarted and lost in-memory extraction state
    When doorway "alpha" detects that the warmup sentinel is stale
    Then doorway initiates a warmup retry for ROOT_APP_SLUG "lamad"
    And elohim-storage re-extracts the SPA blob
    And the root app becomes available again without operator intervention

  @wip
  Scenario: Root app becomes available after warmup resolves the slug
    Given doorway "alpha" has just completed warmup retry for "lamad"
    When a request is made for /
    Then the response is the SPA index.html
    And /health/startup shows rootApp: ready

  # --- Cache Invalidation ---

  @wip
  Scenario: CI uploading a new SPA blob causes doorway to serve the updated version
    # blobHash is the content address. When it changes, the old extraction is stale.
    # Doorway must detect the new hash, evict the old SPA cache, and serve the
    # updated bundle without a manual restart.
    Given doorway "alpha" is serving the SPA from blobHash "sha256-old"
    When CI re-seeds "lamad-spa" with a new ZIP blob (blobHash "sha256-new")
    And doorway receives the content update signal for "lamad-spa"
    Then doorway evicts the extracted SPA cache for blobHash "sha256-old"
    And doorway re-extracts the SPA bundle from blobHash "sha256-new"
    And subsequent requests for / serve the updated SPA

  @wip
  Scenario: Old SPA cached files are evicted when blobHash changes
    Given the projection cache contains SPA assets from blobHash "sha256-old"
    When the blobHash for "lamad-spa" changes to "sha256-new"
    Then all projection cache entries tagged with blobHash "sha256-old" are evicted
    And stale asset requests do not return old bundle files
