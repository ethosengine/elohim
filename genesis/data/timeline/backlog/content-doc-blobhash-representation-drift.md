---
id: "backlog-content-doc-blobhash-representation-drift"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pure iroh had NO projection-reconcile discovery arm — the content heal leg (anchor from own conductor, fed by peer-advertised inventory) rode P2PHandle only, so a recovering iroh peer's drifted row (sha256-… vs the author's bafkrei… CID, same bytes) never healed; homo-libp2p healed it in 58 s. CURED by the ReconcilePeers seam (homo-iroh warm 61 s / 60 s). Residual: the converged content DOC still carries sha256-… while the author's row is bafkrei…, so every doc round re-drifts an amber row until the anchor heal restores it"
slug: "content-doc-blobhash-representation-drift"
written: "2026-08-29"
author: "pure-iroh parity cut (post M0)"
status: "in-progress"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-iroh-only-content-projection-loop-gap"
  - "habit:dataplane-convergence"
tags: [dataplane, projection-reconcile, iroh, transport-parity, blob-hash, cid, recovery-harness, ratchet-lane-P]
---

## Measured (household mesh, 2026-08-28/29, warm recovery jessica←matthew)

- matthew's row `evolution-of-trust`: `blob_hash = bafkreihokma…` (= `blob_cid`, green, `dht_anchor_state live`).
  `bafkrei…` decodes to CIDv1/raw/sha256 with digest `ee5301c9…` — the SAME bytes jessica names as
  `sha256-ee5301c9…`.
- jessica's row was CREATED with `bafkrei…` by the pull leg at 23:25:20Z; by 23:53:52Z (the next boot's
  slug index) it read `sha256-ee5301c9…`. **Writer NAMED 2026-08-29 01:16:20Z** by the new
  `elohim_storage::sync_heal` INFO line: `reverse projection: converged blobHash differs from the local row`
  `from=bafkrei… to=sha256-ee53… green=false` — the doc reverse-projection (`reverse_project_content_doc`,
  after `iroh sync changes applied`) copies what the converged DOC holds. One second later (01:16:21Z) the
  anchor heal — now live on iroh — restored `bafkrei…` from the own conductor.
- homo-iroh warm (`cut=pull-core-10`): P0/P2/P3/P4 green by 469 s, **P1 red** for the whole poll — no heal
  ever touched the drifted row. (The timeline's 742 s "PASS" for that cut is annotated `invalid`: the
  storage arms were restarted on libp2p at 00:56:37Z mid-poll.)
- homo-libp2p warm (`cut=pull-core-10-libp2p-compare`): **PASS 58 s**. jessica's log names the healer:
  `projection-reconcile[content]: HEALED content anchor from own conductor (peer discovery)` at 00:58:06Z,
  `discovered_via_peer=12D3KooW…` — the heal leg re-derived the row from the DHT record (`bafkrei…`).

## Cause

`projection_reconcile::run_discovery` / `run_heal` (+ `participations_reconcile`, and the
adopt-before-author `PeerHeadRecordFetcher`) took `&P2PHandle` and called `list_peers` /
`view_federate(libp2p::PeerId, …)`. `main.rs` gated the whole reconcile loop on `p2p_handle` — so with no
libp2p node there was no discovery and no heal, and `caughtUp` could read true while a row stayed
divergent from the conductor's own truth. The iroh plane already served the view-federation ALPN
(`p2p_iroh::view_fed`, `ViewFedServiceBackend`) — only the requester side was missing.

## Cure (landed in this cut)

`p2p::reconcile_peers::ReconcilePeers` — `agent_pubkey` / `list_peers` / `view_federate(peer_id: &str, …)` —
implemented by `P2PHandle` (libp2p) and `p2p_iroh::IrohReconcilePeers` (peer book + `IrohViewFederationClient`,
same label rule as `pull_core`). Every reconcile arm and the head-record fetcher take `&dyn ReconcilePeers`;
`main.rs` spawns the loop on whichever source exists. Evidence to bank: homo-iroh warm P0–P4 PASS,
`recovery-detail:` line empty.

## Open

1. **Why does the converged doc hold `sha256-…` when the author's row is `bafkrei…`?** The staging PATCH
   rides `update_via_conductor`, which DOES emit `ContentUpdated` (content_service.rs:533) → the projector's
   event path asserts every field (`project_content_doc`). matthew's log shows no projector line at the
   23:33:10Z PATCH, but those lines are `debug!` — invisible at the mesh log level. Candidates: the sled
   DocStore outlives the per-mesh re-seed (yesterday's doc carries yesterday's `sha256-…` and the bulk
   re-seed's `project_content_doc_reconcile` is offer-only, never contesting it) and the PATCH-time assert
   then lost an Automerge concurrent-put resolution against that older actor; or the event-path projection
   lagged/dropped. Decisive probe: a metadata-only PATCH on the author (forces a full assert) and read
   what the recovering peer's `sync_heal` line then copies (`to=`). The reverse heal is now idempotent
   (no write when the row already names the converged value) so a healthy doc stops touching rows.
   **PROBED 2026-08-29 01:22:01Z**: metadata-only PATCH on matthew → jessica `iroh applied announced
   change` at 01:22:01.344Z and NO `sync_heal` line — her row (`bafkrei…`) already equalled the converged
   doc. So the event-path assert works and the doc now names `bafkrei…` fleet-wide; the stale `sha256-…`
   was a one-time state left by the prologue's conductor-bridged staging PATCH at 23:33:10Z. Next time the
   prologue runs, the `sync_heal` line on a non-author peer names whether that PATCH's projection lands.
2. Representation: author and replicas should agree TEXTUALLY on `blob_hash` (`blobHash` ≠ `blobCid` by
   the duality rule — never alias them). Once (1) is named, decide canonical-at-write vs digest-compare in
   `update_content`'s amber path.
