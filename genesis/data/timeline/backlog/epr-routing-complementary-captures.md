---
id: "backlog-epr-routing-complementary-captures"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Complementary captures from the EPR-app routing brainstorm (2026-06-04)"
slug: "epr-routing-complementary-captures"
written: "2026-06-04"
author: "claude"
status: "captured"
priority: "low"
themes: [epr-routing, doorway-projection, observability, ingress-reconcile, memkit]
relatedNodeIds:
  - "genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md"
  - "genesis/docs/superpowers/plans/2026-05-29-substrate-shakeout-epr-delivery-sprint.md"
tags: [captures, D8, doorway]
---

# Complementary captures — EPR-app routing brainstorm

Surfaced while designing §12 of the pillar-EPR decomposition spec (URL & Routing Contract).
Each is adjacent work deliberately NOT absorbed into that design. Domain: D8 (Web2 Projection &
Doorway) unless noted. One line each:

- **Doorway proxy drops `X-Cache`** — storage sets `X-Cache` on app-file hits (`elohim-storage/src/http.rs:4589`) but the doorway proxy forwards only content-type + cache-control (`doorway-service/src/server/http.rs:1395-1404`); cache observability is blind through the doorway. Forward it.
- **Alpha ingress `/lamad/path` SSR-intent rules are dead weight** — `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml:91-97` carries route-specific prefixes anticipating SSR that doorway doesn't serve (SSR is `#[cfg(feature="ssr")]` + `/spa/*` only). Reconcile: remove the rules or wire the SSR seam from spec §12.2.
- **Tauri-direct deep-link verification** — storage-side safety-net fallback (§12.2) is what makes `:8090` deep links work; verify on the steward desktop app once Slice 1 lands (tauri-architect).
- **`LamadNotFoundComponent` → designed gate experience** — upgrade the lamad `**` page to the §6 outward-face pattern (preview + hints) once §12 Slice 3 lands.
- **MAP-drift gate misreport** — SessionStart claimed "9 seeds changed since MAP update" but git shows 2 architecture commits since 2026-06-03; the path-currency accumulator may be over-counting (process-meta, memkit subdomain).
- **`/db/paths` list endpoint absent** — all path-list HTTP probes 404 with the conductor-hint envelope; confirm no client depends on a list route (the app loads paths via `/db/content/{id}`), or add the route to storage's manifest if discovery needs it.
