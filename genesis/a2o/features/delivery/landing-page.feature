@e2e @delivery @landing-page @requires:doorway
Feature: The deployed elohim.host landing page serves its load-bearing surface
  As an operator verifying a landing-page release
  I want confirmation that the deployed doorway actually serves the
  surface's load-bearing elements and its site-wide SEO contract
  So that a release can be checked off as SHIPPED without re-walking the
  full visitor experience on every deploy

  # Boundary with genesis/a2o/features/content/landing-discovery.feature: this
  # file checks DEPLOYMENT of the elohim.host landing surface — does the live
  # doorway serve the hero, the start CTA, the epic-card deck, the page's SEO
  # meta tags. The visitor EXPERIENCE (the story arc, the ways-in, the
  # stewardship ladder, individual epic-card content, graceful degradation of
  # an unreachable card) is discovery's job, not this file's — don't duplicate
  # its assertions here.
  #
  # Retired 2026-07-02, superseded by the 095694ed0 redesign (docs at
  # landing-discovery.feature): the five-pillar-card enumeration and the "42
  # content nodes / 3 humans" stats scenarios were content/experience
  # assertions, not delivery ones — the pillar concern now lives in
  # discovery's landing-pillars assertion, and the stats section was dropped
  # from the surface entirely with no successor anywhere. The standalone
  # "protocol-landing" ContentNode/X-Root-App-header scenario and the
  # SPA-not-yet-extracted bootstrap-fallback scenario are also retired — both
  # were built on a standalone "protocol-landing" root-app that was never
  # real; the actual root-app + bootstrap-fallback mechanism is covered
  # generically by spa-bundle-delivery.feature, the X-Content-Address contract
  # by content-addressing.feature, and the real elohim-host-landing
  # ContentNode by landing-page-dogfood.feature — this file duplicating them
  # under a fictional slug added no coverage.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- (a) Redesigned surface (095694ed0) — held until it deploys to alpha --

  @wip @browser-only
  # @wip drops when the redesigned landing (095694ed0) deploys to alpha
  Scenario: The deployed host serves the redesigned hero and its start-the-journey call to action
    When I open the landing page in a browser
    Then the element [data-testid="landing-hero"] is visible
    And the element [data-testid="landing-cta-start"] is visible

  @wip @browser-only
  # @wip drops when the redesigned landing (095694ed0) deploys to alpha
  Scenario: The deployed host serves the live epic-card deck
    When I open the landing page in a browser
    Then the element [data-testid="landing-epic-cards"] is visible

  # --- (b) Durable delivery concern — unaffected by the redesign ----------

  @wip
  # @wip: step glue for the og:title/og:description assertions was never
  # written — unrelated to the redesign; these tags are static in index.html
  # and unchanged by it.
  Scenario: Landing page has proper SEO meta tags
    When I visit "/"
    Then the page has og:title "Elohim Protocol"
    And the page has og:description containing "human flourishing"

  # --- (c) Blank-landing regression (2026-07-04, doorway fix c00fc7647) -----
  #
  # The EPR router diverts "/" into the V8 SSR engine (serve_ssr_route). The
  # Angular server bundle renders against a minimal SSR_DOCUMENT template that
  # carries NO client bundles by construction. From edge #1150 (02:35Z) the
  # render stopped panicking and began "succeeding" while activating no route
  # component — an empty <router-outlet> shell (terminal=rendered, fetches=0,
  # no <title>, no main-<hash>.js). Doorway served AND cached it: chrome header
  # over blank white on both hosts. Before #1150 every render FAILED, so the
  # projected-bundle fallback (which DOES carry client bundles) kept the site
  # alive — the failure path was the good path.
  #
  # Constraint: an SSR route's served document MUST be hydratable — it carries a
  # client bundle script (fallback bundle, or a real render composed with the
  # index shell). A successful-but-empty render must NEVER be served or cached;
  # doorway sheds it to the bundle fallback tagged `x-ssr-skipped: empty-render`.
  # Guard: elohim-render Rust unit test `render_output_is_empty_*` proves the
  # empty-shell detector; this scenario is the HTTP-level delivery anchor.

  @wip @regression
  # @wip drops when fix c00fc7647 deploys to alpha (currently the live "/" still
  # serves the empty shell). Runnable — all step glue exists.
  Scenario: The landing page serves a hydratable surface, never a bundle-less empty shell
    When the raw HTTP response for "/" is captured
    Then the raw HTTP response status is 200
    And the raw HTTP response body contains a non-empty html title element
    And the raw HTTP response body carries a client bundle script

  @wip
  # @wip: needs a doorway SSR empty-render fault hook (env-gated, sibling to
  # DOORWAY_SSR_FAULT_STALL_PATH) plus a capture step targeting it — neither
  # exists yet. Documents the fix's shed mechanism as a capability proof.
  Scenario: An SSR render that activates no route component sheds to the bundle fallback
    When the raw HTTP response for an SSR route whose render produces an empty shell is captured
    Then the raw HTTP response header "x-ssr-skipped" is "empty-render"
    And the raw HTTP response body carries a client bundle script
