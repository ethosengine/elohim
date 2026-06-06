@e2e @lamad @epr-decomposition @b23 @regression @deep-link @requires:doorway
Feature: Deep links to lamad land on the rendered page, not a 404 shell
  A learner who is handed a URL — shared in a message, bookmarked, or opened
  cold in a fresh tab — lands on the actual rendered surface that URL names.
  This is the §12 URL & Routing Contract (Slice 1, "alpha green"): the doorway
  SPA deep-link fallback serves the bundle's entry file for ROUTE sub-paths so
  Angular can match the route, while ASSET misses stay honest 404s.

  These scenarios guard the live alpha failure of 2026-06-04, where
  GET /lamad/path/foundations-christian-technology returned a storage
  app-file 404 ({"error": "File not found in app: ..."}) even though the
  content was seeded, public, and served fine elsewhere. Render-verified
  discipline: every assertion below reads a RENDERED element (data-testid),
  never the HTML <title> or a raw response shell.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @browser-only
  Scenario: Learner opens a shared path URL cold
    # Data flow §12.4.1 — cold canonical URL: mount match → ROUTE → entry_file
    # (<base href="/lamad/">) → Angular matches path/:pathId → renders the overview.
    Given a learner opens the deep link "/lamad/path/foundations-christian-technology" cold
    Then the lamad path overview renders
    And the rendered surface is not a raw error response

  @browser-only
  Scenario: Deep link straight to a step
    # Data flow §12.4.1 — a deeper ROUTE under the same mount; the step index is
    # a route segment, not a ZIP filename, so the fallback serves entry_file and
    # Angular matches path/:pathId/step/:stepIndex.
    Given a learner opens the deep link "/lamad/path/foundations-christian-technology/step/2" cold
    Then the lamad step navigator renders
    And the rendered surface is not a raw error response

  @browser-only
  Scenario: Legacy doubled URL shows the designed not-found
    # Data flow §12.4.2 — legacy doubled prefix: fallback serves the bundle, the
    # Angular router falls through to ** and renders the DESIGNED not-found page.
    # No longer raw JSON; doubled URLs are no longer minted (§12.3).
    Given a learner opens the deep link "/lamad/lamad/path/foundations-christian-technology" cold
    Then the lamad designed not-found page renders
    And the rendered surface is not a raw error response

  Scenario: Asset miss stays an honest 404
    # §12.2 — a missing hashed bundle file is a real deploy bug and MUST surface.
    # The final segment contains a dot (ASSET), so the fallback does NOT mask it
    # with index.html; the doorway answers an honest 404 with a JSON error body.
    When the missing asset "/lamad/main-DOESNOTEXIST00000000.js" is requested from doorway "alpha"
    Then the response status is 404
    And the response body is JSON, not an index.html shell

  @browser-only
  Scenario: Universal EPR address 302s a claimed type to its pretty mount
    # Spec §5.1 (Slice 3): claimed commons contentType → 302 to the mount.
    # The browser follows the redirect; the learner lands on the path overview.
    Given a learner opens the deep link "/epr/foundations-christian-technology"
    Then the lamad path overview renders
    And the rendered surface is not a raw error response

  @browser-only
  Scenario: Universal EPR address renders unclaimed types in the shell viewer
    # Spec §5.1: unclaimed contentType stays at the universal address —
    # the shell's cross-pillar resource viewer (safe-by-default floor).
    Given a learner opens the deep link "/epr/fct-module-01-church-dilemma"
    Then the cross-pillar resource viewer renders
    And the rendered surface is not a raw error response

  @browser-only
  Scenario: View Resource Details crosses the bundle boundary
    # §12.3 sweep — the lamad step navigator's resource link is a plain href
    # to the universal address; the epr-link interceptor records the handoff
    # and the full doorway load renders the shell viewer. Regression anchor
    # for the resource self-loop redirect killed in Slice 2.
    Given a learner opens the deep link "/lamad/path/foundations-christian-technology/step/0"
    Then the lamad step navigator renders
    When the learner follows the View Resource Details link
    Then the cross-pillar resource viewer renders
    And the rendered surface is not a raw error response

  @browser-only @regression
  Scenario: A monolith-era resource share still lands on rendered content
    # Constraint (story-harvest, Slice 2): an absolute redirectTo inside a
    # based bundle's router re-enters the SAME router and self-loops — it can
    # never escape the bundle. /lamad/resource/{id} was the monolith-era
    # canonical content URL (real shares exist); the legacy bridge COMPONENT
    # hands off to the universal /epr/{id} address instead. If this scenario
    # stops passing, the bridge died or the constraint moved.
    Given a learner opens the deep link "/lamad/resource/fct-module-01-church-dilemma"
    Then the cross-pillar resource viewer renders
    And the rendered surface is not a raw error response

  @regression
  Scenario: The monolith-era share is honored by a notarized alias promise
    # Spec §4 (Slice 3): the redirectTemplates grant on the lamad projection
    # 302s the legacy address at the DOORWAY — before any bundle boots. The
    # client bridge component remains only as the SPA-plane twin.
    When the path "/lamad/resource/fct-module-01-church-dilemma" is requested without following redirects from doorway "alpha"
    Then the response status is 302
    And the response Location header is "/epr/fct-module-01-church-dilemma"

  Scenario: The sitemap enumerates the claimed static plane
    # Spec §7.5: the sitemap is a derived projection of the routing table —
    # mounts plus claimed commons entries; it cannot drift from dispatch.
    When the path "/sitemap.xml" is requested without following redirects from doorway "alpha"
    Then the response status is 200
    And the response body contains "/lamad/path/foundations-christian-technology"
