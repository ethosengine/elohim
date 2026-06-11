---
id: "backlog-ops-adam-pod-log-volume-saturates-loki"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam's alpha pod logs 26GB/day (~20× siblings) and saturates Loki into 502s — observability self-DoS"
slug: "ops-adam-pod-log-volume-saturates-loki"
written: "2026-06-11"
author: "agentic-developer (EPR durability arc, Phase 0)"
status: "backlog"
priority: "high"
tags: [ops, observability, loki, elohim-alpha, adam, log-volume]
cites:
  - genesis/manifests/humans/
---

# adam's pod log volume saturates Loki

## Symptom (2026-06-10, 24h window, `query_loki_stats`)

- `elohim-adam-alpha-0` (all containers): **94,141,822 entries / 25.9 GB** in 24h.
- `elohim-matthew-alpha-0`: 4,643,525 entries / 1.5 GB — adam is ~20× his sibling.
- `elohim-jessica-alpha-0`: ~778 MB across 48h.

During the EPR-arc Phase-0 investigation Loki degraded from slow to consistent
**502s on every query type** — including metadata-only label listings with the
simplest selectors — which blocked the cross-peer custody-sweep correlation
entirely (matthew/adam sweep evidence is still unverified because of this).
The log firehose is the prime suspect for the saturation: ~1,090 entries/sec
sustained from one pod.

## Why it matters

- Observability self-DoS: the pod most in need of investigation (adam's storage
  stayed projection-divergent for 10 days in the prior incident) is the one
  burying the tool used to investigate it.
- 26 GB/day of Loki ingest is real disk/retention pressure on the observability
  stack for zero diagnostic value if it's a spam loop.

## UPDATE 2026-06-11 00:30: burst-shaped, not steady-state

A 1-minute unfiltered window on adam (00:30-00:31Z) returned **32 lines**
(~46k/day pace — normal): sync rounds with 11 peers, content-inventory
serve/receive lines, holochain websocket-close WARNs, PTxnGuard-held WARNs.
So the 94M/24h was a **burst**, not a steady firehose. The burst chunks
remain poison: stats/metric queries touching adam's 06-09→06-10 windows
502 instantly while same-shape queries on matthew succeed. Burst-window
bisect deferred until Loki digests; prime suspect class is a
websocket/gossip reconnect storm window (matthew showed close-bursts at
20:50/20:52Z on 06-10).

## First actions (when Loki is responsive)

1. `{namespace="elohim-alpha", pod="elohim-adam-alpha-0"}` with NO filter over a
   **1-minute** window — see what's actually repeating (target module, level).
2. Check whether the volume is a crash/restart loop (jessica showed 5+ restarts
   in the same 24h; adam's restart cadence unknown) vs a hot loop in one
   tracing target (kitsune2 gossip retry storm and conductor websocket
   reconnect churn are both precedented shapes — cf. a885e19f7's doorway
   reconnect-storm fix).
3. Repo surface for the fix: per-human manifest logging env (RUST_LOG target
   filters) in `genesis/manifests/humans/`, or the upstream bugfix if it's a
   reconnect storm. Never kubectl from dev — manifests are the cleanup surface;
   a targeted RUST_LOG directive for the offending target is the likely shape.

shift_objective: |
  Identify what generates adam's ~1,090 log entries/sec (1-minute unfiltered
  Loki window + restart-cadence check), land the repo-side fix (RUST_LOG target
  filter in the human manifest or the actual loop fix), and verify adam's 24h
  Loki ingest drops to the sibling baseline (~1-2 GB/day) on the next deploy.
