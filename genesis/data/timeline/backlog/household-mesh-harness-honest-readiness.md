---
id: "backlog-household-mesh-harness-honest-readiness"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Household mesh harness readiness probes lie — TCP-listening and process-restarted are not cell-serving"
slug: "household-mesh-harness-honest-readiness"
written: "2026-09-18"
author: "doorway-overnight-20260914 shift (doorway-failover pickup)"
status: "backlog"
priority: "medium"
tags: [a2o, harness, household-mesh, readiness, hc-mesh, prologue]
cites:
  - app/elohim-app/scripts/hc-mesh.sh
  - genesis/a2o/reports/recovery/doorway-pickup-20260917/final/household-gossip-wedge-notes.md
---

# Household mesh harness — honest readiness

A readiness probe that lies is a deterministic-floor defect: it lets the next actor (a rerun, a
restart, a lane launch) act on state that isn't actually there yet.

## 1. `just mesh wait` is TCP-only

`app/elohim-app/scripts/hc-mesh.sh` `wait_all` (~lines 4059–4063) confirms a socket accepts
connections, not that cells are up. After an unclean kill, `create_cells_and_startup` takes
5–17 minutes during which admin/app websockets are already live but `running_cells: []`, and
every zome call fails `CellDisabled`. Restarting mid-window discards the in-flight rebuild and
resets the clock.

- chain / between "conductor process restarted" → "lane can call zomes" / missing node "conductor
  reports >0 running cells": assertion — `hc client call --port <admin> list-cells` returns a
  non-empty list; probe — poll that call, not the TCP accept, in both `wait_all` and `preflight`.
  State: **not built**.

## 2. `just mesh storage-restart` returns before storage can serve conductor-backed reads

The command returns once the process is up, not once it can answer. First minutes answer 503
`{"status":"catching-up"}`.

- chain / between "storage process restarted" → "lane reads content" / missing node "storage
  answers 200 on a conductor-backed read, repeatably": assertion — `/db/content/elohim-host-
  landing/head-record` returns 200 on three consecutive polls AND boot sweeps have gone quiet;
  probe — that two-part check, not process-liveness. State: **not built**.

## 3. Prologue leg `stamp-server-projection-peers` has no retry on a 503 shed

Observed to pass on rerun — the failure mode is transient (a 503 from an arm still catching up),
not a real gap in the projection, but the leg fails hard the first time instead of retrying past a
`catching-up` response.

## 4. Prologue readiness should require anchored heads, not just EXIT=0

Epic `/epr-head/<slug>` 404s for ~25 minutes after a fresh cast while anchoring drains. A green
prologue exit code does not mean the content it staged is retrievable yet.

- chain / between "prologue EXIT=0" → "content is retrievable" / missing node "staged heads are
  anchored, not just committed": assertion — `/epr-head/<slug>` returns non-404 before prologue
  is reported ready; probe — poll the epic head endpoint post-prologue instead of trusting the
  exit code alone. State: **not built**.

## 5. `conductors/.sandbox_run_log` has no rotation

Reached 6.3 GB in the 2026-09-17 incident; 154 MB in 2.5h on 2026-09-18. An unrotated log is
itself a resource-exhaustion risk during any incident long enough to need one.

## 6. A worktree lacks the gitignored font assets

`app/elohim-app/src/assets/fonts` is gitignored; a fresh worktree lacks it, so the landing
package is refused (`assets/fonts/fontawesome/all.min.css` missing) until copied from the main
checkout. Worth a documented copy-step or a fetch script, not a rediscovery per worktree.

## 7. Lane scope is relative to `genesis/a2o`; empty selection is now refused (done, for context)

Commit `502c7bcd7` — noted here only so the next reader of this cluster knows the scope-empty
footgun is already closed, not still open.

## 8. A non-clean `ng build` leaves a stale server entry — the staged SSR is not the staged browser build

Measured 2026-09-18: `app/elohim-app/dist/elohim-app/server` held 600 files across two build
generations; `main.server.mjs` still imported the OLD home chunk, so the doorway's SSR HTML carried
two raw `<iframe src="https://www.youtube.com/embed/…">` while the browser bundle (and
`version.json`, commit 804d5399e) carried the click-to-load facade. A visitor's browser loaded
YouTube from the SSR markup before hydration; the apex-transition shed scenario failed on
third-party `net::ERR_ABORTED` requests that the landed fix had supposedly removed. Cure applied by
hand: `rm -rf dist/elohim-app && ng build --configuration development`, then prologue re-stage →
both doorways' `/` render the facade with 0 raw embeds and the scenario passes.

- chain / between "local dist built" → "prologue stages landing (browser + server)" / missing node
  "the server entry and the browser bundle come from one build": assertion — every chunk
  `main.server.mjs` imports exists in the same build generation as `browser/version.json`; probe —
  refuse a dist whose server dir holds chunks no entry reaches, or always build clean before
  staging. State: **not built**. A served-HTML probe must read the SSR body, not the CSR shell —
  an earlier "0 raw iframes" check in this sprint read the wrong thing and passed falsely.

## 9. The household fixture does not seed what the staged apps reference

The landing carousel probes ten slugs at `/epr-head/<slug>`; the Lamad home journey requests
`theology`, `constitution`, `confession` and the intimate-reach `love-map-matthew-jessica`. A fresh
household after the prologue holds 27 content rows and none of these. The only documented lever,
`just seed apply mesh content`, seeds ~4,000 nodes and built the ~115k-op DHT described in
`household-lamad-gossip-wedge-large-dht.md`. `seed.ts --ids=<csv>` seeds exactly the named nodes
in under a minute (used by hand 2026-09-18) but the `just seed` recipe does not expose it and no
prologue leg calls it.

## 10. Real-app journey through the non-authoring doorway (apex-transition `@browser` scenario)

Red 2026-09-18 on baseline-alpha with four required first-party errors,
`404 /api/v1/cache/Content/{theology,constitution,confession,love-map-matthew-jessica}`. Two seams:

- `real-app-network.ts` `EXPECTED_NEGATIVE_HTTP` declares exactly these four as expected negatives
  under `/db/content/<id>` (404/404/404/403); the app asks the doorway cache route first, and live
  alpha also answers `404 /api/v1/cache/Content/love-map-matthew-jessica`. Declared intent and
  observed URL form have drifted. Reconciling them edits an oracle — operator's call, not applied.
- After seeding the three public nodes by id, doorway A answers 200 on the cache route while
  doorway B answers 404 on `/api/v1/cache/Content/theology` although B's own storage answers 200 on
  `/db/content/theology`. chain / between "content synced to B's storage" → "real-app journey
  through B" / missing node "B's doorway cache route serves content its storage holds": the
  projection cache fills only on the authoring side (post_commit signals are cell-local) and the
  route has no storage fall-through on a miss. State: **unmet**.

## Current decision

**Captured, not started**, except item 7 (already landed — listed for context, not as work).
Items 1 and 2 are the highest-leverage: both are the same shape (process-liveness mistaken for
serving-readiness) and both directly caused lost work during the 2026-09-17/18 incidents this
cluster's sibling entries describe. Owner: next a2o-harness shift.
