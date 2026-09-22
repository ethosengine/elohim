---
name: project_doorway_serving_path
title: "Doorway/EPR serving path, SSR, atom home (umbrella)"
id: project-doorway-serving-path
description: "New 8080 routes need is_service_path; poisoned scope empties EprRouter; EPR GETs → SSR; /epr/{id} atom home."
metadata:
  node_type: memory
  type: project
cites:
  - doorway/doorway-service/src/server/http.rs
  - "doorway-ssr-runtime | Doorway SSR Runtime | sha256:7f75b3027ae4f9d4 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md"
---

# Doorway/EPR serving path + SSR (umbrella)

Folds the doorway request-serving / EPR-router / SSR cluster. Members:

- [[project_doorway_main_route_needs_is_service_path]] — A new doorway 8080 GET route needs BOTH the match arm and is_service_path, else the EPR router shadows it to the SPA bundle; admission_exempt is orthogonal.
- [[project_epr_router_empties_on_poisoned_scope]] — One poisoned scope row empties EprRouter (Welcome at /, 404 /lamad): fail-closed collect + stale-binary array-wrap; resolvers degrade per-row (f38be2635).
- [[project_doorway_ssr_arm_shadowed_chrome_every_serve]] — EPR-matched GETs divert to serve_ssr_route on SsrRoute disposition (shed → chrome-carrying bundle + x-ssr-skipped); page-borne wiring goes on BOTH serve paths
- [[project_ssr_render_trace_and_fixed_fetcher]] — elohim-render SSR core: render() uses ctx.data_fetcher; RenderTerminal splits truthful-empty vs stall; compose derives the root tag (never hardcode app-root) + typed ComposeError skip vocabulary.
- [[project_ssr_first_deploy_seed_then_restart]] — First SSR deploy of an EPR app: App pipeline seeds serverBlobHash, THEN a doorway restart materializes it; a doorway-only push won't trigger App; edge-before-seed needs one extra edge restart.
- [[project_prod_main_lag_vs_alpha_dev]] — A UI bug on one doorway host but not another is per-host deploy lag, not code; two catchError-swallowed legs (edge container + spa-blob) leave a host stale.
- [[project_epr_atom_home_slice1_landed]] — folded 2026-09-22 (index: false); Slice 1 landed 2026-09-02: shell-owned /epr/{id}; habit RED until the fleet renders it. Commons next.

- [[project_doorway_shell_stale_head_incident_2026_09_04]] — folded (index: false); dated incident, stale-shell cache class subsumed by this umbrella.
