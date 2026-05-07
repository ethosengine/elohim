@browser @ssr @delivery @requires:doorway @wip
Feature: Social card crawlers receive rich link previews
  As a social-card crawler (Twitter, Slack, Mastodon, Discord)
  I want to fetch a learning path URL and extract OpenGraph metadata
  So that I can present a rich link preview when the URL is shared

  SSR routes in elohim-storage emit a full HTML document including <meta>
  elements populated by Angular's Meta service during server-side rendering.
  Social card crawlers (facebookexternalhit, Twitterbot, Slackbot) rely on
  og:title, og:description, and og:image being present in the initial HTML
  response — they do not execute JavaScript.

  NOTE: og:image and og:description wiring in elohim-app's SSR Meta service
  is not yet complete as of the doorway-ssr-runtime branch. These assertions
  will fail when the cluster runs until elohim-app sets those meta tags via
  Angular's Meta service in the route's SSR path. The @wip tag acknowledges
  this; remove it once the meta tags are confirmed wired.

  Background:
    Given doorway is running with SSR enabled
    And elohim-storage is seeded with learning path "elohim-protocol"

  @requires:ssr-bundle
  Scenario: Social card crawler previews a learning path step
    # A raw HTTP GET with a social-card crawler User-Agent. Doorway should NOT
    # vary its response body by User-Agent — the og:* meta tags must be present
    # for all clients, not just crawlers. The UA header is set here for
    # documentary symmetry and future UA-vary regression testing only.
    When a social card crawler fetches "/lamad/path/elohim-protocol/step/0"
    Then the response body contains <meta property="og:title">
    And the response body contains <meta property="og:description">
    And the response body contains <meta property="og:image">
    And the og:title equals the path's first step title
