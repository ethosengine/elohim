//! Cross-signed transport-identity binding — the **signature algebra core** (C2-S1).
//!
//! This module is the pure, self-contained cryptographic heart of the
//! `AgentPeerBinding` cross-signature that replaces `STAGE1_SIGNATURE_SENTINEL`
//! (see `identity_binding_gossip.rs`). It defines the canonical bytes both keys
//! attest, the proof that carries the two signatures, and the two-half Ed25519
//! verification. It has **no storage, DHT, transport, async, or serde
//! dependency** — only `ed25519-dalek`.
//!
//! ## What a binding asserts
//! A `(agent_cid, transport_id)` pair — "agent X owns transport endpoint Y" —
//! attested **bidirectionally** so neither side is unilaterally claimable:
//! - the **transport** key signs over the core under the *transport-attests-agent*
//!   domain ("I, this transport key, serve agent_cid");
//! - the **agent** (current head) key signs over the core under the
//!   *agent-attests-transport* domain ("I, this agent, vouch for transport_id").
//!
//! ## What this module DOES (necessary)
//! - `canonical_bytes(domain, core)` — the exact, length-prefixed,
//!   domain-separated preimage each key signs (serializer-independent; never
//!   `serde_json::Value`/MessagePack map-ordering — those are not canonical).
//! - `verify_binding_signatures(core, proof)` — both Ed25519 halves verify
//!   (`verify_strict`, malleability-rejecting) over their domain-separated bytes,
//!   plus version + kind + nonce/issued_at consistency.
//!
//! ## What this module does NOT do (necessary-but-not-SUFFICIENT — later sessions)
//! Verifying the two signatures does **not**, by itself, prove a binding is
//! trustworthy. The following are DELIBERATELY out of this layer:
//! - **transport-id binding (C2-S2):** proving `proof.transport_pubkey` actually
//!   derives to `core.transport_id` (libp2p `PeerId` / iroh `NodeId`). Without it,
//!   a valid signature by a key *unrelated to the claimed transport_id* would pass
//!   here. That derivation needs the transport libraries and lives in S2.
//! - **freshness / supersession-cutoff (C2-S3):** anchored on the **notarized DHT
//!   action timestamp**, NEVER on the self-asserted `issued_at` in the core (a
//!   compromised retired key would backdate it). `issued_at` is signed here only
//!   as bound entropy; it carries no security authority at this layer.
//! - **lineage-currency (C2-S6):** confirming `proof.agent_pubkey` is a
//!   non-superseded head of `agent_cid` at the notarized time (needs the head
//!   resolver + pubkey timeline).
//! - **wire (de)serialization (C2-S4):** `CrossSignatureProof` here carries no
//!   `serde` derive; the wire encoding + `proof_status` projection are S4.
//!
//! ## Red-team fixes baked in (2026-07-18 design review)
//! - **No `binding_action_hash` in the signed core** — it would be circular (the
//!   proof rides *inside* the entry whose ActionHash it would name). Anti-cross-
//!   binding-lift is instead fully carried by `(agent_cid, transport_id,
//!   transport_kind, …)` being inside both domain-separated halves.
//! - **`valid_until` present/absent flag** — `None` and `Some("")` MUST encode to
//!   different bytes (else one signature is valid for both).
//! - **`verify_strict`** (not `verify`) — rejects signature malleability / small-order.
//! - **Domain separation** — distinct equal-length tags make the two halves
//!   non-interchangeable (an agent-half signature can never satisfy the transport slot).
//!
//! ## Extraction note (Tier-2)
//! If a later session moves the two self-contained verifies into the imagodei
//! **integrity** zome (the only path to a *notarized* — not receiver-local —
//! guarantee; a DNA-hash move, which is a normal dev reinstall), this module must
//! be **extracted verbatim** to a shared `wasm32`-compatible crate and depended on
//! by BOTH the zome and elohim-storage. The canonical encoder MUST be
//! byte-identical on both sides — NEVER reimplement it (the BritCid/BlobCid
//! divergence trap). Lineage-currency stays in the coordinator (HDI has no
//! `get_links`).
//!
//! ## Encoder-choice note (conscious, re: interface-first-reuse)
//! `canonical_bytes` is a bespoke length-prefixed encoder, NOT the `elohim-epr`
//! IPLD/dag-cbor codec. Deliberate: epr's `Envelope::canonical_bytes` is
//! content-atom-specific (claims/coupling/kind/reach/schemaRef — none of which a
//! transport binding has) and epr is not depended on by any zome, so its
//! `wasm32` safety for the Tier-2 extraction path is unproven. This is a
//! signature *preimage*, not a content-address/CID — the identity-math reuse rule
//! does not apply. There is exactly one implementation (here); the extraction
//! note above is what keeps it from being re-derived.

use ed25519_dalek::{Signature, VerifyingKey};

/// The cross-signature scheme version carried in every proof. Bump only on a
/// breaking change to `canonical_bytes` or the proof layout.
pub const SCHEME_VERSION: u16 = 1;

/// Domain tag for the **agent-attests-transport** half (the agent/head key vouches
/// for the transport id). Equal length to [`TRANSPORT_DOMAIN`] by construction.
pub const AGENT_DOMAIN: &[u8] = b"elohim:apb:agent-attests-transport:v1";
/// Domain tag for the **transport-attests-agent** half (the transport key attests
/// the agent_cid). Equal length to [`AGENT_DOMAIN`] by construction.
pub const TRANSPORT_DOMAIN: &[u8] = b"elohim:apb:transport-attests-agent:v1";

/// `transport_kind` discriminator: libp2p PeerId.
pub const TRANSPORT_KIND_LIBP2P: u8 = 0;
/// `transport_kind` discriminator: iroh NodeId.
pub const TRANSPORT_KIND_IROH: u8 = 1;

/// The immutable tuple both keys attest. Assembled by the verifier from the
/// notarized `AgentPeerBinding` entry fields plus the proof's `nonce`/`issued_at`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingCore {
    /// `uhCAk…` — the canonical agent identity being bound.
    pub agent_cid: String,
    /// `12D3Koo…` (libp2p) or iroh `NodeId` — the transport endpoint. Resolves TO
    /// `agent_cid`; never string-compared against it.
    pub transport_id: String,
    /// [`TRANSPORT_KIND_LIBP2P`] or [`TRANSPORT_KIND_IROH`].
    pub transport_kind: u8,
    /// RFC3339 — start of the binding's validity window.
    pub valid_from: String,
    /// RFC3339 — end of validity, or `None` for open-ended. (S4 will reject/clamp
    /// `None`; at this layer it is only encoded.)
    pub valid_until: Option<String>,
    /// Signed entropy (base64). NOT a replay guard at this layer (see module doc).
    pub nonce: String,
    /// RFC3339 claimed issue time — signed, but carries NO security authority here
    /// (freshness is anchored on the notarized action timestamp in S3).
    pub issued_at: String,
}

/// The bidirectional cross-signature proof. Rides in `AgentPeerBinding.signature`
/// on the wire (S4 adds the serde encoding); at this layer it is a plain struct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossSignatureProof {
    /// Must equal [`SCHEME_VERSION`].
    pub scheme_version: u16,
    /// Must equal the core's `transport_kind`.
    pub transport_kind: u8,
    /// Ed25519 public key of the transport endpoint (the transport half's signer).
    pub transport_pubkey: [u8; 32],
    /// Ed25519 signature over `canonical_bytes(TRANSPORT_DOMAIN, core)`.
    pub transport_signature: [u8; 64],
    /// Ed25519 public key that signed the agent half (a head in `agent_cid`'s
    /// lineage — checked in S6, not here).
    pub agent_pubkey: [u8; 32],
    /// Ed25519 signature over `canonical_bytes(AGENT_DOMAIN, core)`.
    pub agent_signature: [u8; 64],
    /// Must equal the core's `nonce`.
    pub nonce: String,
    /// Must equal the core's `issued_at`.
    pub issued_at: String,
}

/// Why a proof's signature algebra failed. Every variant is a hard rejection —
/// there is no permissive fallback at this layer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    /// `proof.scheme_version` is not [`SCHEME_VERSION`].
    #[error("unsupported cross-signature scheme_version {0} (expected {SCHEME_VERSION})")]
    UnsupportedSchemeVersion(u16),
    /// `core.transport_kind` and `proof.transport_kind` disagree.
    #[error("transport_kind mismatch (core={core}, proof={proof})")]
    TransportKindMismatch { core: u8, proof: u8 },
    /// The core the caller assembled does not use the proof's `nonce`.
    #[error("nonce mismatch between assembled core and proof")]
    NonceMismatch,
    /// The core the caller assembled does not use the proof's `issued_at`.
    #[error("issued_at mismatch between assembled core and proof")]
    IssuedAtMismatch,
    /// `proof.transport_pubkey` is not a canonical Ed25519 point.
    #[error("transport public key is not a valid Ed25519 verifying key")]
    InvalidTransportKey,
    /// `proof.agent_pubkey` is not a canonical Ed25519 point.
    #[error("agent public key is not a valid Ed25519 verifying key")]
    InvalidAgentKey,
    /// The transport half does not verify over the transport-attests-agent bytes.
    #[error("transport-half signature is invalid over the transport-attests-agent payload")]
    TransportSignatureInvalid,
    /// The agent half does not verify over the agent-attests-transport bytes.
    #[error("agent-half signature is invalid over the agent-attests-transport payload")]
    AgentSignatureInvalid,
}

/// Append a big-endian u32 length prefix followed by the bytes.
fn write_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
}

/// The exact, serializer-independent preimage a key signs for one half of the
/// binding. Every field is length-prefixed (so no field boundary is ambiguous),
/// `transport_kind` is a byte, and `valid_until` carries a 1-byte present/absent
/// flag so `None` and `Some("")` never collide. The `domain_tag` is itself
/// length-prefixed, making the two halves injective across domains regardless of
/// tag length.
pub fn canonical_bytes(domain_tag: &[u8], core: &BindingCore) -> Vec<u8> {
    let mut buf = Vec::new();
    write_len_prefixed(&mut buf, domain_tag);
    write_len_prefixed(&mut buf, core.agent_cid.as_bytes());
    write_len_prefixed(&mut buf, core.transport_id.as_bytes());
    buf.push(core.transport_kind);
    write_len_prefixed(&mut buf, core.valid_from.as_bytes());
    match &core.valid_until {
        Some(v) => {
            buf.push(1);
            write_len_prefixed(&mut buf, v.as_bytes());
        }
        None => buf.push(0),
    }
    write_len_prefixed(&mut buf, core.nonce.as_bytes());
    write_len_prefixed(&mut buf, core.issued_at.as_bytes());
    buf
}

/// Verify BOTH Ed25519 halves of a cross-signature proof against the assembled
/// core. `Ok(())` iff the version + kind + nonce/issued_at are consistent AND both
/// `verify_strict` checks pass. See the module doc for what this does NOT prove
/// (transport-id binding, freshness, lineage-currency).
pub fn verify_binding_signatures(
    core: &BindingCore,
    proof: &CrossSignatureProof,
) -> Result<(), VerifyError> {
    if proof.scheme_version != SCHEME_VERSION {
        return Err(VerifyError::UnsupportedSchemeVersion(proof.scheme_version));
    }
    if core.transport_kind != proof.transport_kind {
        return Err(VerifyError::TransportKindMismatch {
            core: core.transport_kind,
            proof: proof.transport_kind,
        });
    }
    // The core the caller assembled must use this proof's nonce/issued_at (they
    // are carried by the proof, folded into the signed bytes). Defensive: catches
    // an assembly bug before it reads as a signature failure.
    if core.nonce != proof.nonce {
        return Err(VerifyError::NonceMismatch);
    }
    if core.issued_at != proof.issued_at {
        return Err(VerifyError::IssuedAtMismatch);
    }
    // Transport half: the transport key attests the agent_cid.
    let transport_vk = VerifyingKey::from_bytes(&proof.transport_pubkey)
        .map_err(|_| VerifyError::InvalidTransportKey)?;
    let transport_sig = Signature::from_bytes(&proof.transport_signature);
    transport_vk
        .verify_strict(&canonical_bytes(TRANSPORT_DOMAIN, core), &transport_sig)
        .map_err(|_| VerifyError::TransportSignatureInvalid)?;
    // Agent half: the agent (head) key attests the transport_id.
    let agent_vk =
        VerifyingKey::from_bytes(&proof.agent_pubkey).map_err(|_| VerifyError::InvalidAgentKey)?;
    let agent_sig = Signature::from_bytes(&proof.agent_signature);
    agent_vk
        .verify_strict(&canonical_bytes(AGENT_DOMAIN, core), &agent_sig)
        .map_err(|_| VerifyError::AgentSignatureInvalid)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn sample_core() -> BindingCore {
        BindingCore {
            agent_cid: "uhCAkAGENTexample".to_string(),
            transport_id: "12D3KooWTransportExample".to_string(),
            transport_kind: TRANSPORT_KIND_LIBP2P,
            valid_from: "2026-07-18T00:00:00Z".to_string(),
            valid_until: Some("2026-08-18T00:00:00Z".to_string()),
            nonce: "bm9uY2UtYmFzZTY0".to_string(),
            issued_at: "2026-07-18T00:00:00Z".to_string(),
        }
    }

    /// Build a fully-valid proof for `core`, signed by the given transport + agent keys.
    fn valid_proof(
        core: &BindingCore,
        transport_sk: &SigningKey,
        agent_sk: &SigningKey,
    ) -> CrossSignatureProof {
        let transport_sig = transport_sk.sign(&canonical_bytes(TRANSPORT_DOMAIN, core));
        let agent_sig = agent_sk.sign(&canonical_bytes(AGENT_DOMAIN, core));
        CrossSignatureProof {
            scheme_version: SCHEME_VERSION,
            transport_kind: core.transport_kind,
            transport_pubkey: transport_sk.verifying_key().to_bytes(),
            transport_signature: transport_sig.to_bytes(),
            agent_pubkey: agent_sk.verifying_key().to_bytes(),
            agent_signature: agent_sig.to_bytes(),
            nonce: core.nonce.clone(),
            issued_at: core.issued_at.clone(),
        }
    }

    // ── encoder ────────────────────────────────────────────────────────────

    #[test]
    fn canonical_bytes_is_deterministic() {
        let core = sample_core();
        assert_eq!(
            canonical_bytes(AGENT_DOMAIN, &core),
            canonical_bytes(AGENT_DOMAIN, &core),
            "same domain + core must encode identically"
        );
    }

    #[test]
    fn distinct_domains_produce_distinct_bytes() {
        let core = sample_core();
        assert_ne!(
            canonical_bytes(AGENT_DOMAIN, &core),
            canonical_bytes(TRANSPORT_DOMAIN, &core),
            "the two halves must not be interchangeable"
        );
    }

    #[test]
    fn valid_until_none_differs_from_empty_string() {
        let mut none_core = sample_core();
        none_core.valid_until = None;
        let mut empty_core = sample_core();
        empty_core.valid_until = Some(String::new());
        assert_ne!(
            canonical_bytes(AGENT_DOMAIN, &none_core),
            canonical_bytes(AGENT_DOMAIN, &empty_core),
            "None and Some(\"\") must not collide (red-team MINOR 6)"
        );
    }

    #[test]
    fn field_boundaries_are_unambiguous() {
        // Moving a character across the agent_cid/transport_id boundary must change
        // the bytes — length-prefixing defeats the classic concatenation collision.
        let mut a = sample_core();
        a.agent_cid = "ab".to_string();
        a.transport_id = "c".to_string();
        let mut b = sample_core();
        b.agent_cid = "a".to_string();
        b.transport_id = "bc".to_string();
        assert_ne!(
            canonical_bytes(AGENT_DOMAIN, &a),
            canonical_bytes(AGENT_DOMAIN, &b)
        );
    }

    #[test]
    fn transport_kind_is_covered() {
        let mut a = sample_core();
        a.transport_kind = TRANSPORT_KIND_LIBP2P;
        let mut b = sample_core();
        b.transport_kind = TRANSPORT_KIND_IROH;
        assert_ne!(
            canonical_bytes(AGENT_DOMAIN, &a),
            canonical_bytes(AGENT_DOMAIN, &b)
        );
    }

    // ── verification ─────────────────────────────────────────────────────────

    #[test]
    fn valid_cross_signature_accepts() {
        let core = sample_core();
        let proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        assert_eq!(verify_binding_signatures(&core, &proof), Ok(()));
    }

    #[test]
    fn forged_transport_half_rejected() {
        let core = sample_core();
        let mut proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        // Re-sign the transport half with a DIFFERENT key but keep the advertised pubkey.
        proof.transport_signature = signing_key(9)
            .sign(&canonical_bytes(TRANSPORT_DOMAIN, &core))
            .to_bytes();
        assert_eq!(
            verify_binding_signatures(&core, &proof),
            Err(VerifyError::TransportSignatureInvalid)
        );
    }

    #[test]
    fn forged_agent_half_rejected() {
        let core = sample_core();
        let mut proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        proof.agent_signature = signing_key(9)
            .sign(&canonical_bytes(AGENT_DOMAIN, &core))
            .to_bytes();
        assert_eq!(
            verify_binding_signatures(&core, &proof),
            Err(VerifyError::AgentSignatureInvalid)
        );
    }

    #[test]
    fn domain_swap_rejected() {
        // An attacker lifts a valid AGENT-domain signature into the transport slot.
        let core = sample_core();
        let transport_sk = signing_key(1);
        let agent_sk = signing_key(2);
        let mut proof = valid_proof(&core, &transport_sk, &agent_sk);
        // transport slot now holds a signature over the AGENT domain by the transport key.
        proof.transport_signature = transport_sk
            .sign(&canonical_bytes(AGENT_DOMAIN, &core))
            .to_bytes();
        assert_eq!(
            verify_binding_signatures(&core, &proof),
            Err(VerifyError::TransportSignatureInvalid),
            "domain separation must block cross-context signature lift"
        );
    }

    #[test]
    fn tampered_core_rejected() {
        let core = sample_core();
        let proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        // Verify the untouched proof against a core with a mutated transport_id.
        let mut mutated = core.clone();
        mutated.transport_id = "12D3KooWAttacker".to_string();
        assert!(
            verify_binding_signatures(&mutated, &proof).is_err(),
            "signatures bind the core; a changed field must fail"
        );
    }

    #[test]
    fn unsupported_scheme_version_rejected() {
        let core = sample_core();
        let mut proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        proof.scheme_version = SCHEME_VERSION + 1;
        assert_eq!(
            verify_binding_signatures(&core, &proof),
            Err(VerifyError::UnsupportedSchemeVersion(SCHEME_VERSION + 1))
        );
    }

    #[test]
    fn transport_kind_mismatch_rejected() {
        let core = sample_core(); // libp2p
        let mut proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        proof.transport_kind = TRANSPORT_KIND_IROH;
        assert_eq!(
            verify_binding_signatures(&core, &proof),
            Err(VerifyError::TransportKindMismatch {
                core: TRANSPORT_KIND_LIBP2P,
                proof: TRANSPORT_KIND_IROH
            })
        );
    }

    #[test]
    fn nonce_mismatch_rejected() {
        let core = sample_core();
        let mut proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        proof.nonce = "ZGlmZmVyZW50".to_string();
        assert_eq!(
            verify_binding_signatures(&core, &proof),
            Err(VerifyError::NonceMismatch)
        );
    }

    #[test]
    fn issued_at_mismatch_rejected() {
        let core = sample_core();
        let mut proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        proof.issued_at = "2020-01-01T00:00:00Z".to_string();
        assert_eq!(
            verify_binding_signatures(&core, &proof),
            Err(VerifyError::IssuedAtMismatch)
        );
    }

    #[test]
    fn garbage_transport_pubkey_rejected() {
        let core = sample_core();
        let mut proof = valid_proof(&core, &signing_key(1), &signing_key(2));
        // A pubkey unrelated to the real signer. Whether these bytes decompress to
        // a curve point is a property of ed25519-dalek, not of this module: if they
        // fail decompression it rejects as InvalidTransportKey, otherwise the
        // signature can't verify against them so it rejects as
        // TransportSignatureInvalid. Both are hard rejections — a garbage key is
        // NEVER accepted, and the fallible `from_bytes` is never unwrapped/panicked.
        proof.transport_pubkey = [0xFF; 32];
        let result = verify_binding_signatures(&core, &proof);
        assert!(
            matches!(
                result,
                Err(VerifyError::InvalidTransportKey) | Err(VerifyError::TransportSignatureInvalid)
            ),
            "a garbage transport pubkey must be rejected, got {result:?}"
        );
    }
}
