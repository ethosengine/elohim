---
id: "backlog-storage-stale-app-interface-token-after-conductor-restart"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storage never re-mints its conductor app-interface auth token after a conductor restart — the zome path is dead while /health answers 200 and writes are accepted that can never be anchored"
slug: "storage-stale-app-interface-token-after-conductor-restart"
written: "2026-08-21"
author: "claude (found while restarting the mesh conductors to make the sys-validation spin observable)"
status: "wip"
priority: "high"
jobs: [elohim-edge]
nodes: [elohim-matthew-alpha, elohim-jessica-alpha, elohim-james-alpha]
relatedNodeIds:
  - "memory:project_conductor_storm_starves_storage_reads"
  - "memory:project_local_stack_dht_anchor_gap"
  - "memory:project_alpha_substrate_probe_rails"
tags: [storage, conductor, holochain, websocket, auth, reconnect, health-check, liveness, anchoring]
cites:
  - app/elohim-app/scripts/hc-mesh.sh
  - genesis/data/timeline/backlog/rekeyed-peer-serves-dead-key-anchors-as-notarized.md
---

# A restarted conductor leaves every storage peer silently unable to anchor (2026-08-21)

## What happens

Each `elohim-storage` peer authenticates a websocket to its conductor's **app
interface** using a token minted at **the peer's** startup. Restarting the
conductor invalidates that token. The peer does not notice, does not re-mint,
and does not reconnect.

The conductor says so plainly, once per attempt:

```
WARN holochain::conductor::interface::websocket: Connection to Holochain app port 4445
  failed to authenticate: Authentication failed with reason: Invalid token. Dropping connection.
```

and from then on every zome call from that peer fails:

```
GET /db/content/<anchored-id>/head-record
{"error":"Conductor error: Zome call failed: Websocket error: Websocket closed: No connection"}
```

Observed on **all three** household peers after a conductor restart, and still
broken **90+ seconds later** with no recovery attempt. It does not heal on its
own; the peer must be restarted.

## Why it is worse than an outage

The peer stays up and cheerful. It is not down — it is **confidently useless**:

- `GET /health` → **200**, with `conductor: {mode: external}`.
- `GET /p2p/status` → answers, `projectionReconcile.caughtUp: true`.
- `GET /db/humans`, `GET /db/content` → answer normally (local database reads).
- `POST /db/content` → **201 Created**.

That last one is the damage. Writes are accepted and land in the projection, but
the row can never be anchored, because `reanchor_backfill` anchors via a zome
call. A row written in this state stays `dhtAnchorHash: null`, `trust:
published`, forever — verified with `chaos-v2-probe`, which was still NULL at
`trust: published` minutes later while `/health` said 200 the whole time.

So the failure is invisible to every liveness check the system currently
exposes, and the longer it persists the more unanchorable rows accumulate.

## The distinguishing probe

Every cheap endpoint is answered from the local database and stays green. The
probe has to force a **zome call**, and it has to use an **anchored** row —
`head-record` on a NULL-anchored row short-circuits with `"no notarized head
declared for this content"` **before** reaching the conductor, which reports a
dead peer as alive (this cost one wrong diagnosis before it was caught):

```bash
id=$(curl -s "http://localhost:8090/db/content?limit=25" \
     | python3 -c 'import json,sys; print(next(i["id"] for i in json.load(sys.stdin)["items"] if i.get("dhtAnchorHash")))')
curl -s -H "Authorization: Bearer $API_KEY_ADMIN" "http://localhost:8090/db/content/$id/head-record"
```

Also **not** usable: `/db/p2p/conductor-diagnostics` answers `"no embedded
conductor admin connection"` for every external-conductor topology, healthy or
not.

`./hc-mesh.sh zome-probe` implements this for the local mesh, and both
`conductors-restart` and `storage-restart` run it automatically.

## What to decide

1. **Re-mint on auth failure.** The app-interface client should treat
   `Invalid token` as "re-authenticate", not as a terminal connection error —
   mint a fresh token and reconnect with backoff. This is the actual fix.
2. **Liveness must include the conductor link.** `/health` returning 200 while
   no zome call can succeed is the reason this went unnoticed. Health should
   carry the app-interface connection state, and a peer that cannot reach its
   conductor should say so there.
3. **Refuse, or mark, writes that cannot be anchored.** Accepting a 201 for a
   row that provably cannot be anchored manufactures exactly the NULL-anchor
   population the reanchor backfill exists to clean up. Either shed the write
   (`503`, honest) or persist it flagged as un-anchorable.
4. **Alpha exposure.** Any conductor restart on the fleet — a rolling edge
   deploy, a pod bounce, an OOM-kill — puts each affected pod's storage into
   this state. Worth checking whether alpha pods have been silently accumulating
   unanchorable rows after conductor churn, since nothing in the current probe
   set would have shown it.

## Reproduce

```bash
./hc-mesh.sh zome-probe            # green
./hc-mesh.sh conductors-restart    # conductors only
./hc-mesh.sh zome-probe            # all peers: ZOME CALLS ARE DEAD, /health still 200
./hc-mesh.sh storage-restart       # the only way back today
```

## Update 2026-08-21 (late) — decisions 1 and 2 closed by `77dd6b7b6`

`fix(storage): a restarted conductor left the zome path dead forever, and /health said 200` — root cause
was supervision, not tokens per se: `connect_role_forever` returned on first success and nothing watched the
bridge afterwards; the late-connect re-arm was gated on `lamad_client().is_none()` (false once it had ever
worked) and covered only lamad. Now: `conductor_bridge_health.rs` (evidence-based zome-path observer: a
domain error proves LIVE, only a transport failure proves DEAD, an admission shed proves nothing),
`spawn_bridge_supervisor` probing every role every 20 s and re-arming the full re-mint, `/health` carries
`conductor.zomePath` (liveness stays 200), `/health/serving` returns 503 + Retry-After when dead. Throwaway
reproduction: dead forever before; recovered at T+29 s and T+21 s across two cycles after. 21 tests; storage
gate green.

**Decision 3 — refuse or flag writes that cannot be anchored — deliberately still open:** it is a policy
call against the offline-first invariant (shedding a write loses it on an offline-capable peer). Seam noted
by the fixing agent: NULL-anchored rows written during a dead window heal on the next `witness_bootstrap`
tick only where P2P is enabled; on a P2P-disabled peer they wait for restart — an unasserted precondition
between the supervisor and the healer. Also: the doorway's `ServingHealth` does not read storage's
`/health/serving`, so a doorway can report serving while its upstream cannot anchor (small doorway change).

