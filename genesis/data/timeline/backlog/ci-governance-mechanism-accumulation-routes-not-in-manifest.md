---
id: "backlog-ci-governance-mechanism-accumulation-routes-not-in-manifest"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "GET /api/v1/governance/{entityType}/{entityId}/mechanism and /accumulation 404 at the doorway on every content view — storage implements them but its /manifest route list never declares them, so the doorway registry classifies them NotFound"
slug: "ci-governance-mechanism-accumulation-routes-not-in-manifest"
written: "2026-08-25"
author: "epr-card-nav shift (integrator)"
status: "backlog"
priority: "medium"
ci_status: open
fingerprints: []
jobs: [elohim-edge]
relatedNodeIds: []
tags: [governance, doorway, route-registry, storage-manifest, console-noise, wave-d, content-viewer, a2o-console-cleanliness]
cites:
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/api/governance.rs
  - doorway/doorway-service/src/services/route_registry.rs
  - app/lamad/src/app/components/content-viewer/content-viewer.component.ts
  - genesis/a2o/steps/common.steps.ts
---

## Symptom

Every `/epr/{id}` and `/resource/{id}` view logs two console 404s:
`GET /api/v1/governance/content/{id}/mechanism` and `…/accumulation`, body
`{"error":"Not Found","hint":"Use WebSocket connection to /admin or /app/:port"}` — the doorway's
own `not_found_response`, not a storage answer. (Seen on elohim.host / alpha.elohim.host,
2026-08-25, while landing the EPR-card navigation fix.)

## Cause

The doorway routes `/api/v1/*` through a registry compiled from storage's `GET /manifest`
(`route_registry.rs::with_routes` / `fetch_steward_manifest`, matched segment-wise by
`path_matches`). `elohim/elohim-storage/src/http.rs` declares the governance routes explicitly
(`Route::get("/api/v1/governance/state")`, `…/challenges`, `…/proposals`, …) but has NO entry for
`/api/v1/governance/{entityType}/{entityId}/mechanism` or `…/accumulation`, even though
`api/governance.rs` implements both (`path.ends_with("/mechanism")`, `…/accumulation`, Wave D /
M-POLICY-1+2). The registry therefore never forwards them.

## Fix shape

Add the two `Route::get` declarations (pattern params `{entityType}/{entityId}`) to storage's
manifest list; the doorway picks them up on its next manifest fetch. Storage change → elohim-edge
pipeline (fleet redeploy) — schedule with the next planned edge roll, not as a standalone deploy.
Also: `GovernanceApiService.getMechanismSelection/getAccumulationStatus` may want to treat a
doorway NotFound as "no policy" without a console error, but the wire fix is the real cure.
