---
id: "backlog-eprfs-address-reuse-brit-cid-codec"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "eprfs address.rs reinvents CID as String newtypes — reuse brit BritCid / storage cid::Cid (real content-addressing, not a stringly-typed placeholder)"
slug: "eprfs-address-reuse-brit-cid-codec"
written: "2026-07-05"
author: "collaboration-through-the-protocol plan (Task 5 in-flight capture)"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## What

`elohim/eprfs/eprfs-core/src/address.rs` defines `EprRef`, `BlobCid`, and `ProjectionId` as
`#[serde(transparent)]` String newtypes with ZERO CID computation. `BlobCid` carries only the
aspirational comment "The canonical form should be CID-first" — i.e. it is a stringly-typed
placeholder, not real content addressing.

The proven codec already exists in two places, both documented byte-identical to the protocol's
`elohim-epr` codec:

- **brit** `brit-epr/src/engine/cid.rs` — `BritCid` wraps `cid::Cid`, CIDv1 over sha2-256 with
  multicodec `0x71` dag-cbor / `0x55` raw, base32 rendered (`bafyrei…`/`bafkrei…`).
- **elohim-storage** — `cid::Cid::from_str` parsing + the idempotent content-addressed PUT
  (`put_epr` rejects `envelope.cid != path_cid`).

## Why it matters

eprfs is a projection layer: a projected file is a render, and its **identity must be the CID/EprRef,
never the path** (the "projection as truth" bug the seam-map warns against). A String newtype gives no
tamper detection and no cross-version dangle-safety — the exact properties content-addressing exists to
provide. Shipping the placeholder risks eprfs projections drifting from the real EPR substrate identity.

## Proposed work

Wire `eprfs-core::address::BlobCid` (and where applicable `EprRef`) to the brit `BritCid` /
storage `cid::Cid` codec so projection identity is real CIDv1 content-addressing. Keep eprfs-core's
"storage-agnostic, domain-agnostic" boundary (depend on the `cid` crate + a small codec shim, not on
elohim-storage directly). Add a round-trip test (bytes → CID → parse → CID equal) mirroring the storage
idempotent-PUT contract.

## Provenance

Surfaced by the `collaboration-through-the-protocol` plan's understanding pass
(`genesis/docs/superpowers/plans/2026-07-05-collaboration-through-the-protocol-plan.md`).
Sequence after `feat/eprfs` lands on dev (operator-owned brit reconciliation + FF merge).
