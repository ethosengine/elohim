# Re-genesis 1 — the shelf picks (2026-11-05, operator: "pick these up NOW")

Principle these all instantiate: **every primitive that can age — hash, chunker, erasure code,
signature scheme — is declared by code + parameters inside the object that uses it.** Agility by
declaration, at epoch 0, while there is nothing to migrate. CIDv1's multihash is the model; the
other three get the same treatment.

## P1 — Content-defined chunking (in re-genesis 1, beside the BLAKE3 profile)

- **Decision:** artifact and content bytes are chunked by **FastCDC**, not fixed offsets. Chunker
  id and parameters (gear table hash, min / target / max) are part of the byte profile for the
  epoch; two peers chunking the same bytes produce identical chunk CIDs, so manifests of
  deterministic builds converge byte-for-byte.
- **Parameters for epoch 0 (to measure, not promise):** min 256 KiB, target 1 MiB, max 4 MiB —
  target keeps today's 1 MiB bound; max stays far under the 64 MiB frame; min keeps chunk count
  sane for the 454 MB case.
- **Composition:** CDC decides where a chunk ends; BLAKE3 names it (raw CIDv1 `bafkr4i…`); Bao
  verifies inside it over iroh-blobs; the manifest is the ordered list of (chunk CID, length) under
  the artifact's whole-bytes BLAKE3 CID (`record.output`).
- **What it buys:** successive builds of the same binary share most chunks, so the one payload store
  dedups across versions, custody bytes stop growing linearly with release count, and the sibling
  fetch moves only the chunks that changed. Fixed chunks give none of this.
- **Acceptance:** (a) manifest determinism — two devices chunk the same bytes to the same CIDs;
  (b) measured sharing — two successive `just mesh build` storage binaries report their shared-chunk
  fraction and bytes moved on F2; (c) a chunker-id the reader does not implement is a typed
  `unsupported-profile`, never a silent mismatch.
- **Lands:** R1 (profile declaration) and Track E slice 1's manifest (`TierBlobManifest` gains the
  chunker declaration; `TIER_BLOB_MANIFEST_EPOCH` moves).

## P2 — Signature-scheme agility (now, because Track C is in flight)

- **Decision:** every signed envelope (D7 body/sig framing) carries **scheme code + public key as a
  multikey + signature**, never a bare 64-byte ed25519 blob. ed25519 is epoch 0's canonical scheme;
  a hybrid ed25519 + ML-DSA profile is admissible later by declaration.
- **Why now:** hashes are post-quantum safe; signatures are not. For generational architecture
  this is the real "BLAKE4" risk, and it lives in brit's `AgentKey`, the `DerivationRecord`
  witness and every attestation. Track C is signing the record *this sprint*; adding the scheme
  field now costs a line, adding it after records are signed costs a migration.
- **Acceptance:** a record under an unknown scheme is **held** (not "tampered", not accepted); a
  known scheme verifies; brit's signer publishes its key as a multikey; the eprfs verified-read
  tests gain the unknown-scheme case.
- **Lands:** Track C (brit signs `DerivationRecord`), D7's framing rule, brit `AgentKey` encoding.
  Integrity-class: Claude-ruled; Claude-family adversarial review before landing.

## P3 — Erasure code as a declared profile (declare now, implement later)

- **Decision:** RS 4+3 stays epoch 0's code, but the shard manifest names **code id + parameters**
  so a rateless code (RaptorQ, RFC 6330) can be admitted for large swarms without a manifest break.
- **Why:** fountain codes scale better across many holders and lossy paths; RS is simpler and
  already specified. Declaring the field now is free; retrofitting it is a shard-manifest epoch.
- **Lands:** the shard manifest shape in the batched DNA move (`ShardManifest` is moving to CIDs
  anyway); no RaptorQ implementation in this sprint.

## P4 — Range-based set reconciliation for inventory (fold into H1)

- **Decision:** the authenticated inventory envelope from H1 is reconciled by **ranges
  (negentropy)**, not by capped snapshots, so Radicle's "inventory truncated at 2,973" and "may never
  announce a new repository" cannot recur by construction.
- **Lands:** dataplane-borrows row 21 (already minted) sequenced directly after H1; same envelope,
  same epoch.

## What this changes in the record

- R1.1 gains the chunker declaration beside the multihash in the byte profile.
- D7 gains the signature-scheme declaration.
- Re-genesis 1's crossing = typed CIDv1 fields everywhere + BLAKE3 byte profile + CDC chunking +
  multikey signatures + erasure-code field, in one batched DNA move; then the artifact store on top.
- Re-run the Astra challenge on this combined scope before recording.
