---
id: "backlog-conductor-residual-cpu-full-chain-read-and-perpetual-republish"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Alpha conductors stay near their CPU limit after the FK-787 cure: a full source-chain read per call, the bulk Entry fetch, the CapGrant read, and a republish that never completes receipts"
slug: "conductor-residual-cpu-full-chain-read-and-perpetual-republish"
written: "2026-09-28"
author: "claude-opus-5-5 (shift 2026-09-28T03-00-land-fleet-conductor-cpu-cure)"
status: "open"
priority: "high"
severity: medium
nodes: [alpha, household]
tags: [conductor-fork, cpu, sqlite, publish, source-chain, dataplane-convergence, idle-is-free, performance, perf-cpu, perf-io, perf-scale, trustful-self, friction-verify, friction-mechanical, plane-notary, unit-history, phase-steady, lane-borrowed]
cites:
  - genesis/a2o/reports/recovery/fleet-cpu-publish-livelock-2026-09-28.md
  - genesis/a2o/reports/recovery/k0-window-e0bfc6c7a-vs-25dd2d0be.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
---

# Residual conductor CPU after the FK-787 cure

**Context.** On 2026-09-28, edge #1492 rolled conductor pin c8c17202c (fork `fix/fleet-cpu-publish-livelock`: the FK-787 livelock fix, covering indexes for publish and get, EXISTS continuation, bounded WAL checkpoints) onto alpha. On the new-image conductors FK-787 went to 0, and slow statements are roughly ⅓–½ of the old image's over the same window. CPU, however, still sits at or near the limit (susan 1.39–1.5/1.5; james 1.0/1.0) 30 minutes after the roll.

**Residual cost, ranked by summed slow-statement seconds (Loki, 20 min, susan new vs matthew old).** The same shapes appear on both images:

1. `SELECT … FROM Action WHERE author = ? AND record_validity = ? ORDER BY seq ASC`: the full source chain per call (`rows_returned=33608`, ~8 s). Fork `cb61633c2` ("push ChainQueryFilter's sequence range into the chain read") targets exactly this. It sits in the unbisected 7e553f9c3 range and is not on c8c17202c.
2. `SELECT hash, blob FROM Entry WHERE hash IN (…) UNION ALL … PrivateEntry`: the bulk entry fetch.
3. `SELECT cg.action_hash, cg.cap_access, cg.tag … CapGrant`: fixed by fork `61565f320` ("authorise a zome call in one query per access class, not 3N+"), also in the unbisected range.
4. **Perpetual republish.** `receipts_complete` needs 5 receipts; the household never gets more than 2 per op, and the fleet shows publish batches of 90,685 and 133,891 ops completing in full. After the FK fix every authored op is republished every `min_publish_interval` (5 min) forever. Unverified at fleet scale: measure the publish-cycle cost before designing. Candidate designs: cap required receipts at the number of reachable authorities, or back off republishing exponentially.

**Next bounded step.** Cherry-pick `cb61633c2` and `61565f320` onto `fix/fleet-cpu-publish-livelock`. Neither is a named convergence suspect (those are `0efa40939`, already taken and ruled on, and `ff2ea44c6`). A/B them on the K0 store copies, then run the household receipt, then roll. Measure the republish cycle on the fleet (publish lines/min, ops per batch, CPU during a cycle) before choosing between the receipt cap and backoff.

**Why it matters (fleet, 2026-09-28 after the full roll).** matthew hosts 84 agents. Its conductor was back at 2.0/2.0 cores within ~25 min of rolling (08:55Z) and storage admission returned to 5/5. App #1737's channel-bind PATCH answered 503 catching-up for its whole 300 s budget and the release deferred, so this residual now blocks the dataplane-convergence N6 leg. Done when: matthew's conductor stays below its limit for 30 min with the hosted cast, storage admission stays below 5/5, and an App run's notarize/bind PATCH lands on alpha.

**Also seen.** On 2026-09-28 at 07:53:24Z, james logged a `lineage_partition` WARN, "SPACE PARTITIONED — … empty storage arc, no peer has completed a gossip round" (`peer_timeouts=21`), during the rolling window. It may be roll-transient; re-check after the roll settles.
