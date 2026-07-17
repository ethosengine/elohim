---
id: "backlog-eprfs-ipfs-analog-dataplane-sdk-surface"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "eprfs as the IPFS analog — an SDK surface that crystalizes building on the p2p dataplane"
slug: "eprfs-ipfs-analog-dataplane-sdk-surface"
written: "2026-07-17"
author: "did-bridge brainstorm (operator direction)"
status: "open"
priority: "medium"
area: "substrate/dataplane-sdk"
domain: "D5"
jobs: [elohim-storage]
cites:
  - genesis/docs/superpowers/specs/2026-07-17-did-bridge-identity-resolution-design.md
tags: [eprfs, ipfs-analog, dataplane, sdk, dx, content-addressing, did]
---

# eprfs as the IPFS analog — a dataplane SDK surface

Operator direction (2026-07-17, DID-bridge brainstorm): where Dyne's W3C-DID
says "share DIDs p2p using IPFS," our analog should be "share DIDs p2p using
eprfs" — i.e. the p2p dataplane (libp2p/iroh blob + CRDT planes) deserves an
**IPFS-like SDK surface** so builders (and bridges) have one crystallized way
to put content on, and get content from, the dataplane by CID.

Named "eprfs" deliberately: the eprfs package layer already exists as the
elohim-native content-addressed authority layer (plant-eprfs family); this item
is its **byte-plane completion** — the developer-facing surface (add / get /
provide / resolve by CID) over the existing blob store, custody, and sync
machinery, at the SDK grammar seam (compose inward, add a manifest — not a new
transport).

Deferred from the DID-bridge spec (scope discipline): DID documents are small
JSON projections and will ride this surface as ordinary content once it
exists. Not blocking the bridge crate.

Route through `p2p-design-gate` + `atlas-grounding` (seams: 3.5 SDK grammar ×
3.10 peer-hoster dataplane; D5) when picked up.
