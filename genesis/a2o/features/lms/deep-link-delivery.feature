@e2e @lamad @epr-decomposition @b23 @regression @deep-link @requires:doorway @act:i
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

  @browser-only @regression
  Scenario: Universal EPR address renders a claimed type lens-complete, not a 302
    # EPR Slice 1 (lens-complete-epr-resolution): the claims-302 is DEMOTED. A
    # claimed type (a path) at the bare /epr/{id} no longer bounces to its pretty
    # pillar mount (knowledge-only) — it serves the lens-complete shell viewer:
    # the focal content rendered AS its epr-composite outline + the pillar offered
    # as ONE lens ("Open in {pillar}"), the four-leg coupling honored at resolve.
    # (Inverts the Slice-3 302 this scenario used to assert; the pretty mount stays
    # directly reachable + via the lens. The /epr/{id}/raw atom view is a sibling.)
    Given a learner opens the deep link "/epr/foundations-christian-technology"
    Then the lamad composite outline renders
    And the Open in pillar lens is offered
    And the rendered surface is not a raw error response

  @browser-only
  Scenario: Universal EPR address renders unclaimed types in the shell viewer
    # Spec §5.1: unclaimed contentType stays at the universal address —
    # the shell's cross-pillar resource viewer (safe-by-default floor).
    Given a learner opens the deep link "/epr/unit-core-principles"
    Then the cross-pillar resource viewer renders
    And the rendered surface is not a raw error response

  @browser-only @wip
  Scenario: A shared markdown EPR link renders FORMATTED content, not the raw fallback
    # story-harvest (renderer-registration fix, 2026-06-18): the /epr/:resourceId,
    # /resource/:resourceId, and /deliver/:slug routes mount their content
    # component OUTSIDE LamadLayoutComponent. Renderer registration is a
    # side-effect of injecting RendererInitializerService, which only
    # LamadLayoutComponent triggered — so on these standalone routes the
    # RendererRegistryService stayed empty, getRenderer('markdown') returned null,
    # and the body dropped to the raw <pre> fallback (headers flattened, text
    # overflowing). Confirmed live on elohim.host/epr/theology.
    #
    # Why existing coverage MISSED it: "the cross-pillar resource viewer renders"
    # only proves the viewer chrome mounted, and "the markdown content body is
    # rendered" matches the shared .content-body wrapper (present in BOTH the
    # formatted render and the fallback) — both false-green on the broken state.
    # This scenario's assertion proves the markdown renderer ACTUALLY mounted
    # (app-markdown-renderer) and the "Content format:" fallback notice is absent.
    #
    # Constraint: every standalone renderer-consumer route must trigger renderer
    # registration itself. Capability proof — un-@wip to @regression once the fix
    # is deployed to alpha and verified green (RED on alpha until then, by design).
    Given a learner opens the deep link "/epr/theology"
    Then the cross-pillar resource viewer renders
    And the content renders as formatted markdown, not the raw fallback
    And the rendered surface is not a raw error response

  @browser-only @wip
  Scenario: View Resource Details crosses the bundle boundary
    # REACH-PINNED (alpha verification 2026-06-06): the FCT step's resource is
    # community-gated (403 requiredReach:community for anon) — the correct anon
    # outcome is the §6 gate face (head-edge + inclusive path), which is gap
    # #6-2 of epr-route-claims-link-conformance-design (deferred). Un-@wip when
    # the gate face lands (assert the boundary), or run with an authed
    # community-reach visitor. Today the shell bounces home (the undesigned
    # boundary this gap names).
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
    Given a learner opens the deep link "/lamad/resource/unit-core-principles"
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

  @browser-only @regression
  Scenario: View as Content opens the raw-node inspector, not a round-trip
    # EPR Slice 0 (lens-complete-epr-resolution): the overview's "View as Content"
    # used to mint the plain claims-aware universal address, which — before the
    # Slice-1 claims-302 demotion — bounced a CLAIMED path back to its OWN pretty
    # mount, a round-trip to itself (the UX contradiction captured 2026-06-06 in
    # epr-routing-complementary-captures; the earlier /epr/path-{id} nonexistent-id
    # bug it first exposed is fixed). It now mints the raw subview,
    # so following the link lands on the raw-node inspector: the EPR shown AS an
    # atom (id/CID, format, provenance), distinct from the rich path experience.
    # Guards the round-trip from returning.
    Given a learner opens the deep link "/lamad/path/foundations-christian-technology" cold
    Then the lamad path overview renders
    When the learner follows the View as Content link
    Then the lamad raw-node inspector renders

  @browser-only @regression
  Scenario: The universal raw address renders the node inspector, never a 302
    # EPR Slice 0: /epr/{id}/raw is the raw-node inspector. The doorway serves the
    # shell for the /raw subview — never a 302 to a pillar mount, even for a
    # CLAIMED type — and the shell's epr/:resourceId/raw route renders the atom.
    # Contrast the bare "/epr/{id}" sibling above, which (post Slice-1 demotion)
    # ALSO serves the shell — the lens-complete composite outline — never a 302:
    # same EPR, same serve-the-shell contract, different subview → different surface.
    Given a learner opens the deep link "/epr/foundations-christian-technology/raw"
    Then the lamad raw-node inspector renders
    And the rendered surface is not a raw error response

