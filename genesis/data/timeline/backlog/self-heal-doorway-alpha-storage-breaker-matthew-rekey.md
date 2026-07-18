---
id: "backlog-self-heal-doorway-alpha-storage-breaker-matthew-rekey"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway-alpha storage:8090 breaker exhaustion (circuit Open / half-open reshedding) is a FAITHFUL MESSENGER of a substrate defect: matthew-alpha (genesis-pair member) underwent a self-heal REKEY + DNA-reinstall at edge #1199 boot, orphaning its DHT anchors (divergentAnchor climbing, not reconciling) so its server-side read path sheds 503 catching-up; the doorway breaker + conductor-auth remint are working AS DESIGNED"
slug: "self-heal-doorway-alpha-storage-breaker-matthew-rekey"
written: "2026-07-18"
author: "runtime-triage"
status: "backlog"
priority: "high"
self_heal_status: blocked
severity: high
fingerprints: [b7b25f86fe13]
nodes: [doorway-alpha, elohim-matthew-alpha, intel-nuc]
relatedNodeIds:
  - "memory:project_local_stack_dht_anchor_gap"
  - "memory:project_alpha_topology_bootstrap_pair"
  - "memory:project_edge_deploy_restarts_genesis_conductors"
  - "memory:project_dna_hash_blind_to_coordinator_zomes"
  - "memory:project_p1_reconciliation_controller"
tags: [self-heal, circuit, doorway, storage-breaker, conductor-auth, genesis-rekey, anchor-divergence, catching-up, matthew, intel-nuc, operator-domain, substrate]
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
