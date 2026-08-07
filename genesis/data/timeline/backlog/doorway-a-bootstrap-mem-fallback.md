---
id: "backlog-doorway-a-bootstrap-mem-fallback"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway-alpha bootstrap store fell back to mem — shared-mongo invariant broken (A=mem vs B=mongo)"
slug: "doorway-a-bootstrap-mem-fallback"
written: "2026-08-07"
author: "hoot-owl integrator shift"
status: "open"
priority: "medium"
area: "dataplane"
domain: "code"
tags: [doorway, bootstrap, mongo, seam-smoke, iroh, wave2]
---

# Doorway A's bootstrap store is in-memory, not the shared mongo

Observed 2026-08-07 ~13:3xZ on the healed iroh fleet: `GET /admin/bootstrap-coherence`
returns `backend:"mem"` on doorway-alpha (A) vs `backend:"mongo"` on elohim.host (B) —
A=5 spaces/15 agents, B=4/10, and the counts can never equalize because A's PUTs land in
process memory. The bootstrap-sharing seam ("both doorways read the SAME store") is
structurally red until A reconnects to mongo. Discovery still works per-side (each
conductor family PUTs/GETs its configured doorway), so the mesh formed — degraded, not
down.

Likely shape: doorway A's redeploy (edge #1314, ~03:4xZ) lost/failed its mongo
connection at boot and fell back silently to the mem backend. Check doorway A's boot
logs for the mongo connect attempt; decide whether the fallback should be fail-loud
(a doorway that silently downgrades the shared store hides a partition risk — same
untrustworthy-facade class as the /db outage this incident exposed) and whether a
restart/config fix reconnects it. Related: iroh-lane-bootstrap-publish-dark (parent
incident), probe-conductor-diagnostics-doorway-404 (probe honesty).

## 2026-08-07 — fail-loud fix implemented, pending deploy

Root cause confirmed: `DEV_MODE=true` downgraded the mongo connect failure at boot
to a warn, silently selecting `MemK2Store` instead of refusing to start — exactly
the silent-downgrade risk this entry named above.

Fix landed in doorway-service: a `mongo_bootstrap_misconfigured()` decision function
(`src/bootstrap/store.rs`) plus a refuse-to-start check in `main.rs` — when
`BOOTSTRAP_MONGODB_DB` is configured but mongo is unreachable at startup, the
process now fails closed regardless of `DEV_MODE`. Unit-tested; full doorway gate
green.

Status: implemented, pending deploy. Live doorway-alpha (A) is still running the
pre-fix binary and remains on the mem backend until its doorway restarts onto the
fix — the shared-mongo invariant stays broken in production until that redeploy
lands.
