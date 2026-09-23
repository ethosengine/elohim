---
id: "backlog-self-heal-doorway-alpha-storage-breaker-matthew-rekey"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway-alpha storage:8090 breaker exhaustion (circuit Open / half-open reshedding) is a FAITHFUL MESSENGER of a substrate defect: matthew-alpha (genesis-pair member) underwent a self-heal REKEY + DNA-reinstall at edge #1199 boot, orphaning its DHT anchors (divergentAnchor climbing, not reconciling) so its server-side read path sheds 503 catching-up; the doorway breaker + conductor-auth remint are working AS DESIGNED"
slug: "self-heal-doorway-alpha-storage-breaker-matthew-rekey"
written: "2026-07-18"
updated: "2026-09-23"
author: "runtime-triage"
status: "backlog"
priority: "high"
self_heal_status: blocked
severity: high
fingerprints: [b7b25f86fe13, 6cdded115d74]
nodes: [doorway-alpha, elohim-matthew-alpha, elohim-matthew-alpha-conductor-0, intel-nuc, alpha]
relatedNodeIds:
  - "memory:project_local_stack_dht_anchor_gap"
  - "memory:project_alpha_topology_bootstrap_pair"
  - "memory:project_edge_deploy_restarts_genesis_conductors"
  - "memory:project_dna_hash_blind_to_coordinator_zomes"
  - "memory:project_p1_reconciliation_controller"
tags: [self-heal, circuit, doorway, storage-breaker, conductor-auth, genesis-rekey, anchor-divergence, catching-up, matthew, intel-nuc, operator-domain, substrate, projector-reconcile, divergent-refused, tail-phase, restart-churn, cell-disabled, cold-start-window, cpu-throttle, conductor-admission, heals-nothing, regression]
cites:
  - https://doorway-alpha.elohim.host/admin/self-healing
  - https://doorway-alpha.elohim.host/admin/render-stats
  - https://doorway-alpha.elohim.host/health
  - https://doorway-alpha.elohim.host/api/v1/resilience/summary
  - https://doorway-alpha.elohim.host/p2p/status
  - doorway/doorway-service/src/routes/storage_proxy.rs
  - doorway/doorway-service/src/routes/upstream_health.rs
  - doorway/doorway-service/src/worker/conductor.rs
  - doorway/doorway-service/src/main.rs
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/http.rs
  - genesis/data/timeline/backlog/self-heal-doorway-startup-conductor-mint-serialization.md
  - genesis/data/timeline/backlog/2026-07-10-server-side-epr-read-path-catching-up-shed.md
  - genesis/data/timeline/backlog/adam-genesis-anchor-sustained-saturation-post-storm.md
  - CLAUDE.md
  - https://doorway-alpha.elohim.host/db/p2p/conductor-diagnostics
  - https://doorway-alpha.elohim.host/health/serving
  - genesis/data/timeline/backlog/fleet-standing-celldisabled-one-third-party-cell.md
  - genesis/data/timeline/backlog/fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11.md
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
  - genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml
  - elohim/elohim-storage/src/conductor_admission.rs
  - elohim/elohim-storage/src/conductor_bridge_health.rs
  - elohim/elohim-storage/src/services/cell_probe.rs
  - .claude/data/runtime-findings.jsonl
---

# doorway-alpha storage:8090 breaker exhaustion — a faithful messenger of a matthew genesis-pair REKEY that orphaned its anchors

## What is exhausted

The poller class is `circuit:<endpoint>` — the doorway-alpha upstream breaker for
`http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090`. Finding
fingerprint `b7b25f86fe13`
(`fingerprint("alpha","self-heal-exhaustion","circuit:http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090")`).
Ledger line:

```
upstream http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090 circuit Open >= 3 consecutive polls
```

Incident evidence (Loki, pod `elohim-doorway-alpha-86748b885c-j78h2` on intel-nuc,
2026-07-18 ~01:25 UTC): the breaker log
`"upstream circuit OPEN — shedding without calling storage (503 + Retry-After)"`
(counter `doorway_upstream_breaker_open_total`) repeated across `/api/v1/resilience/*`
and `/db/content/*/head` for 40+ min, and doorway-alpha served
`{"status":"catching-up"}` 503s continuously for ~1h45m past the edge #1199 restart
(matthew booted 23:41; genesis self-heal rekey logs mark boot) — far past the ~20min
restart-churn norm.

Live endpoint at triage time (2026-07-18 ~01:27 UTC), `GET /admin/self-healing`:

```json
"admission": { "maxInflight": 256, "available": 256, "shedTotal": 0 },
"upstreams": [ { "endpoint": "http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090",
                 "circuit": "half-open", "errorStreak": 3, "lastGood": null, "skipped": false } ],
"projector": { "lagSeconds": null, "caughtUp": true, "divergentAnchor": 2091 },
"conductor": { "connected": true, "connectedWorkers": 4, "totalWorkers": 4 },
"render": { "total": 5, "degenerateRate": 0.0 }
```

Re-probe ~3 min later: `divergentAnchor` **2091 → 2177** — actively CLIMBING, not
reconciling. The condition is live and worsening at the substrate, even though the
two doorway self-heal layers have recovered.

## Root-cause inventory (scope pass)

**(a) The storage:8090 breaker is NOT stuck-open — it is a faithful messenger.**
`doorway/doorway-service/src/routes/upstream_health.rs` — `is_open()` advances
Open→HalfOpen after the 30s cooldown (`UPSTREAM_CIRCUIT_COOLDOWN_SECS=30`,
`UPSTREAM_CIRCUIT_FAIL_THRESHOLD=3`) and admits exactly one trial; `snapshot()` uses
`state()` (never side-effects a trial). Live state confirms the recovery path works:
the breaker is `half-open`, `skipped:false` (admitting trials), not wedged Open.
Direct probes THROUGH the half-open breaker return storage's own backpressure:
`GET /api/v1/resilience/summary` → 503, `GET /db/content/<x>/head` →
`{"status":"catching-up","retryAfter":30}`, `GET /p2p/status` → same. With
`admission.shedTotal:0`, the 503 is **storage-originated honored-backpressure**
(`storage_proxy.rs` — the 429|503 honor branch, `record(false)` +
`catching_up_proxy_response`), not a doorway-originated shed. The breaker correctly
opens because matthew:8090 genuinely and repeatedly returns 503; it is doing exactly
its job (don't hammer a catching-up upstream). No breaker bug.

**(b) The conductor app-port 4445 auth timeout is NOT a doorway stale-token bug — the
remint self-heal worked.** `doorway/doorway-service/src/worker/conductor.rs` — an
accept-then-drop (auth-reject) session is `session_len < STABLE_SESSION_THRESHOLD`
(10s), which triggers `remint_if_due` (rate-limited to `REMINT_MIN_INTERVAL`=30s):
the minter (`main.rs:make_token_minter` → `mint_app_auth_token` → admin
`issue_app_authentication_token`) fetches a FRESH token, so a stale token on the
doorway side is self-refreshing by construction. Live state confirms it converged:
`conductor.connectedWorkers:4/4`, `connected:true`. The conductor-side log on
matthew-alpha-0 —
`"Connection to Holochain app port 4445 timed out while awaiting authentication.
Dropping connection"` — is the conductor timing out its OWN auth handshake: it was up
on 4445 but the app/cell was not ready to validate any token during the long
genesis-self-heal REKEY + DNA-reinstall window. No fresh token the doorway mints can
authenticate against an app interface that is still reinstalling. This is
conductor/substrate-side, not doorway code.

**(c) The real defect is the substrate: matthew (a GENESIS-PAIR member) was
self-heal-REKEYED, orphaning its anchors.** `divergentAnchor` is computed in
`elohim/elohim-storage/src/p2p/projection_reconcile.rs:520-529`: it counts ids where a
peer advertises a non-empty `dht_anchor_hash` that DISAGREES with matthew's local
anchor. A rekey (new agent key after DNA reinstall) is exactly what produces a growing
divergence: matthew's local source-chain/anchors are now minted under a NEW key, so for
every id the other 5 peers still advertise the OLD-key anchor → disagreement →
`divergentAnchor` climbs (2091→2177 and rising). While divergent, storage's server-side
read path sheds `catching-up` (elohim-storage `http.rs` catching-up shed) rather than
serve a head it cannot trust — correct fail-closed behavior, but it will not
self-clear because a blind rekey has no lineage bridge back to the orphaned anchors.

`CLAUDE.md` names this class precisely: "reinstall mints a new agent key, which on prod
needs migration/lineage, not a blind wipe; the alpha genesis pair must both get the
flag" and "if you force-reinstall on some peers but not all in a namespace, they land
on different DNA hashes → different DHTs → P2P partition." matthew is `genesisPeer` —
per `project_alpha_topology_bootstrap_pair` + `project_edge_deploy_restarts_genesis_conductors`
the genesis pair must stay coherent; a self-heal rekey of one anchor is the anti-pattern.

## Fix path

**Doorway layer: nothing to fix.** Both self-heal mechanisms (per-upstream breaker;
conductor-auth remint) are correct and demonstrably recovered. Any "fix" that made the
breaker serve while matthew is anchor-divergent would serve untrusted heads — the
opposite of the trust contract. The prior read-path work
(`2026-07-10-server-side-epr-read-path-catching-up-shed.md`, read/write admission-pool
split) does NOT apply here: `admission.shedTotal:0`, so this is not a concurrency shed.

**Substrate layer (operator-owned):**
1. **Operator conductor action on matthew** — the acute clear is to bring matthew back
   onto a coherent identity. Two mutually-exclusive routes, operator's call:
   (i) if the rekey is the intended lineage step, land the KeyRotation/identity-lineage
   bridge so the new key inherits the old key's anchors (see the in-flight identity-lineage
   Wave B/C1 work: `rotate_identity_key`, `binds-identity`, chain-root, and the deferred
   `KeyRotation mint path` backlog) — then matthew re-anchors under lineage and
   `divergentAnchor` drains; or (ii) if the rekey was an unintended self-heal wipe,
   restore matthew's prior agent key/source-chain (do NOT blind-reinstall the other
   genesis member to "match" — that partitions the DHT).
2. **Genesis-rekey guard (design/substrate)** — a self-heal loop must NEVER blind-rekey
   or DNA-reinstall a `genesisPeer` anchor; gate that path behind lineage
   (`ALLOW_DNA_REINSTALL` semantics already exist for the pipeline — the runtime
   self-heal arm needs the same fence). This is the durable prevention.

## Current decision

**BLOCKED — substrate/operator-owned.** The doorway self-heal layers are healthy
messengers; the root cause is a matthew genesis-pair rekey that orphaned ~2177+ anchors
(and climbing), which no doorway code change can or should paper over. This requires an
operator conductor action on matthew (lineage bridge or key restore) plus a
genesis-rekey guard in the self-heal path — neither is a background-agent tree fix
(cluster ops are operator-owned; the lineage bridge is a sibling in-flight DNA plan).
Ledger fp `b7b25f86fe13` set `status: blocked` so the poller suppresses re-dispatch on
re-encounter (present fp = suppressed, ANY status) and cites this file; the stasis sweep
owns re-checks.

**Poller-detection note (not a code change here).** `_circuit_open` in
`.claude/scripts/_lib/runtime_harvest.py` fires only on `circuit=="open"` for 3
CONSECUTIVE polls; a breaker that oscillates open↔half-open (as this one does while the
upstream stays catching-up) can slip under that window, and `divergentAnchor` climbing is
not a predicate at all. If this concern proves to under-detect, a future poller-tuning
item could add a `divergentAnchor`-rising or half-open-reshedding predicate — deliberately
left to the deterministic-layer owner, out of scope for this ELEVATE triage.

## Verification

- Live probes at triage (2026-07-18 ~01:27 UTC, quoted above): breaker `half-open`
  `skipped:false`; conductor `4/4` connected; `admission.shedTotal:0`;
  `divergentAnchor` 2091→2177 climbing; storage read paths 503 catching-up through the
  half-open breaker. These jointly prove: doorway self-heal recovered (a,b), substrate
  still degrading (c).
- Closure (operator/poller-owned): resolved when matthew's `divergentAnchor` drains to
  ~0, its server-side reads return 200 (`/api/v1/resilience/summary`,
  `/db/content/<projected-cid>/head`), and the doorway breaker for
  `…matthew-alpha…:8090` sits `closed` with `errorStreak:0`. The poller closes fp
  `b7b25f86fe13` by disappearance once the circuit stops sitting Open. Regression
  signature to watch: `divergentAnchor` climbing again after any genesis-pair conductor
  restart/rekey.

## Recurrence — 2026-07-23 deploy window (fingerprint cross-contamination note)

The same breaker/saturation class fired during elohim/dev #1630's deploy window
(13:35–15:17 UTC) and wore ANOTHER concern's fingerprints: ci-harvest reopened the
projected-head probe fingerprints `15133508b92b` and `7569d2b6e0c6` (backlog
`ci-projected-head-convergence-race.md`) when the alpha-only probe legs failed. The
ci-investigator pass on #1630 grounded the split: the NEW convergence-window probe ran
correctly (14 re-probes over 420s, honest failure), served hash == #1629's declared
(alpha stuck one build behind), and the mechanism was matthew-conductor saturation —
`elohim-matthew-alpha-0` alive/0-restarts but zome-call + gossip-round timeouts
(`projection-reconcile[content]: conductor resolve failed … Websocket error: Timeout`),
doorway-alpha `upstream circuit OPEN — shedding without calling storage` — so all 24×4
DECLARE_ONLY attempts got `HTTP 503 {"status":"catching-up"}`. elohim.host passed all
legs instantly the same build. Reads for the triage owner: a projected-head fingerprint
reopen on alpha-only legs with `served == previous build's declared` is THIS concern,
not the probe race — split before re-triaging. The edge #1220 deploy (16:0x UTC,
storage+doorway image roll, conductor restarts) cleared the saturation; alpha
`/health/startup` back to 200/warmup-complete/circuits-closed at 16:22 UTC.

## 2026-09-11 — triage of ledger fingerprint `6cdded115d74` (projector arm; the TAIL of this same concern)

**This concern did not recur. It is finishing.** A different poller arm
(`projector:reconcile`, not `circuit:<endpoint>`) filed a new fingerprint on node `alpha`:

```
projector caughtUp=false sustained >= 3 polls
```

It is folded in here rather than given its own file because it is the *same root cause in
its residue phase* — and because this record predicted it. The 2026-07-18 "Poller-detection
note" above says, verbatim, that "`divergentAnchor` climbing is not a predicate at all";
what finally caught the tail was `caughtUp`, and the numbers below are this concern's own
closure criterion, partly met.

### What the closure criterion says vs what is live

That criterion: *"resolved when matthew's `divergentAnchor` drains to ~0, its server-side
reads return 200, and the doorway breaker sits `closed` with `errorStreak:0`."*

Live at triage, `GET https://doorway-alpha.elohim.host/admin/self-healing` (HTTP 200):

```json
"upstreams": [{"endpoint": "http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090",
               "circuit": "closed", "errorStreak": 0, "recentFailures": 0, "skipped": false}],
"admission": {"maxInflight": 256, "available": 256, "shedTotal": 0},
"projector": {"lagSeconds": null, "caughtUp": false, "divergentAnchor": 9},
"conductor": {"connected": true, "connectedWorkers": 4, "totalWorkers": 4}
```

- **Breaker leg: MET.** `closed`, `errorStreak: 0`, `skipped: false`, no shed. The
  `b7b25f86fe13` arm of this concern is substantively resolved.
- **Anchor leg: 99.6% drained, not met.** `divergentAnchor` has gone **2177 → 9**. The
  rekey's orphaned-anchor population has very nearly healed; 9 rows remain.

`GET https://doorway-alpha.elohim.host/p2p/status` — `projectionReconcile`, the detail the
`/admin/self-healing` projector block does not carry:

```json
{"pending": 4, "completed": 0, "failed": 2, "caughtUp": false, "peersAsked": 5,
 "divergentAnchor": 9, "healedTotal": 31, "sweeps": 57, "exhausted": 0, "converged": false}
```

`sweeps: 57` with `healedTotal: 31` settles the freshness question raised in the scope pass
(`ProjectionReconcileStatus` carries no timestamp, so a frozen `caughtUp:false` normally
cannot be told from a live one): **the heal leg is running and doing work.** This is a live
stuck residue, not a stale snapshot.

### The A/B asymmetry is the whole finding

The B-side (`https://elohim.host`, adam) reports the **same `divergentAnchor: 9`** and yet
`caughtUp: true`, `converged: true`, `pending: 0`, `failed: 0`.

`divergentAnchor` and `caughtUp` are orthogonal: the former is the discovery-side total of
distinct ids whose local anchor disagrees with a peer's advertised anchor (deliberately
never reduced — `elohim/elohim-storage/src/p2p/projection_reconcile.rs:1391-1401`); the
latter is `pending.is_empty()` at end-of-sweep, ANDed across all four heal arms
(`reconcile_rails.rs:244-246`, folded at `projection_reconcile.rs:1661`). `converged`
additionally requires `divergent_actionable == 0` where
`divergent_actionable = divergent_anchor - divergent_refused`
(`projection_reconcile.rs:1466`, predicate `elohim/elohim-storage/src/metrics.rs:4457-4483`).

So B, at the identical count of 9, has **refused all 9** into the declared-head refusal
partition and converged. A has not: its 9 stay actionable, 4 sit `pending` and 2 `failed`
at the end of every sweep, and `caughtUp` is honestly false.

`exhausted: 0` is the sharp detail. The cross-sweep `MissLedger` give-up arm
(`projection_reconcile.rs:304-380`) writes a row off after `MAX_RETRIES` misses **under
unchanged evidence**, but new evidence resets the counter immediately (`:349-356`). After
57 sweeps nothing has been written off — consistent with these 9 re-presenting as "new
evidence" each sweep and never aging into exhaustion. A retries them forever.

### Same 9 as the B-side provide-loop concern (cross-link, not a fork)

adam's `provideLoop` reports `reanchorDeadRemaining: 9`, `deadRemainingStuck: true`
(ledger fp `2b4761b2eaf6`, already `blocked`, canonicalized in
`genesis/data/timeline/backlog/dataplane-reanchor-dead-remaining-rekeyed-peer.md`). That
record's 2026-09-11 mechanism correction — rows are *settled-not-skipped*, and nothing on
the settled path clears the persisted `dht_anchor_state = 'dead'` verdict — describes the
same population from the other side: **9 content rows anchored under a pre-re-genesis agent
key whose chain nobody holds.** One population, three surfaces (A's actionable divergence,
B's refused divergence, B's dead reanchor remainder). Triage the three together; do not
re-derive them separately.

### Current decision — BLOCKED (unchanged posture, narrowed target)

The cure is the one already written in **Fix path** above, now pointed at 9 rows instead of
2177: either the identity-lineage bridge so matthew's current key inherits the old key's
anchors, or an explicit declared supersession for these specific rows (which is what would
move them from *actionable* to *refused* and let A converge exactly as B already does).
Both are operator/substrate actions — a conductor identity action and an in-flight DNA
lineage plan — and cluster ops are operator-owned.

Nothing in this tree fixes it. Making A refuse the 9 to force `converged: true` would be
falsifying convergence, not achieving it; the storage-side discriminator needed to confirm
*why* A does not refuse them (`elohim_projection_reconcile_converged_blocked_by{term}`,
`..._divergent_refused{stream}`) is per-pod Prometheus, not reachable from the admin
surfaces.

Ledger fp `6cdded115d74` set **`status: blocked`** citing this file, so the poller
suppresses re-dispatch (present fp = suppressed, ANY status) and the stasis sweep owns
re-checks.

**Closure signature, refined.** Previously "divergentAnchor drains to ~0". Now: `alpha`'s
`projectionReconcile` reaching `pending: 0, failed: 0, caughtUp: true, converged: true` —
reached either by the 9 healing under lineage (`divergentAnchor → 0`) or by a filed
supersession moving them to `divergent_refused` (`divergentAnchor` may stay 9, exactly as
B's does). **Watch that second shape**: `divergentAnchor: 9` alone is no longer evidence of
ill health — B proves 9-and-converged is a valid resting state. The regression signature
from the original record still stands: `divergentAnchor` *climbing* after any genesis-pair
conductor restart or rekey.

---

# 2026-09-23 — `6cdded115d74` re-filed as NEW: the tail REGRESSED, and the cause is no longer the rekey

> **This does NOT supersede the rekey record above.** That record is still the history of
> how matthew's anchors were orphaned. What it no longer explains is today's state. The
> 2026-09-11 section called this concern "finishing" (2177 → 9 divergent, `healedTotal: 31`
> over 57 sweeps). It has moved backward since: 9 → 57–59 divergent, and matthew heals
> nothing. Its own regression signature fired: "`divergentAnchor` *climbing* after any
> genesis-pair conductor restart". This time there was no rekey. There was a long run of
> conductor restarts, and a conductor with no CPU headroom between them.

## What is exhausted

Ledger line (`.claude/data/runtime-findings.jsonl`, filed poll 141, `2026-09-23T02:52:02Z`).
The fp was filed NEW at `seen: 1`, so the 2026-09-11 `blocked` line had closed by
disappearance in between. That was legitimate: on 2026-09-13 matthew *was* healing
(`healedTotal` 57 → 73). A regression coming back under the same fp is the intended
behaviour.

```json
{"fp": "6cdded115d74", "class": "self-heal-exhaustion", "node": "alpha",
 "provenance": "projector:reconcile",
 "line": "projector healed NOTHING (healedTotal 0 over 4-4 sweeps, divergentAnchor 57, converged=false) sustained >= 3 polls",
 "status": "open", "seen": 1, "first_poll": 141, "last_poll": 141}
```

The poller's cursor window for `alpha` shows a young process: `sweeps` 1,1,1,1,1,4,4,4, and
the per-sweep `failed` count is 86–92 in every sample.

**Re-fetched live at triage. The condition is LIVE, not a transient.** `GET
https://doorway-alpha.elohim.host/p2p/status` returned HTTP 200 on every read. The
`projectionReconcile` values:

| UTC | pending | failed | divergentAnchor | healedTotal | sweeps | caughtUp / converged |
|---|---|---|---|---|---|---|
| 03:44:13 | 48 | 5 | 59 | **0** | 14 | false / false |
| 03:54:06 | 47 | 4 | 59 | **0** | 16 | false / false |
| 03:56:06 | 54 | 3 | 57 | **0** | 17 | false / false |
| 04:01:52 | 87 | 5 | 57 | **0** | 18 | false / false |

In `/admin/self-healing` (03:44Z), the doorway breaker is `closed` (`errorStreak 0`), the
doorway admission gate is idle (`256/256`, `shedTotal 0`), and `projector {caughtUp: false,
divergentAnchor: 59}`. In `/health/serving` (03:53Z, **HTTP 503**), `storageServing:
{"status": "refused", "httpStatus": 503}`, so matthew's storage refuses to serve.

## Root-cause inventory (2026-09-23, Prometheus + Loki read-only, four ci-investigator passes)

**1. Restart churn is the main driver: six conductor restarts in ~11 h, each followed by
a ~50-minute cold start.** The pod is `elohim-matthew-alpha-conductor-0` and the container
is `elohim-conductor`. Restart boundaries came from `kube_pod_container_status_restarts_total`
and the first log line after each restart:
pod recreation 17:22–17:27Z; in-pod restart 19:52–19:57Z; pod recreation 21:12–21:17Z;
in-pod restarts at 21:52:26Z, 00:28:22Z and 02:30:28Z (09-22 → 09-23). The last-terminated
state reads `reason="Unknown"`, `exitcode="255"`. Working set was ≤2.26 GiB against a
5 GiB limit at every boundary, so this is **not an OOM kill**. No liveness-failure, panic
or SIGTERM line was found before any boundary. The dying instance's last lines were never
surfaced, so **the restart cause remains UNGROUNDED**. The next read is the
`event-exporter` k8s Events for this pod (operator/Grafana). The storage pod
`elohim-matthew-alpha-0` also restarted in the same windows: pod recreation ~20:00Z and
~00:00Z, and an in-pod restart at ~21:50Z. Its current process started ~02:40Z.

Each boot spends its time in the kitsune space join, `Dht::try_from_store`. For example,
the conductor log shows `DHT model initialised in 2847.737582723s` immediately before
`Conductor startup: apps enabled.` at 03:17:54Z (the boot began at 02:30:24Z). The previous
boot logged `3236.9s` and enabled at 01:22:19Z. Until then, every zome call on a cell that
is missing from the running map answers `CellDisabled`. The mechanism is already settled
in `genesis/data/timeline/backlog/fleet-standing-celldisabled-one-third-party-cell.md`
(2026-09-22, "startup is the window, and it is hours" — matthew measured 56 min there).
The storage-side Loki count of `CellDisabled` lines per 10 min, 00:50→03:20Z, was
168, 128, 26, 83, 26, 0…0, then 297, 270, 230, 229, 119. The last one came at 03:17:33Z,
on the lamad DNA `uhC0kZezl4k2nZa5ZyU5O5H-…` with matthew's own agent `uhCAkJH75E0E…`.
Across 11 h, the conductor was up and serving for a minority of the time.

**2. Between boots the conductor is CPU-pegged, so admission sheds the heal calls.** The
per-container CFS throttle ratio for `elohim-conductor` is **0.95** (instant), and
**0.87–1.00 in every populated hourly bucket for 48 h**. Admission capacity is 5 on every
pod. Matthew's shed (`elohim_conductor_admission_shed_total`) was 395 background + 312
interactive over 24 h, and 121 + 161 over the last 6 h, so the rate is accelerating. After
membership was restored (03:30Z → 04:08Z), all 28 `projection-reconcile` failures in
Loki were one of two strings:
- 16× `Request timeout: conductor admission: shed: no conductor permit for content_store within 1000ms (class=background, capacity=5, in_flight=5) — nothing was dispatched`
- 12× `Request timeout: heal conductor call exceeded per-attempt timeout 25s`

This is the same shape the 2026-09-13 section of
`genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md`
measured on adam. **matthew has joined adam's failure mode.** The admission gate
(`elohim/elohim-storage/src/conductor_admission.rs`, capacity derived from
`db_max_readers − 3`) is doing its job. It protects a conductor that has no CPU to spare.

**3. This is fleet-wide.** `healed = 0` over 24 h on all seven alpha storage pods. Only
gertrude shows any `refreshed`/`missing` output. The fleet-wide `healed` count never
exceeded 5 in any 6 h bucket across 7 days (2026-09-16 → 09-23). Before this window,
matthew was the pod that still healed (the 2026-09-13 control: `healed 35/24h`). It is
now the same as the rest. `divergent_actionable` is 51 on matthew, 49 on adam, 51 on eve,
51 on jessica and 47 on gertrude. matthew's gaps sit on the `rea` stream (`rea=51`,
`content=2`). Its `exhausted` count is still low (`rea=6`, fleet 49–146), which fits a pod
that entered this state recently.

**4. A reporting defect in the recovery observer. NAMED, not fixed here: the code is not
on `dev`.** From 03:18:13Z, the conductor's `ListCellIds` running map contains **all five**
of matthew's cells (`/db/p2p/conductor-diagnostics` → `cells.membership.runningCount: 5`,
`authoritative: true`, and the gauge `elohim_conductor_cell_running=1` for every role).
Storage logged `membership present; functional recovery unverified` for infrastructure,
lamad and node_registry at 03:18:13–03:19:34Z. mishpat recovered at 03:19:21Z and imagodei
at 03:21:17Z. The other three have logged nothing since. At 04:01Z the diagnostics
endpoint still reports them as `zomePath: "app-disabled"`, with `appDisabledReason:
"membership: this role's cell is ABSENT from the conductor's running-cell map … observed
state installed-not-running-app-enabled"` and `notRunningSecs` 5339 and still growing.
That contradicts the membership block in the same response, and the gauge
`elohim_conductor_cell_state{state="running-recovery-unverified"}=1`. Two code facts
together explain why this is invisible:
- `services/cell_probe.rs` logs a refused probe **at `debug!`** ("the cell probe was
  refused — the episode stands"). A `ProbeNow` probe fired against a conductor that is
  shedding at 5/5 fails without a trace, so the episode never closes on probe evidence.
- The `appDisabledReason` text is latched from the opening STRANDED observation. It is not
  rewritten when the state moves to `running-recovery-unverified`.

The running storage image is `elohim-storage:1.0.0-dev-571d9ea5`. `571d9ea54` is **not
an ancestor of `dev`**. It is 14 commits ahead on the in-flight
`push/2026-09-22-4.3-5.2` / `sprint/2026-09-22-storage-batch` line, and that line
includes `19c78d4af fix(storage): distinguish running-cell membership from app
enablement`, which introduced this observer. Any fix belongs to that sprint's owner.
Patching `dev` would fork the file under an unmerged branch. The consequence to fix
there: a role whose recovery probe is shed stays `app-disabled` on the diagnostics
surface. If serving is gated on that state, it also keeps `/health/serving` refusing
(503), even after membership proves the cell is running.

## Fix path

None of these is a bounded change in this tree:

1. **Ground the restart cause** (operator read). Pull the k8s Events for
   `elohim-matthew-alpha-conductor-0` from `event-exporter` for the six boundaries above.
   The last-terminated state is exit 255 / `Unknown`, which is not an OOM kill. The
   template's `startupProbe` budget is `40 × 15 s = 10 min`
   (`genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml:398-405`).
   A 47–54 min DHT init only survives that budget because `/health` on 8090 is served by
   the storage supervisor rather than the conductor. So check whether the probe or the
   supervisor's `process_manager` is killing a conductor that is still initialising.
   Suggestive, not proof: the 21:16→21:52 restart (36 min) came **mid-boot**, before a
   ~50 min init could have finished.
2. **CPU headroom for the genesis-pair conductor.** This is owned by
   `fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11.md`: raise
   matthew's conductor CPU limit (2 cores, throttled ~95%), sequence the roll, or add
   the demand-driven reconcile ramp. The ramp is an **actuation** loop, which is outside
   this agent's remit.
3. **Cold-start duration.** The conductor spends 47–54 min in `Dht::try_from_store` after
   every restart. This is owned by the cure order in
   `fleet-standing-celldisabled-one-third-party-cell.md` (conductor-side).
4. **The observer defect in item 4 above**, for the owner of the
   `sprint/2026-09-22-storage-batch` line. Log probe refusals at WARN once per episode,
   and rewrite `appDisabledReason` when the state moves to `running-recovery-unverified`.
   Also decide whether a probe refused by admission shed should count as "unverified"
   rather than "absent" for serving.

## Current decision

**BLOCKED.** The blocker has changed: it is now restart churn plus conductor CPU
saturation, not the rekey tail. matthew's projector heals nothing for three reasons, and
none of them can be fixed by a background agent on `dev`:
(a) the conductor restarts every ~1–3 h for an ungrounded exit-255 reason, and each
restart costs ~50 min of `CellDisabled`. That is a cluster/operator read and an
operator decision.
(b) Between restarts the conductor is ~95 % CFS-throttled, and admission sheds the heal
calls. That is a capacity/actuation decision owned by the fleet saturation record.
(c) The observer defect in the recovery path lives on an unmerged sprint branch, not
`dev`.

The rekey-era cure (lineage bridge or supersession for the 9 orphaned rows) still stands
for those rows, but it is not today's binding constraint. The poller should cite this
file. Ledger fp `6cdded115d74` is set to `status: blocked`.

## Verification

- Endpoints re-fetched 2026-09-23 03:44–04:01Z (all quoted above): `/admin/self-healing`,
  `/p2p/status` (×5), `/health` (`uptime 11335`), `/health/serving` (503),
  `/db/p2p/conductor-diagnostics` (×5). `healedTotal: 0` in every read, with sweeps 14 → 18.
- Prometheus (read-only): `elohim_projection_heal_outcomes_total` by pod/outcome over
  24h/6h/2h, 48 h hourly for matthew, and a 7 d fleet 6 h range;
  `elohim_conductor_admission_shed_total`, `_in_flight`, `_capacity`; the
  `elohim_projection_reconcile_{divergent_actionable,gaps,exhausted}` gauges;
  `kube_pod_container_status_{restarts_total,last_terminated_reason,last_terminated_exitcode}`;
  CFS throttle ratio; `container_memory_working_set_bytes`; `elohim_conductor_{cell_running,
  app_enabled,cell_state}`.
- Loki (read-only): the storage `elohim-node` transition and `CellDisabled` timeline, the
  conductor `DHT model initialised` / `apps enabled` timeline, and the post-recovery
  `projection-reconcile` error census.
- **Regression / clearing signatures.** matthew `elohim_projection_heal_outcomes_total{outcome=~"healed|refreshed"}`
  going nonzero means this is clearing. `kube_pod_container_status_restarts_total` on
  `elohim-matthew-alpha-conductor-0` staying flat for 6 h is the precondition. If
  `/health/serving` still reads `refused` after `cell_running=1` has held for 30 min, that
  is the item-4 observer defect, not the node.
