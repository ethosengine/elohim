---
id: "backlog-conductor-source-chain-unwrap-panic-db-timeout"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Conductor FATAL PANIC on adam under DB pressure — source_chain.rs:499 unwraps a sqlite Timeout; wrapper-restart recovers but the class is an upstream unwrap-on-timeout defect"
slug: "conductor-source-chain-unwrap-panic-db-timeout"
written: "2026-08-05"
author: "Wave-2 landing shift (2026-08-05T03-57-land-shakeout-wave2-relay-dual) — Loki evidence, adam-alpha"
status: "open"
priority: "critical"
tags: [incident, alpha, conductor, holochain, panic, sqlite, write-guard, adam, upstream, 0.6.3]
relatedNodeIds:
  - backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag
cites:
  - genesis/docs/content/elohim-protocol/history/2026-07-20-adam-slow-link-write-guard-saturation.md
  - genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
---

# Conductor source_chain unwrap-panic under DB-lock pressure (adam-only)

**Observed** 2026-08-05 ~04:27–05:59Z, primarily `elohim-adam-alpha-0` (both the pre-Wave-2 pod and the post-restart pod — the class PREDATES the Wave-2 deploy and is 0.6.3-era or older). **Scope correction (05:59Z):** single hits also on eve and susan during the churn settle — all three are shem-node residents (matthew/jessica/james on the ethosengine node showed zero), so the class tracks NODE-level DB/IO pressure on shem (4 full-arc conductors co-resident), not an adam-specific defect. adam decayed 7-in-4min → 1-in-35min as churn settled.

**Mechanism (evidence-grounded):** adam runs sustained `PTxnGuard was held for 1.5–4.7s` warnings (`holochain_sqlite::db::guard`, the write-guard pressure silhouette from the 2026-07-20 slow-link history) → a source-chain transaction times out → upstream code `unwrap()`s the `Err(Timeout(Elapsed(())))` at `crates/holochain_state/src/source_chain.rs:499:22` → `FATAL PANIC` + holochain crash-report block (`/tmp/report-*.toml` in-pod). The edgenode wrapper restarts the conductor child, so the pod does NOT crashloop — k8s-invisible, log-visible only. Companion symptoms in the same windows: `validation_receipt_consumer` `DatabaseError(Timeout)` ERRORs, repeated `Failed to bind IPv6 listener: AddrInUse` on the websocket rebind after each child restart, app-port 4445 auth-timeout drops.

**Rate — CORRECTED 2026-08-05 12:15Z; the earlier "decaying" read does not survive a wider window.** The original note read the 05:23–05:59Z lull as decay. A two-window `count_over_time` comparison says otherwise:

| Window | adam | eve | susan | gertrude | total |
|---|---|---|---|---|---|
| 04:30–06:30Z | 20 | 1 | 1 | 1 | **23** |
| 10:10–12:10Z | 32 | 1 | 0 | 1 | **34** |

Total is **up ~48%**, and the *shape* changed from one sharp burst (11 in the 05:20–05:30Z bucket, quiet either side) to **steady 2–5 per 10-min bucket throughout** the late window. adam is >90% of both. So: steady-to-mildly-escalating, adam-dominant — **not** decaying, and adam alone (~1 panic per 3.75 min) is ~4× the escalation threshold this item's own probe defines below. The lull that produced the original reading was a post-churn trough, not a trend.

**Live user-visible consequence (new, 2026-08-05):** doorway-alpha-b pins its ZomeCaller to a single conductor — `elohim-adam-alpha:4444`/`:4445` — so adam's write-guard contention surfaces directly as doorway-B request hangs. `GET /api/v1/federation/doorways` has been timing out continuously from at least 06:30:47Z through 12:08:50Z (client aborts at ~4.95s; server-side `authorize_signing_credentials timed out after 10000ms for role 'infrastructure'` and `Zome call timed out … infrastructure/get_all_doorways`). Cache-served bootstrap traffic on the same pod succeeds at 20–40ms throughout, so this is precisely the DHT-dependent path, not the pod. **Federation doorway listing is effectively dead on B while adam is under pressure.** Two follow-on levers worth weighing alongside the routing fix: give the doorway a multi-conductor ZomeCaller fallback (single-conductor pinning makes one peer's health a doorway SPOF), and/or reduce adam's `target_arc_factor` (`project_per_node_memory_is_conductor_authority_arc`).

**Not attributed to Wave-2.** Zero `iroh|relay|transport_backend|DualGossip|RelayMode` hits anywhere in doorway-alpha-b's 8h log window, and the panic class was present on adam's *pre*-Wave-2 pod. Whether the dual-transport fan-out raised adam's rate is **unresolved** — adam's own conductor logs were not checked for transport strings, and the +48% is circumstantial. That check is the cheapest next probe.

## 2026-08-06 — doorway ZomeCaller SPOF removed (fallback lever LANDED, code-only)

The second of the two follow-on levers named above ("give the doorway a multi-conductor ZomeCaller fallback") is **implemented**. The upstream unwrap defect and the arc-factor lever are **untouched** — this bounds the blast radius, it does not cure the panic.

**What changed** (`doorway/doorway-service`, no new config schema, no manifest value changes):

- `ZomeCaller` now holds a **pinned primary** (unchanged: `--conductor-url` / `CONDUCTOR_ADMIN_URL`) plus an **ordered fallback list derived from the conductor pool the doorway already declares** (`CONDUCTOR_URLS`, with the primary and duplicates filtered out). A single-conductor deployment dedupes to zero fallbacks and behaves byte-identically to before.
- **Per-conductor signing credentials — the crux.** `authorize_signing_credentials` grants a cap on *that* conductor's cells for *that* conductor's agent, so a primary credential is worthless on a fallback. Both paths now go through one shared `connect_endpoint()` which builds a **fresh `ClientAgentSigner`** and authorizes against **that endpoint's own admin interface** (derived per-entry by the established app-port-minus-one convention, the same derivation `discover_existing_agents` already applies). There is no shared signer field on `ZomeCaller`; that absence is load-bearing.
- **Primary availability cooldown (60s).** Naive "always try primary first" failover would have been a **no-op for this incident**: the primary deadline is 10s and the SPA aborts at ~4.95s, so the route stays dead for the whole outage even with a healthy fallback. After one availability failure the sick primary is skipped entirely; it is re-probed once the cooldown expires (half-open) and reclaims affinity on the first success — no flap logic, no operator action.
- **Failure-class split gates the routing.** Only *availability* failures (connect/list_apps/authorize/token/app-ws timeouts, dead socket, call timeout) fail over. An *application* error — the conductor answered — is returned as-is, since every peer in the pool would answer identically; failing over on it would relocate the same error and double the load. The public API still returns `String`, so downstream matching (`e.contains("already exists")`) is unaffected.
- **Failover is opt-in per call site, and deliberately narrow.** Failing over changes *which agent signs*, so only agent-agnostic DHT reads use it: `get_all_doorways` (the dead route) and `find_publishers` moved to `call_failover` / `call_zome_failover`. `create_human`, `register_doorway`, `update_doorway`, `record_heartbeat` stay **primary-pinned** — a Human binds to the acting agent, and a doorway's own registration authored by a second agent is a validation question nobody has answered yet.

**Why not re-pin doorway-B to a healthier peer instead.** Re-pinning only relocates the SPOF and discards the 2026-05-27 co-location rationale (doorway-B on shem reads its co-located genesis peer adam; re-pinning to matthew re-adds a cross-WireGuard hop on every read). Failover keeps the co-location default and degrades only when adam actually can't answer.

**Verification:** doorway crate green on all four gates (`build --release`, `test --lib --bins` 925 passed / 0 failed, `clippy -- -D warnings`, `fmt --check`). New tests in `services::zome_caller::tests` pin: pool-minus-primary derivation; each fallback authorizing against its **own** admin interface (the credential crux); availability-vs-application class gating; single-conductor no-op equivalence; cooldown skip-then-re-probe and recovery-reclaims-affinity; fallback rotation wrap; and an end-to-end routing proof against closed ports asserting the fallback was attempted and the primary is left in cooldown.

**Deploy:** code-only. `CONDUCTOR_URLS` is already injected into both `alpha.yaml` and `alpha-b.yaml` by `computeConductorUrls` in `elohim/holochain/Jenkinsfile`, so the fallback list populates on the next doorway image roll with **no manifest value change**. Both manifests got a comment naming the new consumer (trimming `CONDUCTOR_URLS` now silently removes federation read failover). Confirm on boot via the log line `ZomeCaller created for federation (… fallback conductors: N)` with `N > 0`, and during a primary outage via `Served from FALLBACK conductor (primary unavailable)`.

**What remains — explicitly NOT done here:**
1. **The upstream unwrap at `source_chain.rs:499`** (routing item 1 below) is untouched. This item stays open on that basis; doorway no longer *surfaces* adam's panics as a dead route, which also means the panic rate is now less user-visible — keep watching the Loki probe, not the doorway route.
2. **adam's `target_arc_factor`** (routing item 2) is deliberately left as an **operator decision** — it trades DHT authority coverage for RAM/IO headroom on a 4-full-arc-conductor node, and that is a substrate-coverage call, not a doorway one.
3. **Write-path failover** (`register_doorway` / `record_heartbeat`) is open by design. During a long primary outage doorway-B's heartbeat still stops, so peers may mark it stale. Resolving it needs an answer to "what does a second author do to a `DoorwayRegistration`'s validation" — a zome question, not a doorway one.
4. **Premise-affinity ordering of the fallback list.** `CONDUCTOR_URLS` is the whole alpha pool, both premises, so doorway-B's fallback may cross WireGuard to an ethosengine-node peer. That is a correct degraded path (any peer answers a DHT read), but ordering same-premise peers first would need premise metadata the pool does not carry. Follow-on, not a blocker.

**Routing:**
1. **Upstream contribution candidate** (ethosengine/holochain fork, `elohim-0.6.3` branch): replace the unwrap at `source_chain.rs:499` with error propagation/retry — an unwrap on a DB timeout is a crash where a backoff belongs. Same contribution lane as the tx5 zombie-fix.
2. **Local pressure lever** (existing history): adam's write-guard saturation is the 2026-07-20 composition defect — receipt/gossip pacing, not hardware. Any Wave-2+ soak that raises adam's panic rate above the ~1/10min floor is a regression signal for the dual-mode fan-out.
3. **Probe:** Loki `{namespace="elohim-alpha"} |= "FATAL PANIC"` rate per instance; shem-cohort at ≤1/30min post-churn and decaying = known; any ethosengine-node peer, or sustained ≥2/30min per instance outside a restart window = escalate.

## 2026-08-06 — the "did dual fan-out raise adam's rate?" probe RAN: verdict (b), transport exonerated at the resource level

The cheapest-next-probe named above (adam's own conductor logs + resource correlation against the deploy boundary) has now run, both legs.

**Log leg (Loki).** Dual-enable deploy timestamped precisely: **2026-08-05T05:06:28.578Z** ("Dual: DualGossipPublisher wired into P2PNode" boot line, pod `eff0fb4d`). Equal 15.1h windows straddling it: **102 panics before → 145 after (+42%)** — the rise is real and time-correlated. But every sampled panic, both windows, carries the byte-identical signature (`source_chain.rs:499` unwrap on `Timeout(Elapsed(()))`, preceded by `PTxnGuard` holds of 1–5s and `Database read connection is saturated` up to 22,662%), and the transport strings near panics are exclusively `kitsune2_gossip` / `kitsune2_core::factories::core_fetch` / `holochain_p2p` — Holochain's own DHT layer. **Zero `iroh`/`libp2p`/`DualGossipPublisher` frames in any panic path, either window.**

**Resource leg (Prometheus).** No step-change at the deploy boundary in any exported pressure metric: CPU 6.68→6.40 avg cores (flat-to-lower), CFS throttling 3.49→3.23 (flat), disk I/O 881KB/s→576KB/s avg (*lower* after); the single largest I/O event of the 48h window (~4.2MB/s sustained ~19:00–19:30Z Aug 4) predates the deploy by 10h. RSS/thread curves show ordinary per-pod-lifetime warm-up, no deploy discontinuity.

**Verdict: (b)** — a pre-existing SQLite read-pool-saturation panic class whose +42% frequency rise is **not explained by measurable host-resource pressure from the dual-gossip deploy**. The one variant still open is *concurrency* contention (dual gossip raising `holochain_p2p` RPC volume rather than host load) — untestable today because no RPC-rate metric is exported. Routing item 2's soak-regression rule stands, but reds against it should be read against this baseline: the class predates Wave 2 and rose without a resource signature.

**Two observability gaps surfaced by the probe (both blind spots on the fleet's worst actor):**
1. **The conductor crash-loop is invisible to `kube_pod_container_status_restarts_total`.** adam's pod `c0b4a5e7` (live since 2026-08-05T14:40Z) absorbed ~145 conductor panics with the K8s restart counter pinned at 0 — an in-container supervisor respawns the panicked binary below the K8s floor. Restart-count alerting cannot see this class; the Loki `FATAL PANIC` rate probe (routing item 3) is currently the *only* working detector.
2. **No live DB-pool saturation metric.** The only DB-pool-adjacent export is `elohim_node_db_max_readers` — a static config ceiling (flat 16). The saturation figures driving this whole item (350%–22,662%) exist only as log strings. Exporting a read-pool utilization/queue-depth gauge from the conductor (or scraping it from elohim-storage's side) would make this item's probe quantitative and alertable.

**Independent review 2026-08-06 (verdict: ship).** Credential invariant confirmed structurally (no signer field exists to share; every endpoint authorizes against its own admin interface), write/read split verified at all call sites. Follow-ons from review, tracked here, none blocking: (a) boundary requests still exceed the SPA's ~4.95s abort — first request against a freshly-degraded primary pays the full primary deadline before cooldown starts, and one request per cooldown expiry pays the half-open re-probe (the 30s background `resolve_epr_storage_pool` refresh usually absorbs this for `get_all_doorways`, but `find_publishers` has no periodic caller and pays it inline); (b) no single-flight on the cooldown-expiry re-probe (N concurrent requests each probe); (c) no test yet for the "primary down, fallback answers Ok" happy path (needs a mock conductor; the credential crux is structurally enforced regardless). Module doc corrected to state the boundary caveat.
