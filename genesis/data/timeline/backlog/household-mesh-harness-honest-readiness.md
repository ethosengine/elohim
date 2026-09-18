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

## Current decision

**Captured, not started**, except item 7 (already landed — listed for context, not as work).
Items 1 and 2 are the highest-leverage: both are the same shape (process-liveness mistaken for
serving-readiness) and both directly caused lost work during the 2026-09-17/18 incidents this
cluster's sibling entries describe. Owner: next a2o-harness shift.
