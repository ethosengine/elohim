---
name: doorway-ssr-arm-shadowed-chrome-every-serve
title: EPR dispatch routes into SSR; chrome on every serve
description: "EPR-matched GETs divert to serve_ssr_route on SsrRoute disposition (shed → chrome-carrying bundle + x-ssr-skipped); page-borne wiring goes on BOTH serve paths"
metadata: 
  node_type: memory
  type: project
  originSessionId: 4a08a7cc-d02d-48dc-b1ef-639a7844a195
---

The doorway's EPR router intercepts non-service GETs BEFORE `classify_dispatch`, so all four
manifest routes declared `render: "angular-ssr"` (`/`, `/lamad/concept/{id}`, `/lamad/path/{slug}`,
`/lamad/path/{slug}/step/{n}`) never reach the `Disposition::SsrRoute` arm while their EPR mounts
exist. Loki-verified 2026-07-03: zero `SSR render trace`/`ssr_busy`/CSR-fallback lines in 14 days
against a positive control — the V8 SSR path (and anything wired only there) is dead code in prod.

That killed the omnibar (trust surface) when it moved out of the Angular bundle (887d81fd5) to an
SSR-arm-only splice (bd22e1559): regression live 2026-06-26 23:57 UTC (app build #1562), ~6.5 days.
Fixed 3fa34ee23: chrome splices on EVERY 2xx `text/html` serve — `dispatch_to_projected_epr`
(pure helper `maybe_inject_chrome`), the `/epr/{id}` universal arm, and the three CSR fallback
shells; injected pages get `cache-control: no-store` (request-specific `authenticated` flag),
assets/non-2xx/invalid-UTF-8 pass byte-identical. a2o guard:
`genesis/a2o/features/protocol/protocol-omnibar-chrome.feature` (@regression, exactly-one island).

**Cured 95dabec2a (operator direction 2026-07-03, spec §8 step 5):** an EPR-matched GET now also
consults the registry; the SsrRoute disposition diverts through `serve_ssr_route` (shared helper,
both dispatch sites), every shed/failure degrading to the chrome-carrying projected bundle with
`x-ssr-skipped:<reason>`. Capacity guard: renderer-present-but-capability-absent (live alpha) sizes
the render semaphore to a 2-permit default (`ssr_semaphore_permits`) — never unbounded V8 on cpu:1.

**Rule:** anything that must appear on served pages (chrome, headers, islands) must be wired on
BOTH terminal serve paths (SSR response + projected-bundle proxy) — a2o pins exactly-one island.

**Open follow-up:** the EPR router is a silent catch-all — any bogus path returns 200 with the
landing shell instead of 404, masking broken links from clients and monitoring.

Related: [[project_doorway_main_route_needs_is_service_path]],
[[project_epr_router_empties_on_poisoned_scope]], [[project_prod_main_lag_vs_alpha_dev]].
