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
tags: [conductor-fork, cpu, sqlite, publish, source-chain, dataplane-convergence, idle-is-free]
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

## 2026-10-01 bounded source follow-up; runtime proof owed

Fixture hygiene 8d6b70468a83f913fe96eed3bb2c784169db07ef and core 2b334df7973d05eab4d72a75006275bcec770321 preserve clean lineage from 517d4c83517b2f9a30f9f69b3b448a73e2d76670. Strict owning-library/test lint and 45-file formatting passed; final data 154 library + 10 integration and state 199 library + 1 cost test passed with zero failures or ignored tests. The Create/Update prefilter bounds the link-heavy fixture at four headers for ten versus 2 000 unrelated links; 5 760 differential result shapes agree. It does not solve AppCreate-heavy entry-type reads (2 001 headers for 2 000 App Creates). The exact 2b334 production pair built at 12:21 UTC; the owned household restart at 12:28 preserved fifteen exact source-chain tips, all installed apps and identity files. Current serving report sprint-report-household-20261001T123345Z-09360a0e passed 5/5 scenarios and 102/102 steps in 23m40s; the current-component receipt checker is GREEN on the host seat. Fleet rollout, actual credential acceptance and sustained live cost/admission/notarize proof remain owed; perpetual-republish cost is not measured by these tests.

Separate operator floor: authenticated C4 admin listApps failed with SQLite code 14 CANTOPEN at 10:40 UTC before native grant authorization. The exact failing database pathname and OS error were not recorded; a shared DNA-definition-store attribution is an inference, not proof from deployed 82fb source. Edge 1517 console line 36203 held Gertrude conductor image elohim-storage:1.0.0-dev-82fb07f1; line 36799 confirms no native restart or settle gate while storage rolled to 5ffe. Operator evidence subsequently confirmed EDQUOT and zero dataset availability; specific CANTOPEN database attribution remains unproved. Recovery proof is restored native read/write availability; the existing 8445-to-localhost-4445 proxy remains a separate transport check. Do not classify this floor failure as credential denial or reset identities to recover it.

## 2026-10-01 operator-confirmed EDQUOT; backup pruning selected

Gertrude holochain-data: quota 20G unchanged, available zero, live 5.27G, snapshots 14.7G, 37 hourly autosnap snapshots plus syncoid history; file creation returned EDQUOT. wasm.db is 3.16G with no observed WAL/SHM or open handle. Adam/Eve available zero; Susan available 1.28G. Eve's approximately 90 CrashLoop restarts have no established cause here. These are operator-reported facts, not fresh repository probes. The selected recovery is pruning backups while preserving the newest usable restore point and required syncoid incremental base, not increasing quotas. No pruning occurred in this session.

Trace the runaway growth separately from the below-50MB seed: inspect a consistent offline copy for compiled/raw WASM payload, DNA/zome references, allocated/free SQLite pages, DHT/WAL bytes and snapshot changed-space history. The old measured household device-health/WAL driver is not proof of the current fleet mechanism. Confirmed EDQUOT establishes the write floor failure while the exact C4 CANTOPEN database attribution remains uncertain; do not classify a storage-floor failure as credential denial or reset peer identities. Detailed evidence and inspection constraints are retained in the existing 2026-09-24-conductor-store-growth-report.

Current-source differential: federation.rs already expires dead roster members after three rounds and grace-probes at most sixteen every twelve rounds (300-second configured round), with explicit deregistration. Do not carry the historical unbounded device-health writer forward as the current fleet cause. Candidate 2b334 stores raw/compiled WASM per hash via INSERT OR REPLACE and retains per-agent DNA/zome references; deployed 82fb schema and growth remain unmeasured. Attribute current attestation rate, distinct WASM/compiled payload, reference multiplicity, free pages/WAL and snapshot changed-space independently on consistent offline evidence.

- 2026-10-02 campaign 1.4: same-cell ceremony scheduling is repaired and tested, but exact pending-head acceptance still exceeds its 4,000 ms native verification budget; lock-free local grant/approval reads took 9,313/10,873 ms with only 28 actions between issuance and approval. Smallest unblock is a focused native record/acceptance cost fix preserving all authority checks and deadlines; deferred per-call-cost boundary, no push. Evidence: `genesis/a2o/reports/recovery/campaign-1.4-restart-20261001/acceptance-cost-stop.json`.

- 2026-10-02 campaign 1.4: operator-approved 30s native acceptance clears the old refusal, but the actual 75s story expires during exact Human witness exercise; reduce the existing approval/witness critical path without weakening checks, then diagnose doorway B projection; report `campaign14-household-budget30-20261002T0110Z-1`, receipt `genesis/a2o/reports/recovery/campaign-1.4-restart-20261001/acceptance-budget-30s-stop.json`; no push or performance implementation.

- 2026-10-02 operator split: land functionality independently of the deferred latency campaign; collect approval/witness cost, native-election read cost, and projection-trigger/adoption delay for a deep dive on the new shem workspace. Existing-witness verification took 11,615/8,979 ms; approval attempts took 19,697/21,480 ms but refused an expired mandate, so they are not successful approval timings. Jessica later adopted the verified head (trigger-to-adopt 49,897 ms), and both doorways served identical head/body at 02:24:30Z, outside the closing deadline. Evidence: `approval-witness-latency-20261002.json`, `approval-witness-latency-20261002-window1/`, `recovered-head-doorways-20261002.json` under the existing persistent recovery directory; read-only handoff `genesis/docs/superpowers/sprints/2026-10-02-campaign-1.4-claude-isolated-handoffs.md`. No performance implementation or timed-pass claim in this push.

- 2026-10-02 operator-directed shem handoff: include the doorway cold-gate cycle finding (Clippy 20m45s, test compile 5m51s, library tests 77.17s; 1,743 passed, two ignored), alongside the approval/witness and adoption timing evidence. Functional landing does not start gate architecture work. See campaign-1.4-claude-isolated-handoffs and the existing architecture-findings ledger.

- 2026-10-02 shem investigation inputs: all three fixture receivers earned/elected the second head while both doorways still served the first body at 03:44:15Z; head reads took 22,333–36,044 ms versus authenticated hydration's 25s outer wait. Che is validating the necessary alignment to the existing 60s conductor bound, preserving batch budgets and authority checks. Separately, serving report `household-campaign14-serving-20261002T0302Z` passed 1/5 stations, with cleanup/setup/health timeouts; retain these boundaries for the shem deep dive, not as successful proof.

- 2026-10-02 shem follow-up inputs: final storage gate passed 4,263 library tests and its integration suites; the gate-cycle reporter measured 13 minutes against its 600-second soft ceiling (`push-gate-storage-retained-hint.log`). Runtime source `659b47803` repairs a retained-hint wakeup gap through the existing bounded verifier. Its startup queue also revisits anchored device-health rows; measure queue wait separately from verification time. Repeated HTTP head reads can outlive observer timeouts in native calls, so the functional observer now waits on cached body/anchor readings before its final head checks. These are investigation inputs, not permission for a performance redesign in Che.
- 2026-10-02 shem input after functional cancellation repair `03f107651`: full storage gate passed (4,271 library tests, all integration suites and doctests), with a 13-minute cycle above the 600-second soft ceiling and below the 1,200-second hard ceiling (`push-gate-storage-cancel-safe.log`). The repair retains existing write locks/capacity through the offered response; read cancellation behavior is unchanged. Measure remaining native work after read observers expire separately on Shem; no read-cost result or performance redesign is claimed here.
