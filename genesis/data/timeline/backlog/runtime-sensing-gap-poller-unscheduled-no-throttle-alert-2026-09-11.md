---
id: "backlog-runtime-sensing-gap-poller-unscheduled-no-throttle-alert"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The runtime self-report harvester had 66 polls and an empty window while the apex node reported a stuck provide loop for days, and no alert rule watches conductor CFS throttle — a week of 100 % saturation reached no human and no agent"
slug: "runtime-sensing-gap-poller-unscheduled-no-throttle-alert-2026-09-11"
written: "2026-09-11"
author: "shift 2026-09-11T09-00-land-batch-2c338124a (integrator)"
status: "open"
priority: "high"
tags: [runtime-harvest, findings-sentinel, alerting, prometheus, conductor, sensing, D8]
relatedNodeIds: []
cites:
  - .claude/scripts/runtime-harvest.py
  - genesis/data/timeline/backlog/dataplane-reanchor-dead-remaining-rekeyed-peer.md
  - genesis/data/timeline/backlog/fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11.md
---

# The node told the truth; nobody was polling

## Observed (2026-09-11)

- `.claude/data/runtime-cursor.json` read `{poll_index: 66, windows: {}}` and `runtime-findings.jsonl` was empty. One hand-run poll (`python3 .claude/scripts/runtime-harvest.py`) stored samples for both doorways and IMMEDIATELY filed `2b4761b2eaf6 provide-loop-dead-remaining-stuck` on alpha-b (`reanchorDeadRemaining=9, stuckSweeps=168, deadRemainingStuck=true`) and dispatched runtime-triage, which canonicalized it (blocked — fix lands in the other lane's storage WIP). The elevate arm works when it runs; it was not running.
- Prometheus has carried `container_cpu_cfs_throttled_periods_total / container_cpu_cfs_periods_total = 1.0` on gertrude/susan/eve conductors for 7 days and on adam for ~24 h. No alert rule, no ledger finding, no notification.

## Why it matters

The findings-sentinel pattern (flag → agent → canon → stasis) is only as good as its flag. A poller that never samples and a metric nobody thresholds are both "honest absence" from the node's side and a dump from ours: the exhaustion sat in plain view until a pipeline red made a human ask.

## Fix (bounded)

1. Schedule the harvester: a `/loop`-fired or cron-fired poll every 15 min while a session is berthed, with the cursor's `windows` non-empty as its own health check (an empty window after N polls is itself a finding: `harvester-blind`).
2. Add a Prometheus alert rule (repo-owned under `genesis/manifests/` monitoring rules): conductor CFS throttle ratio ≥ 0.95 for 30 min → warning; ≥ 0.95 for 6 h → the runtime ledger (via the harvester reading Alertmanager, keeping the no-runtime-write rule).
3. Add `conductor-throttle-sustained` as a harvester class fed from Prometheus, so the same ledger carries both self-reports and external saturation.

## Done when

- `runtime-cursor.json` windows are non-empty on every poll for a week; the harvester files a finding within one poll of a synthetic stuck state on the household mesh.
- A throttle alert fires in Alertmanager against the live fleet state as measured today.
