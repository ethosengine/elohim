---
title: "Cite Fingerprint ↔ Canonical CID Convergence"
id: cite-fingerprint-cid-convergence
tier: spec
status: Draft
created: 2026-07-12
maintainers: Matthew Dowell + Claude Fable 5
class: architecture
topic: [cite-graph, content-addressing, cid, eprfs, brit, fingerprint, convergence]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR machine-facing-surfaces-seal-full-cids
cites:
  - elohim/eprfs/eprfs-core/src/address.rs
  - elohim/eprfs/eprfs-cli/src/main.rs
  - elohim/brit/brit-epr/src/engine/cid.rs
  - elohim/brit/brit-epr/src/engine/frontmatter.rs
  - .claude/scripts/_lib/cite_graph.py
---

# Cite Fingerprint ↔ Canonical CID Convergence

## The decision — one digest, two renderings

The repo had two content-addressing systems that looked unrelated:

- **Cite fingerprints** — `sha256:` + `hex(sha256(canonical_body))[:16]`, the drift identity a
  doc's `cites:` envelope carries (`_lib/cite_graph.py`, brit's `drift_fingerprint`).
- **Canonical CIDs** — CIDv1 / sha2-256 / base32, the protocol's real content address (eprfs-core
  `BlobCid`, brit `BritCid`, `elohim-epr`).

They were never two systems. **Both hash sha2-256.** The cite fingerprint is, mathematically, the
first 16 hex chars of the multihash digest that a raw-codec CID over the same canonical-body bytes
already carries. Convergence is therefore a **declaration + test-pin**, not a rewrite of the sealed
`sha256:hex16` envelopes.

**Canonical identity of a doc's body** = `body_cid` = `CIDv1(raw 0x55, sha2-256(canonical_body_bytes))`
(`bafkrei…`). The cite fingerprint is a defined **short-form projection** of that CID:

```
fingerprint = "sha256:" + hex(multihash_digest)[:16]
```

One sha2-256 digest, two renderings — the full CID and the short form.

## The codec rule (the ambiguity this closes)

The digest is codec-agnostic, but the CID that wraps it must tag its bytes honestly:

| Bytes | Codec | CID prefix | Constructor |
|-------|-------|-----------|-------------|
| File / body / arbitrary blob bytes | `0x55` raw | `bafkrei…` | `compute_raw` |
| Already-canonical dag-cbor atoms | `0x71` dag-cbor | `bafyrei…` | `compute` |

A document body is arbitrary bytes, **not** a canonical CBOR atom, so its address is a **raw**
(`0x55`) CID. `BlobCid::compute` previously labeled all bytes as dag-cbor — that ambiguity is now
documented and closed by `compute_raw` (eprfs-core mirrors brit, same golden vectors).

## Short-form definition (the single derivation)

`short_fingerprint(cid) = "sha256:" + hex(cid.multihash.digest)[:16]`. Authoritative in Rust:
`eprfs_core::BlobCid::short_fingerprint` and `brit_epr::engine::cid::BritCid::short_fingerprint`.
Python's `cite_graph.fingerprint_text` is the byte-identical fast path (hashlib), pinned to the
Rust CID short-form by a cross-language corpus parity test.

## Full-CID tokens in the fingerprint slot

The `cites:` fingerprint slot now accepts **either** rendering:

- `sha256:hex16` — the short form (human-facing default; unchanged).
- `bafk…` / `bafy…` — a full CIDv1 token (machine-facing). Already prefix-**tolerated** by the
  parser (`_is_fingerprint` broadened from `bafy` to `baf` so raw-codec body CIDs are recognized);
  now verdict-**supported** — `envelope_verdict` DECODES the CID's sha2-256 digest and compares it
  to the recomputed body digest.

## Python never encodes CIDs (single-source invariant)

Python may **decode** a CID (base32 → multihash digest) to compute a verdict, but it must **never
encode** one. CID/base32 construction lives in exactly one place: the `eprfs cid` CLI
(`elohim/eprfs/eprfs-cli`, verb `cid <path> [--body] [--short]`). Any language needing a full CID
shells out to it. This keeps the encoder single-sourced and prevents a second, drift-prone CID
implementation from re-appearing in a scripting layer.

## Upgrade path (no mass rewrite)

- **Human-facing envelopes stay short-form.** The ~435 sealed `sha256:hex16` envelopes are already
  correct — they are the short-form projection of their body CIDs. Nothing to migrate.
- **New machine-facing surfaces MAY seal full CIDs** — eprfs packages, agentdoc projections, and
  other content-addressed artifacts where the full CID is the natural identity. They round-trip
  through the same cite pipeline and verdict cleanly.
- The two renderings are interchangeable evidence of the same digest, so a surface picks whichever
  fits its audience without breaking cross-references.

## Pins (do not regress)

- `eprfs-core`: `raw_codec_matches_brit_vector`, `short_fingerprint_equals_python_cite_fingerprint`.
- `eprfs-cli`: `short_form_of_body_matches_python_oracle` + the corpus parity test
  (`eprfs cid --body --short` == `cite_graph.fingerprint()`).
- `brit-epr`: `cid_fingerprint_derivation` (`drift_fingerprint` == body CID short-form).
- Python: full-CID ok/stale verdicts in `cite_graph_test` + `cite_gen_test`.
- Existing parity contracts (`cite_parity.rs`, `epr_meta_cascade_test`) stay green — this ADDS a
  derivation pin, it does not change the fingerprint format.
