---
id: "backlog-storage-per-role-bridge-stuck-dead-after-conductor-restart"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "After a conductor restart, storage re-mints the imagodei/infrastructure bridges but the lamad (and node_registry) role bridge stays zomePath: dead"
slug: "storage-per-role-bridge-stuck-dead-after-conductor-restart"
written: "2026-09-03"
author: "tevah-station-3b-lane"
status: "open"
priority: "high"
themes: [storage, hc-client, conductor-bridge, resilience, death-witness]
relatedNodeIds:
  - "genesis/a2o/features/resilience/death-witness.feature"
  - "elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md"
tags: [household-testable, standing-bug, upgrade-gate]
---

# The lamad role bridge does not come back after a conductor restart

Observed 2026-09-03 on BOTH the 0.7 household mesh (the cutover session, 14:07/14:20Z restarts) and the
0.6 household mesh (this lane, `MESH_CONDUCTOR_LAUNCH=ark`, stock holochain 0.6.0, after the Act I
lane's mesh-wide conductor restarts). It is therefore a **standing storage-client bug**, not a 0.7 delta.

`GET /health` → `conductor.perRole` at 17:35Z on the 0.6 mesh:

| peer | lamad | imagodei | infrastructure | node_registry | note |
|---|---|---|---|---|---|
| matthew | live / rc0 | live / rc0 | live / rc0 | live / rc0 | never restarted |
| jessica | **dead / rc7** | live / rc7 | live / rc7 | **dead / rc0** | restarted by the ark |
| james | **dead / rc5** | live / rc5 | live / rc5 | **dead / rc0** | restarted by the ark |

`bridgeReconnects` climbs on the roles that recover and stays 0 on `node_registry`; `lamad` counts
reconnect attempts yet stays `dead` ("Websocket closed: No connection" on every lamad zome call, ~4400
failures / 15 min on 0.7). Sibling counter: "database is locked" (SQLITE_BUSY in `integrate_dht_ops`) in the
ark conductor logs, cumulative 16:32–17:35Z: matthew 5, jessica 452, james 543 — the restarted conductors
carry it.

## Why it matters to the death-witness habit

Every custodian-side write (`SpoolCustodyAuthor` → `create_rea_commitment`) and the ward's deterministic-id
conductor fallback in the custody read gate go through the **lamad** role. A custodian whose lamad bridge is
dead cannot counter-sign custody for a ward whose conductor just died — the exact moment the habit exists
for. Station 2/3b on the mesh had to `just mesh storage-restart` the custodians after the lane's restarts.

## Where to look

- `elohim/elohim-storage/src/hc_client_registry.rs` — the supervised per-role slots; which roles are
  re-minted on reconnect and why `lamad` / `node_registry` are not (role name conformance? a cell-id lookup
  that fails after the reinstall path? a role-specific app-interface token gone stale?).
- `/health` `conductor.perRole` is the probe; a2o step to write: "after the envelope restarts a conductor,
  every role bridge is live within N seconds" (belongs beside station 4 in `death-witness.feature`).

Evidence: the lane's probe log (`bridge-run2b.log`, session scratchpad) and the 0.7 session's 14:07/14:20Z
observations.
