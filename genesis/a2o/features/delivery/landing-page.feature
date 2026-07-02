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
