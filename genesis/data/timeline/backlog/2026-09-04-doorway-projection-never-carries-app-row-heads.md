---
id: "backlog-doorway-projection-never-carries-app-row-heads"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Deployed doorways never project an app row's declared head — DEV_MODE drops the projection engine's signal sender at boot, and storage's content.updated event clears the /apps slug index without re-projecting the row"
slug: "doorway-projection-never-carries-app-row-heads"
written: "2026-09-04"
updated: "2026-09-04"
author: "claude (landing-shell regression RCA, Opus + Codex converged, Loki-confirmed)"
status: "open"
priority: "high"
ci_status: open
jobs: [elohim-edge]
tags: [doorway, projection, projected-entries, app-file-cache, warm-shell, dev-mode, storage-events, content-updated, pattern-z, self-heal, doorway-failover]
cites:
  - doorway/doorway-service/src/main.rs
  - doorway/doorway-service/src/projection/storage_events_subscriber.rs
  - doorway/doorway-service/src/cache/app_file_cache.rs
  - doorway/doorway-service/src/render/warm_shell.rs
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - genesis/a2o/features/dataplane/served-shell-boots.feature
---

# The doorway's own projection has no head for any app row on the fleet

## Evidence (2026-09-04)

- `HEAD /apps/elohim-host-landing/_capability` and `HEAD /apps/lamad/_capability` on doorway-alpha both answer
  `x-projection-ready: false` while storage's rows carry `blobHash` (PATCHed 04:52Z by app build #1692).
- Loki, pod boot 03:40:58Z: `Projection engine started` → `Signal channel closed, engine stopping` within 150 µs, and
  never again. `main.rs` in the `dev_mode && !dev_signal_subscriber` arm builds `broadcast::channel(1)`, subscribes, and
  drops the sender on the next line — the engine has no signal source by construction. Every deployed manifest sets
  `DEV_MODE=true`.
- `projected_entries` is written only by the boot warm-stream (`warm_stream.rs`), conductor signals through the (dead)
  engine, and a manual full `/admin/cache/warm`. Storage's `content.updated` SSE reaches
  `storage_events_subscriber::handle_event`, which calls `app_file_cache.clear_slug(id)` and explicitly notes
  *"projected_entries is not refreshed by this event — Pattern Z.D scope"*.
- Consequence today: the `/` shell path (`warm_shell.rs`) looked up a `None` declared head, fetched the shell by SLUG,
  and pinned a previous-era index.html (`main-EAKNZDUP.js`) as AtHead — blank landing on both hosts (habit
  `doorway-failover` RED; story `served-shell-boots.feature`). The cure shipped for the shell path makes an unknown
  head `Behind` with a 30 s re-check, so the fleet converges; but the doorway still cannot address a shell by hash while
  its projection is blind, and `x-projection-ready` is a permanent `false` on every deployed doorway.

## What closes it

Two bounded moves, either sufficient for the shell path, both for the projection:

1. **Re-project the single row on `content.created|updated`.** In `handle_event`, after `clear_slug`, GET
   `{storage_base_url}/db/content/{id}` and `ProjectionStore::set()` it through the warm-stream row→doc mapping
   (`warm_stream.rs` ~118) so `resolve_blob_hash` sees the new `blobHash` at once. Needs the projection store handed
   into the subscriber (it already receives the EprRouter and app cache). One row per event, no full corpus pull.
2. **Stop wiring a sender-less channel under DEV_MODE on the fleet.** Either honour `--dev-signal-subscriber` in the
   deployed manifests (the doorways front real conductors), or make the projection-engine arm depend on
   `network_stage`, the way auth posture and seed authority already do (2026-08-25/27 DEV_MODE derivations), instead
   of on DEV_MODE.

Verification: `x-projection-ready: true` on `HEAD /apps/<slug>/_capability` after a deploy PATCH without a pod restart;
`served-shell-boots.feature` green with `x-elohim-bundle` absent (confirmed head) rather than `last-reconciled`.

## Related

- `2026-05-25-stagespablob-substrate-correct-deploy.md` (Pattern Z) names the fuller tightening (stageSpaBlob →
  `PUT /api/v1/epr/{cid}`); this row is the narrow reconcile that stops the doorway from being blind meanwhile.
- SSR breaker (`render/breaker.rs`) opened 04:33Z on build #1691's server bundle panic (`isUint8Array`) and its
  "subsequent skips are silent" cooldown never re-probed #1692's bundle — a separate observability gap: a silent open
  breaker should surface on `/health/serving`.
