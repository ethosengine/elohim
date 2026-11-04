---
id: "backlog-native-dev-track-e-tier-blob-store-slice1-local-chunked-put"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Track E: eprfs tier blob store slice 1 - local chunked CID-addressed put, then serve-blob mount, one-device I1-I7 failure tests"
slug: "native-dev-track-e-tier-blob-store-slice1-local-chunked-put"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, eprfs-peer, blob-store, custody, row-5, track-e]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Item (spec §10, §11 Track E)

Write-set: `elohim/eprfs/eprfs-peer`, `eprfs-storage`.

1. **Local tier put first** (chunks <= 1 MiB, each CID-named in a manifest; write, address, record, flow event). Publishing and replication are asynchronous widenings, state shown per copy.
2. Then a `serve-blob` mount beside the channel store, with a `serve-blob` `Use` writer. No second protocol.
3. I1-I7 failure tests (wipe, corrupt, kill a fetch, revoke a credential) on **one device with zero peers** before any two-workspace run.

**Ruling.** This is plan row 5's first slice. The plan's "slice 1 PUTs to a server" is struck. Cache bytes are a local `custody-blob` pledge (class B, born `self`), zero DHT heads (D6). The 454 MB storage binary is the acceptance case (RS 4+3 only at community reach).

**Receipt.** One-device failure tests green. Unblocks join F2.

**retire-when.** Tier blob put and serve-blob are the only byte path for derivation artifacts; sccache-style server PUT is not built.

## 2026-11-04 (adversarial review): serving scope, daemon-down put, tolerant reader (spec §10 items 12, 12a; §7 D7; §11 track E)

- **Order.** local tier put (with daemon-down path) -> A11 tolerant reader -> `serve-blob` mount with scope enforcement. The tolerant reader ships before any new byte path on the tier.
- **Serving enforces scope.** A carrier's grant covers exactly its task's projected view and declared outputs; every served object is checked against its own reach. Today the tier gate ignores root and object CIDs, so any admitted device can read every mounted root (`elohim/eprfs/eprfs-peer/src/gate.rs:14,64`). That is safe only because the channel allow-list limits what is published; a general blob mount removes that implicit boundary. A requester's `AskPolicy` cannot constrain a hostile requester.
- **Daemon-down local put.** The put works with the daemon down: a direct library write to the tier, or an explicit `queued, unrecorded` state; never a silent drop.
- **New receipts.** (a) put and read with no daemon running; (b) an admitted carrier asks for an unrelated CID and is refused; plus the existing one-device I1-I7 failure tests with zero peers.
