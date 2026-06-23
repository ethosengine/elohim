---
name: project_doorway_main_route_needs_is_service_path
description: any new doorway 8080 main-listener route must be added to is_service_path or the EPR router silently shadows it
metadata: 
  node_type: memory
  type: project
  originSessionId: 43157925-f031-4663-a48e-e8b292dd8fa2
---

Adding a new explicit GET route to doorway's main listener (`doorway-service/src/server/http.rs`)
takes **two** independent gates, not one — and the second is easy to miss because it lives ~1000
lines from the dispatch arm:

1. The `match (method, path)` dispatch arm (next to `/version`, `/status`).
2. **`is_service_path()`** (http.rs) — the path's prefix MUST be listed there.

Why #2: BEFORE the match block, the EPR router fires for `method == GET && !is_upgrade &&
!is_service_path(&path)` and calls `epr_router.resolve_alias/dispatch`. If a root projection
(`url_path="/"`) is registered, an unlisted path **dispatches to the SPA bundle instead of your
handler** — your arm is never reached. This is the same shape as the `/auth/portal` catch-all
incident and the all-zeros root-projection bugs ([[project_epr_router_empties_on_poisoned_scope]],
[[project_sprint_branch_not_orchestrator_indexed]]).

**`admission_exempt()` is a DIFFERENT, orthogonal gate** — it only stops the inbound-admission
semaphore from 503-shedding the path. It does NOTHING about EPR shadowing. A scrape/health/util
route typically needs BOTH: add to `admission_exempt` (don't 503 it under load) AND `is_service_path`
(don't let EPR shadow it).

**How to apply:** When you add a doorway main-listener route, add a `is_service_path` regression test
(`assert!(is_service_path("/yourpath"))`) — none of the dispatch/admission unit tests exercise a live
EPR router, so the shadow bug only surfaces at runtime once a root projection exists. Verified
2026-06-17 wiring the doorway `/metrics` surface (the `looking-at-frontend`/route work landed it in
commit `25bc75b1b`). The `server/CLAUDE.md` route-registry gate covers WHEN to add a code route at
all; this is the wiring detail it doesn't spell out.
