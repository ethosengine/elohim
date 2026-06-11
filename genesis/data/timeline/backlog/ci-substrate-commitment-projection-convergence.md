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

## UPDATE 2026-06-11 00:30 (post-grant Loki re-read): candidates narrowed

Fleet sweep table (live, every pod on the same 300s cadence):

| pod | local_total | peers_asked | ids_discovered |
|---|---|---|---|
| matthew | **28** | 10 | 0 |
| adam | 0 | 11 | 0 |
| jessica | 0 | 2-3 | 0 |

And `d88bba0a1` (the commit carrying the CURRENT ProjectionInventory
responder — latest touch of view_federation.rs on origin/dev) **is deployed
fleet-wide** (edge #1056 built after it). So candidate 2 (version skew) is
DEAD, and candidate 3 stays dead. If any pod successfully asked matthew, it
would discover 28 ids; every pod discovers 0. The break is therefore:
**matthew is absent from every other pod's successful-responder set** —
either he's not in their connected-peer lists (adjacency/visibility), or
requests to him fail (timeout/Err), which silently drops him from
`peers_asked` (the counter only increments on a received response).
matthew asking 10 peers while jessica reaches 2-3 shows the topology is
already asymmetric. The landed per-peer INFO lines will name each pod's
responders on the first post-deploy sweep; `mesh.adjacency-reverse` asserts
the jessica→matthew edge in CI.

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

## UPDATE 2026-06-11, genesis #1119: candidate 1 CONFIRMED — adjacency partition

The new mesh asserts ran live and pinned it:

```
❌ mesh.adjacency — matthew does NOT list jessica (12D3KooWBS8a…) — cross-pod fetch will starve
❌ mesh.adjacency-reverse — jessica does NOT list matthew (12D3KooWQAaK…) — reconcile discovery on jessica cannot reach the row/blob holder
✅ mesh.{matthew,adam,jessica}.connected — connectedPeers=10/11/10
✅ mesh.version-parity — all pods on 195bc4f
```

Both directions of the matthew↔jessica edge are absent while every pod
holds 10-11 live connections — **the row-holder is partitioned from the
askers at the libp2p adjacency level**, not at decode (candidate 2 dead,
version parity proven) nor responder scope (candidate 3 dead). And
`propagation.custody-convergence` fired exactly as designed: commitment
present on matthew, `missing on: adam jessica` after 315s/21 attempts.

Open question the artifact could not answer: WHO are the 10-11 peers each
pod IS connected to? `substrate-verify-mesh.json` archived only counts —
fixed this session: the mesh stage now archives `connectedPeerIds` per pod,
so the next run yields the full adjacency matrix (suspects: doorways,
steward devices, browser/relay clients crowding a connection budget, or a
dial-policy gap where alpha pods never dial each other absent gossip
inventory need).

## Verification path

Next genesis run after the edge deploy (instrumented storage + custody
fallback kicker): (1) read `connectedPeerIds` in substrate-verify-mesh.json
for the true topology; (2) correlate per-peer "serving local inventory" vs
"peer inventory received" Loki lines to see whether ANY pod successfully
asks matthew; (3) watch `propagation.custody-convergence` + fallback kicks.
NOTE the fallback kicker races CONNECTED peers — while matthew↔jessica
adjacency is absent, the kicker cannot reach the holder either; the
adjacency fix is the convergence fix. Done = three consecutive genesis
builds with convergence passing (arc plan's stability gate).

shift_objective: |
  Make REA commitment projections converge on every alpha pod without CI nudging:
  pin the discovery break via the new per-peer inventory INFO/WARN lines from the
  next instrumented genesis run, fix the pinned leg (adjacency, decode skew, or
  responder), and keep propagation.custody-convergence green for three
  consecutive builds.

## UPDATE 2026-06-11 (live Loki, post edge-#1059 deploy): MESH + DISCOVERY HEALED — new front is the conductor bridge

The persistent-peering fix is proven on all 14 pods: `Bootstrap link
established` on first dial, redial arm resolved the startup stagger
(attempts 1-7, zero failures), `Bootstrap links recovered, connected:13`
on EVERY pod by 15:02:54Z. matthew↔jessica adjacency: HEALED. And
discovery works end-to-end for the first time: every pod's sweep reports
`ids_discovered:34` (matthew's full commitment inventory, responders now
NAMED in per-peer lines).

The new (and likely final) break in the chain: `conductor_missing:34,
healed:0` on every non-matthew pod — the heal leg asks the LOCAL conductor
for the discovered entries and gets nothing. Prime suspect is not DHT
gossip but the bridge: at boot, storage retries the conductor connection
5x while cells are still CellDisabled, then PERMANENTLY disables the
reconcile bridge ("Reconcile controller disabled" fleet-wide at 15:00Z) —
the same one-shot-init disease the peering fix cured in libp2p. Fix in
flight: persistent HcClient reconnect (capped backoff, late-success ==
boot-success wiring). Secondary candidate if conductor_missing persists
with a healthy bridge: true DHT gossip lag (kitsune2 "Bootstrap
overloaded, dropping put" was observed under load).
