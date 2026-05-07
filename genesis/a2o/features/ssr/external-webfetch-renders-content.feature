@browser @ssr @delivery @requires:doorway @wip
Feature: External WebFetch renders concept HTML readable without JS
  As an AI design tool, search engine, or accessibility client without a JS engine
  I want to fetch a concept page and read its content
  So that I can index, summarize, or render the content without executing the SPA

  SSR routes in elohim-storage declare `render = "angular-ssr"`. When
  SSR_BUNDLE_PATH is set on doorway startup, the in-process elohim-render
  engine produces a full HTML document. An HTTP client without a JS engine —
  a search crawler, an AI assistant, a screen-reader proxy — receives complete,
  readable markup on first fetch.

  The key contract: the concept title appears in <title>, the concept body
  appears inside an <article> element, and <app-root> is not empty. These three
  invariants confirm Angular's SSR pass ran and the route resolved correctly.

  NOTE: <meta property="og:image"> and <og:description> meta tags are not yet
  wired in elohim-app's SSR Title/Meta service. The social-card scenario
  (social-card-crawler-gets-rich-preview.feature) covers og:* completeness once
  those are wired. This feature intentionally omits og:* assertions.

  Background:
    Given doorway is running with SSR enabled
    And elohim-storage is seeded with concept "elohim-protocol-overview"

  @requires:ssr-bundle
  Scenario: HTTP client without a JS engine fetches a concept page
    # An HTTP GET with no JS execution — same wire bytes a search crawler or
    # AI design tool would receive. The SSR contract guarantees readable HTML
    # before any client-side script runs.
    When an HTTP client without a JavaScript engine fetches "/lamad/concept/elohim-protocol-overview"
    Then the response has status 200
    And the response body contains the concept title in <title>
    And the response body contains the concept body in <article>
    And the response body contains <sophia-question> placeholders if any quizzes exist
    And no <app-root> tag is empty
    And the response includes the x-render-cache header
