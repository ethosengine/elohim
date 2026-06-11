---
id: "backlog-ci-substrate-commitment-projection-convergence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "REA commitment rows reach only the POST pod — peer discovery returns 0 ids fleet-wide, so projections never converge (DHT-leg break, Workstream A of the EPR durability arc)"
slug: "ci-substrate-commitment-projection-convergence"
written: "2026-06-11"
author: "agentic-developer (EPR durability arc, Phase 0)"
status: "wip"
priority: "high"
ci_status: pending-verification
jobs: [elohim-genesis]
tags: [substrate, epr-durability-arc, rea-commitments, projection-reconcile, custody-sweep, dht, observability]
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/p2p/view_federation.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
  - elohim/elohim-storage/src/reconcile/custody_sweep.rs
  - genesis/scripts/ci/substrate-verify.sh
  - genesis/seeder/src/seed-household-formation.ts
  - genesis/docs/superpowers/plans/2026-06-10-epr-durability-replication-arc-plan.md
---

# Commitment-projection convergence break — rows reach only the POST pod

## Symptom (live alpha, 24h Loki read 2026-06-10 → 06-11, Phase 0 of the EPR durability arc)

- matthew's projection holds **16 custody-blob rows, ALL with `dhtAnchorHash` set**
  (read via `doorway-alpha.elohim.host/api/v1/commitments?action=custody-blob` —
  anchoring to the DHT WORKS; provider ids are Stage-1 fakes, separately fixed).
- jessica's `projection-reconcile: sweep complete` lines for 24h:
  `peers_asked=2-3, ids_discovered=0, healed=0, local_total=0, caught_up=true` —
  she has **zero rows and discovers zero ids from peers**, every 300s, all day.
- Blob-level content inventory exchange is healthy in the same window (jessica
  served `count=3555` and received `count=3554`), so the P2P channel itself works.
- Genesis #1118's M1 cross-pod fetch: jessica GET matthew's blob → **404**.

## What is and is not the break

`ReaCommitmentCommitted` is emitted from the zome's `post_commit`, which Holochain
runs **only on the authoring peer** — non-authors never hear the signal by design.
The subscription in storage (`main.rs:737-758`) exists and registers on every boot;
the "missing 2a subscription" framing is wrong. The DESIGNED convergence path for
non-authors is `projection_reconcile` (300s sweep): ask connected peers for their
`(id, anchor)` inventory over `ViewKind::ProjectionInventory`, then heal each
missing id from the OWN conductor. That path is alive (`peers_asked=2-3`) but
**discovery returns 0 ids fleet-wide** while a peer demonstrably holds 16.

Remaining root-cause candidates (not yet discriminable from the evidence):
1. The 2-3 responding peers don't include the row-holder (jessica↔matthew
   adjacency gap — consistent with the #1118 cross-pod 404, since heal-on-read
   races CONNECTED peers and still missed).
2. The holder responds but the payload doesn't decode (version skew) — was
   logged at DEBUG, invisible in Loki.
3. The holder's responder serves empty for another reason (h_app_id scope was
   ruled out: POST and inventory both resolve to `"lamad"`).

## Landed this session (status: wip)

- **Observability that makes the next run decisive**: per-peer
  `projection-reconcile: peer inventory received` INFO on the asker; undecodable
  payload upgraded DEBUG→WARN; `ProjectionInventory: serving local inventory`
  INFO on the responder; `T23: custody reconcile pass completed` upgraded
  DEBUG→INFO (Phase 0 scanned 953k jessica lines and could not tell a dead sweep
  from a quiet one).
- **Inventory-blind fallback in the custody sweep** (`reconcile/custody.rs`):
  when `peer_blob_inventory` has no candidates for a missing provider-owned blob,
  race the connected peers (cap 8) — gossip publish dies on every pod restart
  (`InsufficientPeers`), so sweep convergence no longer depends on gossip health.
  New `fallback_kicks` counter in the pass outcome + INFO line.
- **CI assertion** `propagation.custody-convergence` in `substrate-verify.sh`:
  the custody-blob commitment for the content blob must be visible on EVERY
  reachable pod within the propagation window, not just the POST pod.
- **Ceremony peer ids**: `buildCeremonyCustodyInput` now resolves Stage-2 REAL
  peer ids (was the last custody-blob writer on Stage-1 fakes).

## Verification path

Next genesis run with these commits (plus the operator netpol apply for the
conductor-seed legs): correlate per-peer "serving local inventory" vs "peer
inventory received" Loki lines to pin which of the three candidates is real, and
watch `propagation.custody-convergence` + `kicksFiredTotal` rising on receivers.
Done = three consecutive genesis builds with convergence passing (arc plan's
stability gate).

shift_objective: |
  Make REA commitment projections converge on every alpha pod without CI nudging:
  pin the discovery break via the new per-peer inventory INFO/WARN lines from the
  next instrumented genesis run, fix the pinned leg (adjacency, decode skew, or
  responder), and keep propagation.custody-convergence green for three
  consecutive builds.
