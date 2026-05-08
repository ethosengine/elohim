@browser @ssr @delivery @requires:doorway @wip
Feature: Browser hydrates SSR'd content without a re-render flash
  As an Elohim Protocol learner browsing on a slow connection
  I want server-rendered content to upgrade to interactive without flickering
  So that the experience is uninterrupted from first paint through hydration

  SSR routes in elohim-storage declare `render = "angular-ssr"`. When
  SSR_BUNDLE_PATH is set on doorway startup, the in-process elohim-render
  engine produces a full HTML document with Angular 19 hydration markers
  (`ngh="..."` attributes). The browser receives this document, paints it
  immediately, then Angular's `provideClientHydration()` attaches event
  listeners to the existing DOM nodes without re-rendering them.

  The key contract: element identity (DOM tree shape + text content) must
  be identical before and after hydration. Any re-render would cause visible
  flicker on slow connections — the test fails if Angular reconstructs the DOM.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" with device

  @requires:ssr-bundle
  Scenario: Concept page hydrates seamlessly
    # The /lamad/concept/{id} route is declared with render="angular-ssr" in
    # elohim-storage's manifest. Doorway dispatches it to elohim-render when
    # SSR_BUNDLE_PATH is set. The rendered HTML contains ngh="..." hydration
    # markers that Angular's client runtime uses to reuse the existing nodes.
    When the raw HTTP response for "/lamad/concept/fct-bible-micah-6-8" is captured
    Then the raw HTTP response status is 200
    And the raw HTTP response header "x-render-cache" is present
    And the raw HTTP response body contains Angular hydration markers
    And the raw HTTP response body contains a non-empty html title element
    When Matthew navigates to "/lamad/concept/fct-bible-micah-6-8" in the browser
    Then the concept page content is visible
    And there should be no console errors
    And no uncaught JavaScript errors occurred
    And the DOM tree matches the SSR'd content without reconstruction
