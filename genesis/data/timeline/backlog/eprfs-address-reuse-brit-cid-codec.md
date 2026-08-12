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
cites:
  - genesis/research/serialization-canonicality-cross-pollination-2026-08-11.md
---

## STATUS UPDATE 2026-08-11 — original premise LANDED; the residue is a codec-tag mismatch

The "stringly-typed placeholder" below **has since been fixed** ✅. `eprfs-core/src/address.rs` now
wraps a real `cid::Cid`, computes `CIDv1(sha2-256)` via `cid` + `multihash-codetable`, exposes both
`compute` (`0x71` dag-cbor) and `compute_raw` (`0x55` raw), forbids construction from an arbitrary
string, and pins the format with golden-vector tests against brit (`raw_codec_matches_brit_vector`)
and elohim-epr (`cid_matches_canonical_format`). The §What/§Proposed-work below is historical.

**What remains is narrower and concrete: three call sites tag non-CBOR bytes as dag-cbor**, in
direct violation of the rule `address.rs` itself documents — *"Arbitrary file/body/blob bytes are
NOT dag-cbor and must use `BlobCid::compute_raw` (codec 0x55) so the codec tag tells the truth
about the bytes."*

| Site | Input | Uses | Should use |
|---|---|---|---|
| `eprfs-agent/src/canonical.rs:139` | hand-rolled line-oriented **text** envelope (not CBOR) | `compute` (`0x71`) | `compute_raw` (`0x55`) |
| `eprfs-storage/src/lib.rs:83` (`put_blob`) | arbitrary rendered file bytes (markdown) | `compute` (`0x71`) | `compute_raw` (`0x55`) |
| `eprfs-local/src/verify.rs:36` | `fs::read()` file bytes | `compute` (`0x71`) | `compute_raw` (`0x55`) |

**Not a live break inside eprfs** — `put_blob` and `verify` both use `compute`, so drift detection is
internally consistent. Two real consequences:

1. **Operator-facing divergence.** `eprfs-cli/src/main.rs:90` addresses a file with `compute_raw`
   (`bafkrei…`) while the projection manifest addresses the *same bytes* with `compute` (`bafyrei…`).
   `eprfs cid <file>` can never reproduce a manifest's blob CID.
2. **Latent interop break.** `rust-ipfs/src/block.rs:58` and `elohim-storage/src/dag_store.rs:103`
   both dispatch on `cid.codec()`; a `0x71`-tagged markdown blob routes to `DagCborCodec::decode`
   and fails. Any eprfs blob CID crossing into an IPLD-aware consumer breaks on dereference.

Cite-fingerprint parity survives by luck: `short_fingerprint()` reads only the multihash digest,
which is codec-independent, so the `sha256:hex16` short form matches across both codecs.

**Work:** flip the three sites to `compute_raw`; add a test asserting `eprfs-cli` and `put_blob`
agree on the CID for identical bytes. **Migration note:** this changes existing blob CIDs, so any
persisted manifest needs a re-projection pass — size that before picking it up.

Surfaced by [serialization-canonicality](epr:serialization-canonicality-cross-pollination-2026-08-11) §2a.

---

## What (historical — premise landed 2026-08-11, see above)

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
