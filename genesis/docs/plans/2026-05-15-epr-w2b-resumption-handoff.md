# EPR W2B Resumption Handoff

**Date:** 2026-05-15
**From:** Recovery M4 sprint (M4 owner)
**To:** EPR phase worker (W2B branch — paused mid-T5/T6)
**Status:** EPR W2B is now unblocked. The M4-side gates you listed in your pause-note are met.

## What landed on M4 since you paused

All architectural decisions you flagged at pause time have been resolved:

| Gate | Status | Commit(s) |
|---|---|---|
| T6 — `recovery_flows` + `key_revocations` Diesel migration | landed | `d89abd019` |
| T7 — Diesel models + CRUD | landed | `1d34a9153`, `c2b70133c` |
| T8 — `RecoveryFlowProjector` skeleton | landed | `1d34a9153` |
| T9 — state-machine branches | landed | `c2b70133c` |
| T10 — `elohim_content_dispatcher` prefix routing | landed | `596c7b8c9` |
| T11 — `hc_client::subscribe_elohim_content_signals` | landed | `4bf242ddb` |
| T12 — `main.rs` wires subscriber → dispatcher | landed | `2195bd284` |
| **T18 — DnaSignal::KeyRevocation EPR envelope** | **landed** | `7f8972b9c`, `a4a9c60a3`, `26e7e186e`, `423adbb1b` |

The architectural reframe is captured in `/projects/elohim/genesis/docs/superpowers/specs/2026-05-15-dna-signal-as-epr-envelope.md`. Read that first — it's the load-bearing context for everything below.

## What changed from your pause-note expectations

You acknowledged the reframe in your reply. To restate it for the record:

**Your T5/T6 consumer now reads `DnaSignal::KeyRevocation`, not `RecoveryV2Signal::KeyRevocationEffective`.**

The legacy `RecoveryV2Signal::KeyRevocationEffective` is marked `#[deprecated(note = "T18: superseded by DnaSignal::KeyRevocation envelope; remove after one release cycle")]` and continues to emit alongside the new envelope at all three producer sites for back-compat during the transition window. Your W2B branch can drop the legacy reader as soon as you've migrated.

## Consumer contract — what your T5/T6 implements

**Wire shape:** `DnaSignal::KeyRevocation` envelope, see schema at `elohim/sdk/schemas/v1/dna-signals/key-revocation.schema.json`:

```
{
  type:             "keyRevocation",         // outer enum tag from DnaSignal
  attestationKind:  "attestation:key-revocation-emit",
  subjectCid:       string,                  // CID of the governance-action:key-revocation Content entry on elohim DNA
  issuer:           string,                  // base64-STANDARD pubkey of authoring elohim
  issuedAt:         RFC3339 string,
  signature:        base64-STANDARD ed25519 sig over canonical envelope bytes,
  metadata: {
    revocationId:           string,          // STABLE LOGICAL KEY — dedupe on this
    revokedPubkey:          string,          // base64 32-byte ed25519
    agentCid:               string,          // Stage 1: humanId; Stage 2: imagodei Human CID
    compromiseAt:           RFC3339 string,  // M4: == effectiveAt; future may diverge
    effectiveAt:            RFC3339 string,
    triggeringRevocationId: string | null,   // back-pointer to driving request (vote_id for voted path)
    supersedesCid:          string | null    // prior revocation CID this supersedes (null on initial CREATE)
  },
  relayChain:       []                       // empty in T18, opaque placeholder for future relay-elohim provenance
}
```

**Three-step verification on receipt (mandatory):**

1. **Schema-validate offline** against `dna-signal-stream.schema.json` (oneOf, picks up key-revocation). Reject on shape failure; don't poison the projector.
2. **Verify signature** over canonical envelope bytes — see "Canonical bytes" below. If verification fails, log + drop. Do not write to your projection table.
3. **Consult notary** for authority + currentness:
   - Authority: ask elohim-storage's HTTP layer (or cross-DNA bridge if you're in-zome) for the Content entry at `subjectCid`. The notary's storage projection serves only validated entries — return = authoritative.
   - Currentness: check `supersedesCid` chain — if a downstream entry supersedes this one, current effectiveness has moved.

**Canonical bytes for signature verification:**

```
rmp_serde::encode::Serializer::new(buf).with_struct_map() of {
  attestationKind:  string,
  subjectCid:       string,
  issuer:           string,
  issuedAt:         string,
  metadata: {
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

**Excluded:** `signature` and `relayChain`. The sub-struct must be serialized in struct-map form (field names as strings), camelCase, in struct-declaration order. The M4 producer side uses identical encoding — see `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` `canonical_envelope_bytes` for the reference implementation. Storage side mirror is in `elohim/elohim-storage/src/signals.rs` (with a sync-by-comment pointing to the spec doc).

**Verification code path** (reference, already implemented in `signals.rs`):

```rust
let issuer_bytes = base64::decode_engine(&envelope.issuer, &STANDARD)?;
let signature_bytes = base64::decode_engine(&envelope.signature, &STANDARD)?;
let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&issuer_bytes.try_into()?)?;
let signature = ed25519_dalek::Signature::from_bytes(&signature_bytes.try_into()?);
let canonical = canonical_envelope_bytes(&envelope)?;
verifying_key.verify_strict(&canonical, &signature)?;
```

## What you should add in your T5/T6

1. **Consumer arm** in your W2B branch that reads `DnaSignal::KeyRevocation` envelopes from the signal stream. Mirror the canonical-bytes helper (or import from a shared crate if you extract one — coordinate with M4 if you do).
2. **Dedup** on `metadata.revocationId` — this is the logical key M4 emits. The format is `rev-{humanId}-{ts}`; treat it as opaque.
3. **Lineage** via `metadata.supersedesCid` — a non-null value indicates this envelope replaces a prior one. Your projection should chain them, not overwrite.
4. **Retroactive attestation invalidation** (your A.8 controller sweep) keys off `metadata.compromiseAt` — any EPR attestation referencing the revoked key, issued after compromiseAt, is tainted. M4 currently sets `compromiseAt == effectiveAt`; if your sweep semantics require an earlier compromise window, file a M4-followup backlog item.
5. **Don't touch** the existing `RevocationAttestation` arm — that's the vote-aggregation signal (separate signal class, still flat shape, no envelope migration scoped). Your T7 already lands that arm and it's unaffected.

## What's deferred (not in T18, not in your immediate scope)

- Migration of `KeyRotationCommitted`, `RevocationVoteSubmitted`, `AgentPeerBindingCreated` to envelope shape. T17 audit matrix in commit `d00752e86` lists each. Backlog items at `genesis/data/timeline/backlog/`.
- Typed `RelayAttestation` struct replacing `Vec<serde_json::Value>` for `relayChain`. Future task when relay-elohims are running.
- Stage 2 `agentCid` migration (humanId → imagodei Human CID). Tracked separately.

## Coordination notes

- M4 sprint will continue through T19–T28 (Shamir wiring, UX audit, cucumber features, branch retirement, sprint acceptance) in parallel with your W2B resumption.
- T28 (sprint acceptance) requires the full test suite + orchestrator dev SUCCESS×2. Your W2B branch landing before T28 is helpful but not blocking — M4 can complete with W2B parallel.
- If you discover anything in the envelope shape that doesn't fit your A.8 sweep semantics, file the issue on the M4 worktree and we'll converge.

## Files to read first

1. `genesis/docs/superpowers/specs/2026-05-15-dna-signal-as-epr-envelope.md` — architectural decision + canonical bytes spec
2. `elohim/sdk/schemas/v1/dna-signals/key-revocation.schema.json` — envelope schema
3. `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` `canonical_envelope_bytes` — producer side reference
4. `elohim/elohim-storage/src/signals.rs` `handle_imagodei_dna_signal` — consumer side reference (already verifies signatures; you can model on it)

## Memory updates suggested for the convergence entry

The `project_epr2b_recovery_m4_convergence.md` memory entry should be updated to reflect:
- Shared schema is no longer `dna-signal-stream.schema.json` generic envelope; the load-bearing pieces are `dna-signals/key-revocation.schema.json` (envelope) + the canonical-bytes spec
- Resumption gate now satisfied — both sides can converge
- Add a one-line "future relay-chain provenance is the wisdom-layer evolution path"

That's the handoff. Resume when ready.
