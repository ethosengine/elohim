---
id: "backlog-rea-inventory-advertises-rows-of-another-notary"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Peers advertised commitments only the author's mishpat DNA can answer for; every receiver kept asking its own content_store for them, 63 to 109 get_rea_commitment calls a minute per peer"
slug: "rea-inventory-advertises-rows-of-another-notary"
written: "2026-10-12"
author: "claude (idle call-floor sprint 2, operator-directed)"
status: "wip"
priority: "medium"
severity: medium
nodes: [alpha, household]
relatedNodeIds:
  - "habit:idle-is-free"
  - "habit:dataplane-convergence"
tags: [elohim-storage, projection-reconcile, rea, mishpat, commitments, performance, perf-cpu, perf-queue, idle-is-free, lesson, cross-namespace-identity, trustful-declared, friction-verify, friction-wait, plane-projection, plane-value, unit-item, unit-peer, phase-steady, lane-background]
cites:
  - elohim/elohim-storage/src/p2p/view_federation.rs
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/db/rea_commitments.rs
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
  - genesis/data/timeline/backlog/rea-stream-no-divergence-adjudication-drain-path.md
  - genesis/data/timeline/backlog/provide-revoke-arm-dormant-behind-namespace-mismatch.md
---

**What.** The "stale REA cohort" named in `idle-is-free` DELTA 2026-10-12d was about 800 commitments per peer that
peers advertise and the local conductor cannot see, re-read on the miss ledger's dormancy ladder at 63 to 109
`get_rea_commitment` calls a minute per peer whenever a rung lapses. It was not slow gossip and not divergence. The
ids are mishpat entry hashes, and they were advertised to peers that could never resolve them.

**What the census showed (2026-10-12, measured).** The ids are mishpat entry hashes (`uhCEk...`) that
`mirror_replication_commitment` (`src/db/rea_commitments.rs`) writes into `rea_commitments` on the authoring peer
only: 729, 613 and 202 on matthew, jessica and james, with no id shared between any two peers. Advertised but absent
per receiver: 818, 937 and 1353, of which 815, 931 and 1343 are `replicates-commons`. Each of those is present in
the advertiser's `mishpat_commitments` and in no receiver's.

**Why the receiver could never heal it.** The reconcile inventory responder (`build_filtered_inventory_payload` in
`src/p2p/view_federation.rs`, the unfiltered REA branch) advertised every `rea_commitments` row. The receiver
(`discover_rea` in `src/p2p/projection_reconcile.rs`) heals by asking its own conductor
`content_store::get_rea_commitment`. That function resolves ids through the content_store zome's `commitment_id`
anchor, so it can never answer for a mishpat entry hash, on any conductor, including the author's own.

**Live confirmation (measured).** The 300 s window after a storage restart at 2026-10-12T20:18Z
(`genesis/a2o/reports/recovery/idle-call-floor-20261012/after-cold-boot-summary.txt`) shows `get_rea_commitment`
819, 937 and 1354 calls on the three peers, each within one of its census count. A restart empties the in-memory miss
ledger, so every boot re-reads the whole cohort once.

**Cure (in code, not measured).** Commit `89c28f109`: `inventory_for_peer_reconcile` omits rows whose id is a
`mishpat_commitments.cid` (an exact join, not a test on id shape), and `total` is counted over the same set. The
project-scoped inventory the trigger courier uses, and the local diff, are unchanged. The cure is stateless: no table,
column, wire field or zome change. It had not been measured on the household when this was written; the check is the
same 300 s window after a restart, where `get_rea_commitment` should fall from the census counts to the few residual
ids below.

**Rejected design.** The "Stage 2 peer-probe adjudication" in `rea-stream-no-divergence-adjudication-drain-path`
would have had every advertiser answer Absent from its content_store, and would have durably retired ids that are in
fact notarized on another DNA.

**Residual, not addressed.** 3 to 10 ids per peer of other derived or local shapes
(`provide-<agent>-content:<reach>`, unanchored `provide-<hex>`, `operate-doorway-...`, `hosting-agreement-...`) still
ride the ladder. At the ladder's cap their cost is near zero; that is a statement about the ladder, not a measurement
of these ids.

**Open product question, separate from the call budget.** Should each peer's resilience views show other peers'
replication commitments? If so, those ids would have to route to `mishpat::get_commitment` and be mirrored on every
peer. Today they are not, and the cure above makes that explicit instead of leaving it to a read that cannot succeed.

**Mixed versions.** An old advertiser still advertises the rows; an upgraded receiver then behaves as before for
that advertiser until the advertiser is upgraded.

**Done when.** The post-restart 300 s window on the household shows `get_rea_commitment` at or near the residual
count on every peer, and `idle-is-free`'s ledger carries the reading.
