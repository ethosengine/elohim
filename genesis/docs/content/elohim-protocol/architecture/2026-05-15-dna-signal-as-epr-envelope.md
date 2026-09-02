---
title: DnaSignal as EPR Envelope — T18 Specification
id: dna-signal-as-epr-envelope
tier: architecture
status: accepted — Implemented (legacy signal retired, T30)
created: 2026-05-15
pillar coupling: elohim (EPR envelope primitive), imagodei (key-revocation producer site)
informed-by:
  - elohim/sdk/schemas/v1/dna-signals/key-revocation.schema.json (EPR envelope schema)
  - RecoveryV2Signal::KeyRevocationEffective (legacy signal this envelope superseded; variant retired T30)
informs:
  - All future DnaSignal variants that need EPR-shape envelope semantics
  - Any new dna-signal schema declared under elohim/sdk/schemas/v1/dna-signals/
  - Future imagodei coordinator emit-site work that adopts the EPR-envelope pattern
---

# DnaSignal as EPR Envelope — T18 Specification

**Date:** 2026-05-15  
**Task:** T18 — DnaSignal::KeyRevocation EPR envelope  
**Status:** Implemented; legacy signal retired (T30, same-day follow-up)

## Summary

T18 introduces a new `DnaSignal::KeyRevocation(KeyRevocationEnvelope)`, originally
emitted alongside the legacy `RecoveryV2Signal::KeyRevocationEffective` at every
producer site in the imagodei coordinator zome during a back-compat window
(closed at T30 — see Legacy Deprecation below). The new signal frames the
wire message as an EPR (Elohim Protocol Record): the authoring elohim's
attestation over a content-addressed (CID) subject, signed at emit time with
the calling agent's lair-managed ed25519 key.

The legacy `RecoveryV2Signal::KeyRevocationEffective` was marked
`#[deprecated]` for a one-release back-compat window, then the variant itself
was deleted from the coordinator zome's `RecoveryV2Signal` enum in the
same-day T30 follow-up — producers now emit only `KeyRevocationRequested`
(unchanged) plus the `DnaSignal::KeyRevocation` envelope at all three producer
sites. See Legacy Deprecation below for current status.

## EPR Envelope Shape

Schema: `elohim/sdk/schemas/v1/dna-signals/key-revocation.schema.json`

```json
{
  "type": "keyRevocation",
  "attestationKind": "attestation:key-revocation-emit",
  "subjectCid":  "<CIDv1 dag-cbor sha256 of governance-action:key-revocation Content>",
  "issuer":      "<base64(STANDARD) ed25519 pubkey, 32 bytes>",
  "issuedAt":    "<RFC3339 UTC>",
  "signature":   "<base64(STANDARD) ed25519 sig over canonical bytes, 64 bytes>",
  "metadata": {
    "revocationId":          "<rev-{humanId}-{ts}>",
    "revokedPubkey":         "<base64(STANDARD) ed25519 pubkey, 32 bytes>",
    "agentCid":              "<Stage 1: human_id; Stage 2: CID of imagodei Human entry>",
    "compromiseAt":          "<RFC3339 UTC — M4: equals effectiveAt>",
    "effectiveAt":           "<RFC3339 UTC>",
    "triggeringRevocationId": "<rev-id of vote that drove this, or null>",
    "supersedesCid":         "<CID of prior pending entry, or null on initial CREATE>"
  },
  "relayChain": []
}
```

The `type` discriminator comes from `#[serde(tag = "type", rename_all = "camelCase")]`
on the outer `DnaSignal` enum; the variant name `KeyRevocation` serializes as
`"keyRevocation"`.

## Canonical Signing Bytes

The issuer signature is **ed25519** over the MessagePack encoding of the
following sub-struct (excludes `signature` and `relayChain`):

```
CanonicalEnvelopeCore {
  attestationKind:  string,
  subjectCid:       string,
  issuer:           string,
  issuedAt:         string,
  metadata: CanonicalMetadataRef {
    revocationId:           string,
    revokedPubkey:          string,
    agentCid:               string,
    compromiseAt:           string,
    effectiveAt:            string,
    triggeringRevocationId: string | null,
    supersedesCid:          string | null,
  }
}
```

### Serialization details

- Encoder: `rmp_serde::encode::Serializer::new(buf).with_struct_map()` — struct-map
  form (field names included as strings, NOT array form).
- Field rename: `#[serde(rename_all = "camelCase")]` — all field names are camelCase
  on the wire (e.g. `attestationKind`, `subjectCid`, `revocationId`).
- Field order: declaration order of the `#[derive(Serialize)]` struct — deterministic
  under rmp_serde's struct-map serializer.
- Both `triggeringRevocationId` and `supersedesCid` serialize as MessagePack `nil`
  when `None`.

### Zome-side implementation

`imagodei/zomes/imagodei/src/lib.rs` — function `canonical_envelope_bytes`.

### Storage-side mirror

`elohim/elohim-storage/src/signals.rs` — function `canonical_envelope_bytes`
(local, not exported). **Must be kept byte-identical to the zome-side
implementation.** The sync-by-comment pattern documents the obligation:
```
// **Must match the mirror function in `elohim-storage/src/services/recovery_flow_projector.rs`.**
```

### Verification recipe (consumer)

```
1. Deserialize envelope from wire JSON.
2. canonical_bytes = canonical_envelope_bytes(&envelope)
3. issuer_bytes   = base64::STANDARD.decode(&envelope.issuer)       // 32 bytes
4. sig_bytes      = base64::STANDARD.decode(&envelope.signature)    // 64 bytes
5. key   = ed25519_dalek::VerifyingKey::from_bytes(&issuer_bytes)
6. sig   = ed25519_dalek::Signature::from_bytes(&sig_bytes)
7. key.verify_strict(&canonical_bytes, &sig)  →  Ok(()) iff valid
```

## Dedup Key

`metadata.revocationId` — format `rev-{humanId}-{ts}`. The storage projector
uses this as the `key_revocations` table PK. Delivering both the legacy
`KeyRevocationEffective` signal AND the new `DnaSignal::KeyRevocation` envelope
results in the same idempotent `set_effective` call keyed on this field.

## Producer Sites (three, all in imagodei coordinator zome)

| Function | Path | Trigger type | supersedes_cid | triggering_revocation_id |
|---|---|---|---|---|
| `create_self_revocation` | `lib.rs` | voluntary | None | None |
| `submit_revocation_vote` (threshold reached branch) | `lib.rs` | steward_vote | Some(prior pending CID) | Some(vote_id) |
| `submit_specialist_revocation` | `submit_specialist_revocation.rs` | specialist_attestation | None | None |

Each producer:
1. Builds the envelope with empty `signature`.
2. Calls `canonical_envelope_bytes` to produce the signing input.
3. Signs with `hdk::ed25519::sign_raw(agent_pk, canonical_bytes)`.
4. Fills `signature` with `base64::STANDARD.encode(&raw_sig.0)`.
5. Emits `DnaSignal::KeyRevocation(envelope)` via `emit_signal`.

## Consumer Side (elohim-storage)

### Types (`elohim/elohim-storage/src/signals.rs`)

- `KeyRevocationEnvelope` — storage mirror of the zome struct (camelCase serde).
- `KeyRevocationMetadata` — nested metadata mirror.
- `ImagodeiDnaSignal` — discriminator enum, `KeyRevocation(KeyRevocationEnvelope)` variant.
- `canonical_envelope_bytes(&KeyRevocationEnvelope) -> Vec<u8>` — mirror of zome helper.
- `verify_envelope_signature(&KeyRevocationEnvelope) -> Result<(), StorageError>` — ed25519 verify.
- `handle_imagodei_dna_signal(conn, ImagodeiDnaSignal) -> Result<(), StorageError>` — dispatcher.

### Dispatch logic

`handle_imagodei_dna_signal` verifies the signature before projecting. On
failure: logs a warning at `imagodei.dna_signal` and returns `Ok(())` (best-
effort — the legacy signal still lands the same projection row). On success:
calls `crate::db::key_revocations::set_effective` with `metadata.revocation_id`
as the PK.

### Wire format note

The `type` tag value (`"keyRevocation"`) is NOT a field in `KeyRevocationEnvelope`
itself — it lives one level up in the `ImagodeiDnaSignal::KeyRevocation` wrapper.
`ImagodeiDnaSignal` uses `#[serde(tag = "type", rename_all = "camelCase")]` to
decode it.

## relayChain Field

Present-but-empty `[]` on all T18 wire. Future task: relay-elohims append their
own `RelayAttestation` objects here as the signal propagates across hops. The
field type is `Vec<serde_json::Value>` (opaque) on both zome and storage sides;
a typed `RelayAttestation` struct lands in a future task.

## EPR Worker Handoff (W2B)

The EPR W2B controller sweep uses `metadata.compromiseAt` for retroactive
attestation invalidation. In M4, `compromiseAt == effectiveAt` (no separate
discovery timestamp). A post-M4 revision of Recovery may populate this from
revocation request metadata.

**Key integration points for W2B:**
- Schema path: `elohim/sdk/schemas/v1/dna-signals/key-revocation.schema.json`
- Envelope shape: see above, serde `camelCase` throughout
- Canonical bytes recipe: struct-map MessagePack of the 5-field core struct
- Dedup key: `metadata.revocationId`
- Signature verification: ed25519, 32-byte pubkey + 64-byte signature, both base64(STANDARD)
- `metadata.agentCid`: Stage 1 = `human_id` string; Stage 2 = CID of imagodei Human entry
- `relayChain`: opaque `[object]`; empty in T18

The `RevocationAttestation` EPR-worker arm (Task 7, `attestation:key-revocation-emit`
flat signal) is a separate signal class and is NOT migrated here. The flat
`revocation-attestation.schema.json` stays untouched.

## Legacy Deprecation — CLOSED (T30)

`RecoveryV2Signal::KeyRevocationEffective` was marked `#[deprecated]` at T18
for a one-release back-compat window, then the variant was **deleted
entirely** from the coordinator zome's `RecoveryV2Signal` enum in the T30
follow-up (`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`, see the
comment at the `RecoveryV2Signal` enum definition). All three producer sites
(`create_self_revocation`, `submit_revocation_vote`,
`submit_specialist_revocation`) now emit only `KeyRevocationRequested` plus
`emit_key_revocation_envelope` (the T18 `DnaSignal::KeyRevocation` path) — the
legacy flat emit is gone, not merely suppressed. There is no
`#[deprecated]` attribute left anywhere in the imagodei coordinator zome for
this signal; the back-compat window is closed.

`elohim-storage/src/signals.rs` still declares a `RecoveryV2Signal::KeyRevocationEffective`
match arm as an explicit no-op (kept for wire-decode exhaustiveness against
older DHT history / possible replay); its own doc comment predates T30 and
should be read as historical, not as evidence the variant is still emitted.
Removing that dead arm is a separate, low-risk storage-side cleanup, not
tracked here.
