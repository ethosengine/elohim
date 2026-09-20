---
id: "backlog-no-latency-metric-for-change-doorbell"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "No change-to-peer-notified latency metric exists on either transport plane, so 'announce on both' (story 4.3) cannot be measured before/after"
slug: "no-latency-metric-for-change-doorbell"
written: "2026-09-20"
author: "serving-edge failover-balance-stream campaign, 2026-09-20 review"
status: "open"
priority: "medium"
jobs: [elohim]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [dataplane, iroh, libp2p, observability, streaming, doorbell]
---

**The fact.** In `TransportBackend::Dual` mode, the iroh announce bridge is deliberately not spawned:
`elohim/elohim-storage/src/main.rs:4500` guards `spawn_iroh_announce_bridge` behind `if p2p_node.is_none()`,
and the surrounding comment (`:4494-4498`) states why — in dual mode the libp2p arm already spawned the
listener against the same shared `Arc<SyncManager>`, and a second one would race the projection
read-modify-write on every content write. So an iroh-only peer in a mixed household learns of a change
only through the 60s round driver, not the eager doorbell. The sender side is complete
(`p2p_iroh::announce_change`, `spawn_iroh_announce_bridge` / `announce_local_change`), and the storage
metrics registry has counters for iroh sync request outcomes (`IROH_SYNC_REQUESTS`,
`elohim/elohim-storage/src/metrics.rs:1663-1670`, labeled `kind`/`result`) — but no histogram or gauge
anywhere in `metrics.rs` measures time from a local change to a peer's notification, on either the iroh
or the libp2p plane.

**Evidence.** `elohim/elohim-storage/src/main.rs:4494-4527` (the guard, its rationale comment, and the
`spawn_iroh_announce_bridge` call site); `elohim/elohim-storage/src/p2p_iroh/announce_change.rs` (the
complete sender, `spawn_iroh_announce_bridge` / `announce_local_change`); `elohim/elohim-storage/src/metrics.rs:1663-1670`
(`IROH_SYNC_REQUESTS`, the closest existing metric, confirmed by full-file grep to be the only
announce-adjacent series — no `_latency_` or `_seconds` metric matches `announce`/`sync`/`notif`/`doorbell`
anywhere in the file).

**Why it matters.** Story 4.3 ("iroh peers get the doorbell") is scoped as "measure it, then announce on
both" — but there is nothing to measure change-to-peer-notified latency with today, on either transport.
Without a shared metric, landing the iroh announce arm for dual mode has no before/after number to point
to, and the existing comment's own claim (an iroh-only peer "waits the 60s round") is currently a code
inference, not a measured fact.

**Smallest next step.** Add one histogram (e.g. `elohim_change_notified_latency_seconds{plane}`) recorded
at the point a peer's sync-apply observes a change that originated from another node's announce — the
natural site is wherever `IROH_SYNC_CHANGES_APPLIED` already increments, paired with a timestamp carried
in the announce payload or approximated from the announce-received log line. Measure the current libp2p
plane's latency first (it is the only one wired end-to-end today) as the baseline story 4.3 needs before
touching the dual-mode guard. Household lever for varying which peers run which plane:
`MESH_PEER_TRANSPORTS` (`app/elohim-app/scripts/hc-mesh.sh:73,267-292`, e.g. `matthew=libp2p,jessica=iroh`).

**Links.** Plan: `genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md`
story 4.3 (corrected 2026-09-20 note under story 4.1). Habit:
`elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`.
