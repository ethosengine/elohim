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
priority: "high"
tags: [incident, alpha, conductor, holochain, panic, sqlite, write-guard, adam, upstream, 0.6.3]
relatedNodeIds:
  - backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag
cites:
  - genesis/docs/content/elohim-protocol/history/2026-07-20-adam-slow-link-write-guard-saturation.md
  - genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
---

# Conductor source_chain unwrap-panic under DB-lock pressure (adam-only)

**Observed** 2026-08-05 ~04:27–05:23Z on `elohim-adam-alpha-0` (both the pre-Wave-2 pod and the post-restart pod — the class PREDATES the Wave-2 deploy and is 0.6.3-era or older; adam-only fleet-wide by Loki `sum by (instance)`).

**Mechanism (evidence-grounded):** adam runs sustained `PTxnGuard was held for 1.5–4.7s` warnings (`holochain_sqlite::db::guard`, the write-guard pressure silhouette from the 2026-07-20 slow-link history) → a source-chain transaction times out → upstream code `unwrap()`s the `Err(Timeout(Elapsed(())))` at `crates/holochain_state/src/source_chain.rs:499:22` → `FATAL PANIC` + holochain crash-report block (`/tmp/report-*.toml` in-pod). The edgenode wrapper restarts the conductor child, so the pod does NOT crashloop — k8s-invisible, log-visible only. Companion symptoms in the same windows: `validation_receipt_consumer` `DatabaseError(Timeout)` ERRORs, repeated `Failed to bind IPv6 listener: AddrInUse` on the websocket rebind after each child restart, app-port 4445 auth-timeout drops.

**Rate:** ~1 per 10–20 min steady-state; burst of ~7 in 4 min during the post-deploy restart churn (05:19–05:23), zero since 05:23 — decays with churn, consistent with load-proportional, not monotonic melt. Watch, don't grind.

**Routing:**
1. **Upstream contribution candidate** (ethosengine/holochain fork, `elohim-0.6.3` branch): replace the unwrap at `source_chain.rs:499` with error propagation/retry — an unwrap on a DB timeout is a crash where a backoff belongs. Same contribution lane as the tx5 zombie-fix.
2. **Local pressure lever** (existing history): adam's write-guard saturation is the 2026-07-20 composition defect — receipt/gossip pacing, not hardware. Any Wave-2+ soak that raises adam's panic rate above the ~1/10min floor is a regression signal for the dual-mode fan-out.
3. **Probe:** Loki `{namespace="elohim-alpha"} |= "FATAL PANIC"` rate per instance; adam-only and decaying = known; fleet-spread or sustained-rising = escalate.
