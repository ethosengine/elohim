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

**Routing:**
1. **Upstream contribution candidate** (ethosengine/holochain fork, `elohim-0.6.3` branch): replace the unwrap at `source_chain.rs:499` with error propagation/retry — an unwrap on a DB timeout is a crash where a backoff belongs. Same contribution lane as the tx5 zombie-fix.
2. **Local pressure lever** (existing history): adam's write-guard saturation is the 2026-07-20 composition defect — receipt/gossip pacing, not hardware. Any Wave-2+ soak that raises adam's panic rate above the ~1/10min floor is a regression signal for the dual-mode fan-out.
3. **Probe:** Loki `{namespace="elohim-alpha"} |= "FATAL PANIC"` rate per instance; shem-cohort at ≤1/30min post-churn and decaying = known; any ethosengine-node peer, or sustained ≥2/30min per instance outside a restart window = escalate.
