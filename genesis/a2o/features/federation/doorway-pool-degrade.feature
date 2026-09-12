@e2e @federation @resilience @act:i
Feature: Doorway EPR router degrades through the storage pool
  As a household member visiting elohim.host
  I want the doorway to serve the landing and lamad apps even when its
  primary storage peer is missing my doorway's projection rows
  So that one peer's gap never makes the whole front door go dark —
  the federation pair is mutual aid at the gateway layer, not a diagram.

  VOCABULARY. An EPR is a published app bundle (the landing page, the lamad
  app). A "project-epr row" is a commitment stored on elohim-storage that says
  "doorway X serves EPR Y at URL path Z"; the set of those rows for one doorway
  (keyed by its doorwayId, e.g. alpha-elohim-host) IS that doorway's router
  table — with no rows the doorway has nothing to route and the front door is
  dark. The doorway re-reads them on a refresh cycle from its PRIMARY storage
  peer first, then from the rest of its storage POOL. "alpha" and "apex" are the
  two doorways of the federation pair — alpha fronts the alpha.elohim.host
  stand-in, apex fronts the elohim.host stand-in — each with its own primary
  peer and the same three-peer pool (on the household mesh: two local doorways
  in front of matthew, jessica and james). Scenario 3 targets apex because the
  2026-06-09 outage was apex's front door going dark, not alpha's.

  HOW THE HELD SCENARIOS EXECUTE. Each Given below that names a degraded
  primary CONSTRUCTS it: it calls elohim-storage's admin verb
  `POST {storage}/admin/projections/shade` (body {"kind":"project-epr",
  "shaded":true}) to hide that peer's project-epr rows from the read route the
  doorway's EPR router refresh consults — nothing is deleted, so there is no
  race with the 30s projection reconcile that heals a genuine gap, and an
  After hook always un-shades every peer this run shaded (pass, fail, or
  held) before the scenario ends. The verb is landing separately, from the
  storage side; until it is deployed on the build under test the route
  answers 404 and the Given step answers cucumber's `skipped` status with a
  printed reason naming the missing verb, so the scenario reads as held,
  never as passed or failed on a premise the substrate does not yet offer.

  # Evidence anchor (2026-06-09 /deliver iter-0): doorway-B's EPR refresh
  # loop read 0 rows from its primary (adam) every 30s at DEBUG — invisible —
  # while the same pod's /db proxy was serving 3 rows from a pool peer
  # (matthew). elohim.host 302'd to /threshold for ten days.
  # Journal: .claude/deliver/journal-resilient-dual-doorway.md
  #
  # PRECONDITION, measured 2026-08-22 on the Act I household mesh (run
  # 20260822T201747Z-3bd326d6): every household peer holds every doorway's
  # project-epr rows, because the Prologue seeds them to all three — so the
  # "primary returns zero rows" shape only exists on this mesh once a Given
  # below actively shades it. The degrade path is ALSO bound by the doorway's
  # mock-pool unit tests (doorway-service/src/projection/epr_router.rs); this
  # feature is the household-mesh-real half of that same proof.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Router populates from a pool peer when the primary returns no rows
    Given the doorway's primary storage returns zero "project-epr" rows for its doorwayId
    And a configured pool peer holds the doorway's "project-epr" rows
    When the EPR router refresh runs
    Then the router table contains the pool peer's projections
    And a WARN log names the degraded primary and the serving pool peer

  Scenario: Empty everywhere is a genuine empty state, not a silent wipe
    Given the doorway's primary storage and all pool peers return zero "project-epr" rows
    When the EPR router refresh runs
    Then the router table is empty
    And the empty state is logged at INFO with the consulted peer list

  Scenario: The apex front door serves through the degraded primary
    Given doorway "apex" at "E2E_DOORWAY_APEX"
    And the apex doorway's primary storage is missing its projection rows
    When I GET "/" from the doorway
    Then the doorway response status is 200
    And the doorway response Content-Type contains "text/html"
    When I GET "/lamad" from the doorway
    Then the doorway response status is 200
    And the doorway response Content-Type contains "text/html"
