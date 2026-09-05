---
name: project_doorway_shell_stale_head_incident_2026_09_04
title: Doorway stale-shell incident 2026-09-04
description: "Blank `/` on both hosts — warm-shell cache pinned a slug-fetched shell under an empty head; fleet doorways never project app-row heads (DEV_MODE); bites on any stale-but-200 landing"
metadata: 
  node_type: memory
  title: Doorway warm-shell served a previous bundle era (2026-09-04)
  type: project
  originSessionId: ea648d6c-d8fd-4f39-b661-1d4473759d02
  modified: 2026-09-04T14:50:05.501Z
---

**Incident (2026-09-04, ~04:23Z → afternoon):** alpha.elohim.host and elohim.host both answered 200 at `/` with an
index.html naming `main-EAKNZDUP.js` (app build #1691) while the row's declared browser head was #1692's bundle
(`main-7QFGHX5X.js`, PATCHed 04:52Z). Every other asset resolved; entry script 404 → blank page. `/lamad/` and
`/apps/elohim-host-landing/index.html` were correct (they proxy by slug / by hash).

**Mechanism (Opus + Codex RCA converged, log-confirmed):**
- The `/` shell path (`render/warm_shell.rs` + `server/http.rs` `dispatch_to_projected_epr` / `resolve_projected_shell`)
  fetched the shell by SLUG and stocked it under the head the doorway's own projection declared. With `declared == None`
  the store classified the hot shell **AtHead** (`Some(shell) if declared.is_none()`) → `ServeWarm` forever, no refetch,
  and no admin route evicts the hot map. Wire tell: `x-elohim-bundle: last-reconciled`; Loki: `EPR router: shell served
  from the warm-boot cache — no upstream fetch, head:""`.
- On the fleet `HEAD /apps/<slug>/_capability` → `x-projection-ready: false` for EVERY app: all deployed doorways run
  DEV_MODE=true, so main.rs wires the projection engine to a broadcast channel whose sender is dropped at once
  ("Projection engine started" / "Signal channel closed, engine stopping" in the same ms). `projected_entries` is only
  filled by the boot warm-stream; storage `content.updated` SSE clears the `/apps` slug index but explicitly does NOT
  re-project the row (storage_events_subscriber.rs "Known follow-up gap (Pattern Z.D)"). So a head PATCHed after boot
  is invisible to the shell path.
- The SSR breaker OPENED at 04:33Z (#1691's server bundle panics: `Cannot read properties of undefined (reading
  'isUint8Array')`) with "subsequent skips are silent" → every `/` rode the warm arm. `SSR bundle reconcile` loop reports
  `0 refreshed` forever.
- Sibling defect: apex API host `doorway.elohim.host` is nginx 503 — ingress backend `elohim-prod-elohim-edgenode-prod-8080`
  in a namespace with zero pods (manifests: genesis/orchestrator/manifests/doorway/prod.yaml). `elohim.host/` itself is
  served by the alpha doorways.

**Probes that discriminate (reach for these first on a blank `/`):** `curl -I <host>/` for `x-elohim-bundle` /
`x-ssr-skipped`; `HEAD /apps/<slug>/_capability` (`x-projection-ready`, `x-blob-hash`); download `/blob/<blobHash>` and
list the zip's `main-*.js` vs the served shell; `/db/content/<slug>` `blobHash` + `updatedAt`.

**Story + register:** a2o `genesis/a2o/features/dataplane/served-shell-boots.feature` (@act:ii, @concern:doorway-failover,
runnable from any host against the fleet with `--name 'handed a page that can boot'`); habit `doorway-failover` flipped
RED 2026-09-04 (atom `doorway/doorway-service/.epr-meta/doorway-failover.habit.md`). Cure: hash-bound shell fetch,
head-bound archive marker, unknown-head never AtHead (rate-limited refresh), warm-entry eviction on admin clear.

**How to apply:** a stale-but-200 shell is a cache-layer era mismatch, not a storage/DHT problem — check the four probes
before touching storage or seeds. Related: [[project_doorway_serving_path]], [[project_doorway_ops_incidents]],
[[project_app_pipeline_e2e_ghost_and_apex_seed_gate]].
