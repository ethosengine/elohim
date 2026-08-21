# HTTP-only by construction: every assertion below reads a raw response the serving peer
# produced, so the story drives no browser — it is the compose contract as a crawler,
# onebox or text browser meets it.
@e2e @ssr @delivery @observability @requires:doorway @act:i
Feature: SSR compose serves the projected app's own markup — or names why it stepped aside
  As a visitor without a JS engine (a crawler, a link-preview onebox, a reader on a text browser)
  I want a projected route to carry server-rendered markup only when it is THAT app's markup
  So that the manifesto page is readable at the address it lives at, and a skipped SSR names
  the seam that skipped it instead of hiding every cause behind one opaque tag

  Three words this story leans on, so nothing below depends on knowing them already:
    * A peer PROJECTS an app when it answers that app's routes and serves its bundles. One
      peer may project many apps; whether it can RENDER any of them is a separate fact.
    * To SHED is to decline to server-render and answer with the client-side-rendering (CSR)
      shell instead — a correct, deliberate answer, not an error. What matters is whether the
      peer paid for a render before shedding, and whether it said why.
    * The COMPOSE step is where a rendered document is spliced into the shell that will be
      served. It is the seam that can splice the WRONG app's markup, so it is the seam that
      has to refuse.

  A serving peer (doorway projection today; a capable storage sidecar or steward hub on the
  same primitive — SSR is p2p-native, not doorway-owned) may project MANY apps while holding
  a renderer for only SOME of them. Two constraints discovered by driving the live system to
  its failure (2026-07-18 root-cause session):

  1. WRONG-APP RENDER WASTE: a doorway with one loaded server bundle (elohim-host-landing)
     dispatched SSR for every render-eligible route, including routes projected to a
     different app (lamad-spa). Each such request burned a full V8 render (12-420ms wall)
     whose output the compose step then rightly refused — the manifesto page served blank
     to crawlers on every request for ~2 weeks while reds read as one string.
  2. OPACITY: the single `x-ssr-skipped: shell-unavailable` tag collapsed a cross-app
     selector mismatch (deterministic, instant) and an upstream peer stall (intermittent,
     exactly the 10s dispatch timeout) into the same signal — undiagnosable from outside.

  The design that holds: the compose primitive (elohim-render) derives the app root tag from
  the rendered document itself — <app-root>, <lamad-root>, any selector — and refuses a
  cross-app splice with a typed reason; the serving layer gates renderer-vs-projection app
  identity BEFORE spending a render; every shed names its seam in `x-ssr-skipped`.

  # Operational parameters: 1 loaded server bundle vs N projected apps; wasted render
  # 12-420ms/request; shell-fetch timeout 10s (EPR_DISPATCH_TIMEOUT_SECS); opacity window
  # 2026-07-05..07-18. Informs: per-app bundle capability (SSR_BUNDLES_DIR) rollout and
  # peer render-capability presets.
  # Review after: multi-bundle renderer selection lands (lamad-spa server bundle seeding).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @requires:ssr-bundle @regression
  Scenario: A route projected to an app the renderer does not serve skips render-free
    # The regression anchor: with only the landing bundle loaded, a lamad route must NOT
    # burn a V8 render just to discard it — the skip happens before rendering and says why.
    Given the loaded SSR bundle serves a different app than the route's projection
    When the raw HTTP response for "/lamad/path/elohim-protocol" is captured
    Then the raw HTTP response status is 200
    And the raw HTTP response header "x-ssr-skipped" is "renderer-app-mismatch"
    And no SSR render trace is recorded for the request

  @requires:ssr-bundle @regression
  Scenario: The app the renderer serves still composes true SSR
    # The capability that must survive the gate: the landing page keeps its composed,
    # hydratable server-rendered document.
    When the raw HTTP response for "/" is captured
    Then the raw HTTP response status is 200
    And the raw HTTP response header "x-ssr-rendered" is "1"
    And the raw HTTP response body contains Angular hydration markers
    And the raw HTTP response body carries a client bundle script

  # Still @wip: the proof needs a SECOND server bundle staged — the one for the route's own
  # projected app. A household mesh stages its bundles once at start-up (`just mesh prologue`),
  # and that step stages only the landing server bundle today, so such a peer has nothing to
  # observe here and the Given holds the scenario rather than failing it. Sheds @wip when
  # multi-bundle renderer selection stages a second app's server bundle.
  @requires:ssr-bundle @wip
  Scenario: A projected app with its own server bundle composes its own selector
    # Selector-agnosticism proof: when a lamad server bundle is loaded for lamad-spa, the
    # compose primitive splices <lamad-root> exactly as it splices <app-root> — no code
    # change per app, the root tag is derived from the rendered document.
    Given a server bundle for the route's projected app is loaded
    When the raw HTTP response for "/lamad/path/elohim-protocol" is captured
    Then the raw HTTP response header "x-ssr-rendered" is "1"
    And the raw HTTP response body contains server-rendered markup for the projected app's root element

  @requires:ssr-bundle @regression
  Scenario: A compose shed names the seam that failed
    # Observability gate: the old single "shell-unavailable" hid a selector bug and a peer
    # stall behind one string. Every shed now self-classifies: renderer-app-mismatch,
    # shell-fetch-failed (with elapsed_ms in the peer log), shell-root-missing,
    # render-root-missing, shell-breaker-open, shell-no-projection.
    #
    # SCOPE, stated so the proof is not overread: the route read here is whichever shed this
    # peer can actually construct — by default the same cross-app route as the first scenario
    # (E2E_SSR_SHED_ROUTE picks a different seam where one is staged). So this proves the
    # NEGATIVE for real — the opaque "shell-unavailable" is gone and a typed reason is present
    # — while the other five reasons stay unproven here until a peer can stage them. Each of
    # those deserves its own scenario when its seam becomes constructible.
    When the raw HTTP response for an SSR-eligible route that sheds to CSR is captured
    Then the raw HTTP response header "x-ssr-skipped" is present
    And the raw HTTP response header "x-ssr-skipped" is not "shell-unavailable"
    And the raw HTTP response header "x-ssr-skipped" names a known compose seam
