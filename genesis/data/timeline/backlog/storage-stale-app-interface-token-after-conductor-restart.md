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

## Sibling observed 2026-08-21 20:05 — the app-websocket LIFETIME class (open)

After `hc-mesh.sh conductors-restart` (stock 0.6.0, direct launch), the storage bridges on matthew and
jessica FLAP: `/health` `conductor.zomePath` alternates live→dead→live every few seconds
(`lastZomeFailureAgeSecs` 0–14, `bridgeReconnects` 3 and climbing; storage log: `conductor ping failed …
Websocket closed: No connection`, `heartbeat tick failed: record_peer_status zome call failed`,
`projection-reconcile: rea heal OPENED the unresponsive-conductor circuit`). James's bridge stayed
`unknown` (never a first zome call). The supervisor (77dd6b7b6) re-mints correctly — this is not the
stale-token defect; it is the conductor closing app websockets after seconds. The doorway agent measured
the same thing on its workers (`da63fd7a0` report, root-caused not fixed): sessions 2.4 s / 7.5 s, below
`STABLE_SESSION_THRESHOLD` (10 s), `Conductor closed connection: None` (stream end, no close frame), the
doorway sends nothing between zome calls — leading hypothesis a conductor-side idle reap; 18 reconnects / 40 s.
Before the restart (the 19:25 `hc sandbox run` boot) the storage bridges were stably live for an hour, so
compare the two launch paths' conductor configs first (app interface `allowed_origins`, interface
persistence, the `-p` difference the chaos agent found), then the fork's `holochain_websocket` keepalive.
Next morning's order: `just mesh stop` alone → fresh `just mesh start` (stock) → confirm stable bridges →
reproduce with `conductors-restart` → then decide where the keepalive belongs. Until then, do not use
`conductors-restart` on a mesh you intend to measure.


## Root-caused 2026-08-21 (late) — the flap is a FOURTH, unsupervised bridge, not an idle reap

Reproduced end to end on a throwaway conductor (stock 0.6.0, dead bootstrap/signal `:9999`,
admin 4744 / app 4745, throwaway storage `:8099`, live mesh untouched). Three runs, 3 min each:

| run | conductor launch | storage | zomePath over 3 min | bridgeReconnects |
|---|---|---|---|---|
| **A** | `hc sandbox run -a -p=4745` | fresh | `live` for 180 s, no failure | **0** |
| **B1** | **direct** (`holochain --piped --structured=Log --config-path …`), storage NOT restarted | carried over | **FLAPS** `live↔dead`, `lastZomeFailureAgeSecs` resetting to 0 every ~60 s | 3, then **flat** |
| **B2** | same direct conductor | fresh | `live` for 180 s, no failure | **0** |

**The launch path is exonerated.** Across A and B the conductor's `argv` is identical but for
the absolute-vs-PATH binary path; the process `environ` differs only in `PWD`, `OLDPWD`, the
PATH prefix and `_` (same `RUST_LOG`); and `conductor-config.yaml` is **byte-identical** through
generate → run A → run B (`hc sandbox run` rewrote it to the same bytes). B2 is the clincher: the
*same* direct-launched conductor is stable for 3 min once the storage peer is fresh. The variable
is **"a conductor restarted under a running storage"**, not "direct vs `hc`" — the earlier
correlation was an artifact of `conductors-restart` being the only action that produces that state.

**What actually flaps.** `spawn_bridge_supervisor` covers `SUPERVISED_ROLES =
[infrastructure, imagodei, lamad]`. All three re-minted correctly (`bridgeReconnects` 3, then flat,
`conductor bridge RE-MINTED …` ×3) and their sockets stayed alive — the supervisor's 20 s ping
succeeds and pushes `zomePath` back to `live`. But the **PeerStatus heartbeat task holds its own
`HcClient`** (`heartbeat.rs`), created at storage boot, in **no** supervised role slot, and it is
never re-minted. Every 60 s it logs, forever:

```
23:03:09 WARN conductor ping failed: Connection error: Conductor ping failed: Websocket error: Websocket closed: No connection
23:03:10 WARN heartbeat tick failed: record_peer_status zome call failed: … Websocket closed: No connection
23:04:09 WARN conductor ping failed: … (23:05:09, 23:06:09, …)
```

Both of those fold into `bridge_health()`, which is a **process-global** observer, not per-role. So
one dead client among four makes the whole `/health conductor.zomePath` flip `dead` at each 60 s
heartbeat tick and back to `live` at the next 20 s supervisor ping — the reported
`lastZomeFailureAgeSecs 0–14` with `bridgeReconnects` NOT climbing. **Two defects, both storage-side,
neither in the conductor:**

1. **The heartbeat's `HcClient` is outside the supervisor.** Either register the heartbeat's client
   as a supervised slot, or have the heartbeat borrow `registry.client("infrastructure")` per tick
   instead of holding a private handle for the life of the process. (It already logs its own
   failure — the supervisor is the only thing that can cure it.)
2. **`bridge_health()` is global where the bridges are per-role.** One dead role among N cannot be
   reported as a single boolean without flapping. `/health` should carry per-role zome-path state
   and derive the summary from it (any-dead → degraded, honestly named), so a flapping field stops
   meaning "the conductor is closing our sockets".

**Corollary found in the same run — no peer-policy file means NO bridge supervision at all.**
`spawn_bridge_supervisor` is nested inside the PeerStatus-heartbeat block, which is skipped whole on
`PeerStatus heartbeat disabled: policy config load failed`. A throwaway storage started without
`ELOHIM_STORAGE_PEER_POLICY_PATH` reproduced the ORIGINAL 77dd6b7b6 defect exactly: after a conductor
restart, `zomePath: dead`, `consecutiveFailures: 1`, `bridgeReconnects: 0`, still dead 80 s later with
no recovery attempt. The self-heal that exists to survive a conductor restart is gated on an unrelated
config file. Hoist `spawn_bridge_supervisor` out of that block — it depends on nothing in it.

**Not the conductor.** The fork's `holochain_websocket` already pings the client every 5 s
(`WebsocketReceiver::new`, `crates/holochain_websocket/src/lib.rs`), and the app-interface auth path
drops a connection only on a 10 s auth timeout or a rejected token
(`conductor/interface/websocket.rs:264`). No conductor change was made and none is indicated by this
evidence. The doorway's separate 18-reconnects-in-40 s measurement is NOT explained by this and stays
open — check `doorway-boot-self-heal-family-mesh-repro.md` **open sibling 5** first (doorway B derives
its app port from the global `--app-port-min` and therefore authenticates against conductor 0's app
interface with conductor 1's token) before reaching for a keepalive.

**`conductors-restart` remains half an operation**, for the reason above rather than the one first
suspected: follow it with `storage-restart`, and confirm with `zome-probe`.
