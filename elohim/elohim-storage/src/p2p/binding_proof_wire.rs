//! Cross-signed binding proof — **wire encoding + the classification chokepoint** (C2-S4).
//!
//! [`binding_cross_signature`](super::binding_cross_signature) is the pure
//! signature algebra (C2-S1): it answers "do these two Ed25519 halves sign this
//! core?" and deliberately carries no `serde`, no policy, and no storage. This
//! module is the layer directly above it — the one that turns a **string on the
//! wire** into a **classification a projection row may carry**:
//!
//! 1. **Envelope codec.** An `AgentPeerBinding.signature` field is a string. A
//!    real proof rides it as `elohim:apb:1:<base64url-nopad(msgpack(ProofWire))>`.
//!    Anything else — the `STAGE1_SIGNATURE_SENTINEL`, an empty field, a
//!    truncated blob, attacker-chosen bytes — decodes to an error, never a panic
//!    (the `EprRouter`-poisoned-row class: one bad row must not empty a
//!    projection).
//! 2. **The chokepoint.** [`classify_binding_signature`] is the ONLY function in
//!    this crate that can produce a [`BindingProofStatus`] whose value is
//!    `cross_signed`. The status type's inner enum is private, so no other
//!    writer — however careless — can mint a verified-looking row by assigning a
//!    string. That is the "fail-closed classification, type-level and not a
//!    remembered `WHERE`" the 2026-07-18 red-team review required.
//!
//! ## What `cross_signed` here does and does not mean
//!
//! `cross_signed` means: this node verified, locally, that the transport key and
//! the agent key each signed over the other's identity in their own
//! domain-separated preimage, AND the binding carries a bounded validity window.
//! It does NOT mean the binding is notarized-verified: Tier-1 verification is
//! **receiver-local**, so a DHT-direct third party can still be shown a
//! shape-only forged entry until the integrity-zome fold (C2-S7) lands. It also
//! does not yet prove the transport *public key* derives to the claimed
//! `transport_id` (C2-S2), nor that the agent key is a non-superseded head
//! (C2-S6). Each of those narrows the gap further; none of them is assumed here.
//!
//! Consequence for consumers: `cross_signed` is *necessary* for economic
//! attribution, and today it is not yet *sufficient* for third-party trust.
//! Nothing in the tree may treat `unverified` as attributable — that is the
//! habit `identity-cross-signed` exists to keep.
//!
//! ## Why the validity window is policy here and not in the algebra
//!
//! The algebra signs `valid_from`/`valid_until` as bound fields but takes no view
//! on their values. Refusing `valid_until: None` (so every credential expires and
//! is re-minted under present key control) is a *projection policy*: it belongs
//! where a row is classified, not where signatures are checked. Freshness itself
//! is deliberately NOT anchored on the self-asserted `issued_at` — see the
//! "pincer" resolution in `backlog/agent-peer-binding-signing.md`: the durable
//! anchor is the notarized DHT action timestamp, which arrives with C2-S3.
//!
//! ## Encoder-choice note (re: interface-first-reuse)
//!
//! Nothing here is a CID, a content address, or a fingerprint, so the canonical
//! addressing homes (`eprfs-core::BlobCid`, `elohim-epr`'s dag-cbor codec) are
//! not the interface being re-derived. The *signed preimage* is
//! `binding_cross_signature::canonical_bytes` — reused verbatim, never
//! reimplemented, because the Tier-2 extraction path (C2-S7) requires it to stay
//! byte-identical on both sides of the zome boundary. The envelope BODY is
//! `rmp-serde`, matching the sibling gossip payload in
//! `identity_binding_gossip.rs`; MessagePack map-ordering is safe for transport
//! precisely because it is never the thing signed.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use super::binding_cross_signature::{
    verify_binding_signatures, BindingCore, CrossSignatureProof, SCHEME_VERSION,
};

/// Prefix marking a `signature` field that claims to carry a cross-signature
/// proof. Version-tagged so a future scheme can coexist rather than
/// retro-invalidate: an unknown prefix classifies `unverified`, it never panics
/// and never throws the row away.
pub const PROOF_ENVELOPE_PREFIX: &str = "elohim:apb:1:";

/// Maximum validity window a cross-signed binding may claim, in days.
///
/// Red-team requirement: reject open-ended (`valid_until: None`) credentials so
/// every binding expires and must be re-minted under *present* key control. A
/// window longer than this classifies `unverified` rather than being clamped —
/// clamping would silently disagree with the signed bytes.
pub const MAX_VALIDITY_DAYS: i64 = 90;

/// Why a `signature` field did not yield a usable proof. Every variant is a
/// classification outcome, never a panic and never a hard error for the caller:
/// a row that fails to decode is projected as `unverified`, not dropped.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProofDecodeError {
    /// The field is empty.
    #[error("signature field is empty")]
    Empty,
    /// The field carries no proof envelope — the Stage-1 sentinel and every
    /// legacy/foreign value lands here. This is the COMMON case today, and it is
    /// the honest one: those bindings are self-asserted.
    #[error("signature field is not a cross-signature envelope (self-asserted)")]
    NotAnEnvelope,
    /// The envelope body is not valid base64.
    #[error("cross-signature envelope body is not valid base64")]
    Base64,
    /// The decoded bytes are not a well-formed proof payload.
    #[error("cross-signature envelope body is not a well-formed proof payload")]
    Malformed,
    /// A fixed-width key or signature field had the wrong length.
    #[error(
        "cross-signature proof field has wrong length ({field}: expected {expected}, got {got})"
    )]
    FieldLength {
        /// Which field was mis-sized.
        field: &'static str,
        /// Expected byte length.
        expected: usize,
        /// Actual byte length.
        got: usize,
    },
}

/// MessagePack wire shape of a [`CrossSignatureProof`].
///
/// Fixed-width keys/signatures travel as `Vec<u8>` because `serde` has no
/// blanket impl for `[u8; 64]`; the lengths are validated on decode, which is
/// exactly where an attacker-controlled payload must be checked rather than
/// unwrapped. Field names are short because this rides a gossip payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProofWire {
    /// scheme_version
    v: u16,
    /// transport_kind
    tk: u8,
    /// transport public key (32)
    tpk: Vec<u8>,
    /// transport signature (64)
    tsg: Vec<u8>,
    /// agent public key (32)
    apk: Vec<u8>,
    /// agent signature (64)
    asg: Vec<u8>,
    /// nonce (base64 string, as signed)
    n: String,
    /// issued_at (RFC3339, as signed)
    ia: String,
}

fn fixed<const N: usize>(field: &'static str, bytes: &[u8]) -> Result<[u8; N], ProofDecodeError> {
    <[u8; N]>::try_from(bytes).map_err(|_| ProofDecodeError::FieldLength {
        field,
        expected: N,
        got: bytes.len(),
    })
}

/// Encode a proof into the `signature` field string.
///
/// Used by the minting path (C2-S2) and by tests; decoding is the security-
/// relevant direction and is where all the validation lives.
pub fn encode_proof(proof: &CrossSignatureProof) -> String {
    let wire = ProofWire {
        v: proof.scheme_version,
        tk: proof.transport_kind,
        tpk: proof.transport_pubkey.to_vec(),
        tsg: proof.transport_signature.to_vec(),
        apk: proof.agent_pubkey.to_vec(),
        asg: proof.agent_signature.to_vec(),
        n: proof.nonce.clone(),
        ia: proof.issued_at.clone(),
    };
    // rmp_serde only fails here on a serializer bug (no IO, no unsupported
    // shape), so an empty body on error degrades to `NotAnEnvelope` on decode —
    // fail-closed, and still no panic.
    let body = rmp_serde::to_vec(&wire).unwrap_or_default();
    format!("{PROOF_ENVELOPE_PREFIX}{}", URL_SAFE_NO_PAD.encode(body))
}

/// Decode a `signature` field into a proof. **Total and panic-free** over
/// attacker-controlled input: every failure is a typed [`ProofDecodeError`].
pub fn decode_proof(signature: &str) -> Result<CrossSignatureProof, ProofDecodeError> {
    if signature.is_empty() {
        return Err(ProofDecodeError::Empty);
    }
    let body = signature
        .strip_prefix(PROOF_ENVELOPE_PREFIX)
        .ok_or(ProofDecodeError::NotAnEnvelope)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(body)
        .map_err(|_| ProofDecodeError::Base64)?;
    let wire: ProofWire = rmp_serde::from_slice(&bytes).map_err(|_| ProofDecodeError::Malformed)?;
    Ok(CrossSignatureProof {
        scheme_version: wire.v,
        transport_kind: wire.tk,
        transport_pubkey: fixed::<32>("transport_pubkey", &wire.tpk)?,
        transport_signature: fixed::<64>("transport_signature", &wire.tsg)?,
        agent_pubkey: fixed::<32>("agent_pubkey", &wire.apk)?,
        agent_signature: fixed::<64>("agent_signature", &wire.asg)?,
        nonce: wire.n,
        issued_at: wire.ia,
    })
}

/// Private inner classification. Kept out of the public API so that
/// [`classify_binding_signature`] is the only constructor of the verified value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Unverified,
    CrossSigned,
}

/// The proof standing of one binding row.
///
/// **The point of this type is what you CANNOT do with it.** Its inner value is
/// private, so `BindingProofStatus::unverified()` is the only status any writer
/// can name by hand; the `cross_signed` value exists solely as the return of
/// [`classify_binding_signature`], which cannot produce it without two verifying
/// Ed25519 halves. A future writer that forgets the rule gets a compile error,
/// not a silently-attributable row.
///
/// It is the `proof_status` column's type on the insert model
/// ([`crate::db::models::NewPeerIdentityBindingRow`]) so the guarantee holds at
/// the database boundary rather than in a reviewer's memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, diesel::expression::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct BindingProofStatus(Status);

impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::sqlite::Sqlite>
    for BindingProofStatus
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.as_str().to_string());
        Ok(diesel::serialize::IsNull::No)
    }
}

/// Column value for an unverified (self-asserted) binding.
pub const PROOF_STATUS_UNVERIFIED: &str = "unverified";
/// Column value for a locally cross-signature-verified binding.
pub const PROOF_STATUS_CROSS_SIGNED: &str = "cross_signed";

impl BindingProofStatus {
    /// The fail-closed default every non-verifying writer uses.
    pub fn unverified() -> Self {
        Self(Status::Unverified)
    }

    /// Did this row's signature verify as a cross-signature?
    pub fn is_cross_signed(self) -> bool {
        matches!(self.0, Status::CrossSigned)
    }

    /// Persisted column value. Positive-match (`= 'cross_signed'`) is the only
    /// legal read gate — never `!= 'unverified'`, which admits every future
    /// status string that has not been reasoned about.
    pub fn as_str(self) -> &'static str {
        match self.0 {
            Status::Unverified => PROOF_STATUS_UNVERIFIED,
            Status::CrossSigned => PROOF_STATUS_CROSS_SIGNED,
        }
    }
}

impl Default for BindingProofStatus {
    fn default() -> Self {
        Self::unverified()
    }
}

impl std::fmt::Display for BindingProofStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// =============================================================================
// Identity derivation (C2-S2) — a signature is only a proof if the KEY that
// signed is the key the claimed identity NAMES.
// =============================================================================

/// HoloHash prefix bytes for `AgentPubKey` (`uhCAk…`). A HoloHash is 39 bytes:
/// 3 prefix + 32 core + 4 DHT-location.
const HOLO_HASH_AGENT_PREFIX: [u8; 3] = [0x84, 0x20, 0x24];
/// Total raw byte length of any HoloHash.
const HOLO_HASH_LEN: usize = 39;

/// Recover the raw 32-byte Ed25519 public key an `agent_cid` names, when
/// `agent_cid` is a HoloHash `AgentPubKey` (`uhCAk…`).
///
/// Returns `None` for every other namespace — a genesis-seeder human slug
/// (`"matthew"`, see `genesis/seeder/src/seed-agent-bindings.ts`), an Agent-EPR
/// CIDv1, an empty field, attacker-chosen bytes. That is deliberately
/// fail-closed: binding a key to *those* identities needs the head/lineage
/// resolver (C2-S6), and until it exists a proof over them cannot be
/// distinguished from a proof by an unrelated key.
///
/// The DHT-location suffix is NOT checked: it is derived from the core and
/// carries no authority. Encoding convention mirrors
/// [`crate::signals::HoloHashB64`], which is where this crate already
/// hand-holds the holochain base64 form — `holo_hash` is a transitive
/// dependency with two versions resolved in the lockfile and is deliberately
/// not taken as a direct dep here (see `Cargo.toml`'s holo_hash pin note).
///
/// Total and panic-free over attacker-controlled input.
pub fn agent_pubkey_from_agent_cid(agent_cid: &str) -> Option<[u8; 32]> {
    let body = agent_cid.strip_prefix('u')?;
    // Bound the work before decoding: a HoloHash is exactly 39 bytes, so its
    // base64url-no-pad form is exactly 52 characters. A 100k-character field
    // never reaches the decoder.
    if body.len() != 52 {
        return None;
    }
    let bytes = URL_SAFE_NO_PAD.decode(body).ok()?;
    if bytes.len() != HOLO_HASH_LEN || bytes[0..3] != HOLO_HASH_AGENT_PREFIX {
        return None;
    }
    <[u8; 32]>::try_from(&bytes[3..35]).ok()
}

/// The inverse of [`agent_pubkey_from_agent_cid`] — the `uhCAk…` string naming
/// this Ed25519 key.
///
/// The 4-byte DHT-location suffix is emitted as zeros: this crate does not
/// compute holochain's location fold, and the verifier deliberately does not
/// read it. Use this to construct fixtures and to cross-check a conductor-
/// supplied `agent_cid`; the minting path takes the node's real `agent_cid`
/// from the identity seam rather than re-deriving one here.
pub fn agent_cid_from_agent_pubkey(pubkey: &[u8; 32]) -> String {
    let mut raw = Vec::with_capacity(HOLO_HASH_LEN);
    raw.extend_from_slice(&HOLO_HASH_AGENT_PREFIX);
    raw.extend_from_slice(pubkey);
    raw.extend_from_slice(&[0u8; 4]);
    format!("u{}", URL_SAFE_NO_PAD.encode(raw))
}

/// The libp2p `PeerId` an Ed25519 public key derives to, in the canonical
/// base58btc form this codebase stores (`PeerId::to_base58()`, which is also
/// `Display`) — see `p2p/mod.rs`'s handshake self-claim and the
/// `peer_identity_bindings.peer_id` column.
///
/// `None` only when libp2p refuses the bytes outright. Note that libp2p does
/// NOT decompress the point here, so garbage bytes generally yield *a* PeerId
/// rather than `None` — which is safe, because the derived id then fails to
/// equal the claimed one, and `verify_binding_signatures` rejects a
/// non-canonical key independently. This check NARROWS the algebra; it never
/// replaces it. Panic-free.
pub fn libp2p_peer_id_from_ed25519_pubkey(pubkey: &[u8; 32]) -> Option<String> {
    let ed = libp2p::identity::ed25519::PublicKey::try_from_bytes(pubkey).ok()?;
    let public = libp2p::identity::PublicKey::from(ed);
    Some(libp2p::PeerId::from(public).to_base58())
}

/// Does `transport_id` actually derive from `transport_pubkey` under
/// `transport_kind`?
///
/// This is the check `binding_cross_signature`'s module doc assigns to C2-S2.
/// Without it, a proof signed by a key **unrelated to the claimed endpoint**
/// verified: both halves are self-consistent, so the algebra alone cannot tell
/// "this endpoint's key attested the agent" from "some key attested the agent
/// while naming someone else's endpoint".
///
/// Only [`TRANSPORT_KIND_LIBP2P`](crate::p2p::binding_cross_signature::TRANSPORT_KIND_LIBP2P)
/// is derivable today, because libp2p is the only namespace any writer
/// classifies from (all four call sites assign it explicitly). An iroh-kind row
/// is refused rather than admitted on an unchecked derivation — there is no
/// iroh classification site to make it correct, and fail-closed is the rule
/// here. Adding one means adding its derivation to this function first.
pub fn transport_id_derives_from(
    transport_kind: u8,
    transport_id: &str,
    transport_pubkey: &[u8; 32],
) -> bool {
    use crate::p2p::binding_cross_signature::TRANSPORT_KIND_LIBP2P;
    if transport_kind != TRANSPORT_KIND_LIBP2P {
        return false;
    }
    libp2p_peer_id_from_ed25519_pubkey(transport_pubkey).as_deref() == Some(transport_id)
}

/// Is a `(valid_from, valid_until)` pair an acceptable, bounded credential window?
///
/// Open-ended bindings are refused outright (see [`MAX_VALIDITY_DAYS`]). Both
/// ends must parse as RFC3339 and be correctly ordered.
fn window_is_bounded(valid_from: &str, valid_until: Option<&str>) -> bool {
    let Some(until) = valid_until else {
        return false;
    };
    let (Ok(from), Ok(until)) = (
        chrono::DateTime::parse_from_rfc3339(valid_from),
        chrono::DateTime::parse_from_rfc3339(until),
    ) else {
        return false;
    };
    let span = until - from;
    span > chrono::Duration::zero() && span <= chrono::Duration::days(MAX_VALIDITY_DAYS)
}

/// **The classification chokepoint.** Decide the proof standing of one binding
/// from the row's own fields plus the `signature` string it carries.
///
/// Returns `cross_signed` only when ALL of the following hold:
/// - the signature field carries a well-formed `elohim:apb:1:` envelope;
/// - the envelope's scheme version is the supported one;
/// - the transport kind the *verifier* assigns (from which table/namespace the
///   id came, never from the payload's claim) matches the proof;
/// - `transport_id` DERIVES from `proof.transport_pubkey`
///   ([`transport_id_derives_from`]) — the signing key must BE this endpoint;
/// - `agent_cid` names `proof.agent_pubkey`
///   ([`agent_pubkey_from_agent_cid`]) — the signing key must BE this agent;
/// - both Ed25519 halves verify (`verify_strict`) over their domain-separated
///   preimages, assembled from these row fields and the proof's own
///   nonce/`issued_at`;
/// - the validity window is present and bounded.
///
/// The two derivation clauses are what make the signatures a *binding* rather
/// than two self-consistent attestations by arbitrary keys. They are why an
/// `agent_cid` outside the HoloHash `AgentPubKey` namespace (a seeder human
/// slug, an Agent-EPR CIDv1) classifies `unverified` today: binding a key to
/// those identities is C2-S6 resolver work, and fail-closed is the answer
/// until it lands.
///
/// Every other input — sentinel, empty, garbage, a valid signature over a
/// *different* core, an open-ended window — returns `unverified`. There is no
/// dev-mode bypass: dev mode governs whether a deployment *enforces* the
/// attribution cut (see `db::peer_identity_bindings`), never whether a
/// self-asserted string counts as a proof.
pub fn classify_binding_signature(
    agent_cid: &str,
    transport_id: &str,
    transport_kind: u8,
    valid_from: &str,
    valid_until: Option<&str>,
    signature: &str,
) -> BindingProofStatus {
    let Ok(proof) = decode_proof(signature) else {
        return BindingProofStatus::unverified();
    };
    if proof.scheme_version != SCHEME_VERSION || proof.transport_kind != transport_kind {
        return BindingProofStatus::unverified();
    }
    if !window_is_bounded(valid_from, valid_until) {
        return BindingProofStatus::unverified();
    }
    // Identity derivation (C2-S2) — cheap structural checks BEFORE the two
    // Ed25519 verifies, so a hostile row costs a base58 encode rather than two
    // curve operations.
    if !transport_id_derives_from(transport_kind, transport_id, &proof.transport_pubkey) {
        return BindingProofStatus::unverified();
    }
    if agent_pubkey_from_agent_cid(agent_cid) != Some(proof.agent_pubkey) {
        return BindingProofStatus::unverified();
    }
    let core = BindingCore {
        agent_cid: agent_cid.to_string(),
        transport_id: transport_id.to_string(),
        transport_kind,
        valid_from: valid_from.to_string(),
        valid_until: valid_until.map(str::to_string),
        nonce: proof.nonce.clone(),
        issued_at: proof.issued_at.clone(),
    };
    match verify_binding_signatures(&core, &proof) {
        Ok(()) => BindingProofStatus(Status::CrossSigned),
        Err(_) => BindingProofStatus::unverified(),
    }
}

/// May a binding be joined onto economic attribution, given ONLY its signature
/// string? **Never** — and that is the answer, not a stub.
///
/// A signature cannot be verified without the fields it signs: the two halves
/// are checked over a domain-separated preimage assembled from `agent_cid`,
/// `transport_id`, `transport_kind` and the validity window. A caller holding a
/// bare string has none of that, so no string — sentinel, envelope, or
/// otherwise — can establish admissibility here. The fail-closed answer is the
/// only sound one, and it is why no such call site should exist: consumers read
/// the persisted `proof_status` classified once at the writer chokepoint
/// ([`classify_binding_signature`]), or call
/// [`binding_admissible_for_attribution_proof`] with the bound core.
///
/// `dev_mode` is accepted and ignored **by design**: dev mode governs whether a
/// deployment enforces the attribution cut, never whether a self-asserted
/// string counts as a proof. There is no dev bypass for attribution.
pub fn binding_admissible_for_attribution(_signature: &str, _dev_mode: bool) -> bool {
    false
}

/// May a binding backed by this proof be joined onto economic attribution?
///
/// True iff both Ed25519 halves verify over the bound core (see
/// [`verify_binding_signatures`]). This is the *signature-algebra* decision —
/// necessary for attribution, and the strongest statement this layer can make.
/// Row-level policy (bounded validity window, transport-kind assignment from
/// the id's own namespace) is applied at the writer chokepoint, and freshness
/// against the notarized DHT action timestamp arrives with C2-S3; a consumer
/// gating on the persisted `cross_signed` status gets all three.
///
/// `dev_mode` is accepted and ignored for the same reason as above.
pub fn binding_admissible_for_attribution_proof(
    core: &BindingCore,
    proof: &CrossSignatureProof,
    _dev_mode: bool,
) -> bool {
    verify_binding_signatures(core, proof).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::binding_cross_signature::{
        canonical_bytes, AGENT_DOMAIN, TRANSPORT_DOMAIN, TRANSPORT_KIND_IROH, TRANSPORT_KIND_LIBP2P,
    };
    use crate::p2p::identity_binding_gossip::STAGE1_SIGNATURE_SENTINEL;
    use ed25519_dalek::{Signer, SigningKey};

    /// The two keys every fixture in this module signs with. The ids below are
    /// DERIVED from them (C2-S2): a fixture that names an arbitrary transport id
    /// or agent cid can no longer classify `cross_signed`, which is the point.
    fn transport_sk() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }
    fn agent_sk() -> SigningKey {
        SigningKey::from_bytes(&[9u8; 32])
    }

    fn transport_id() -> String {
        libp2p_peer_id_from_ed25519_pubkey(&transport_sk().verifying_key().to_bytes())
            .expect("fixture transport key derives a PeerId")
    }
    fn agent_cid() -> String {
        agent_cid_from_agent_pubkey(&agent_sk().verifying_key().to_bytes())
    }

    const FROM: &str = "2026-08-01T00:00:00Z";
    const UNTIL: &str = "2026-08-31T00:00:00Z";

    fn core() -> BindingCore {
        BindingCore {
            agent_cid: agent_cid(),
            transport_id: transport_id(),
            transport_kind: TRANSPORT_KIND_LIBP2P,
            valid_from: FROM.to_string(),
            valid_until: Some(UNTIL.to_string()),
            nonce: "dGVzdC1ub25jZQ".to_string(),
            issued_at: FROM.to_string(),
        }
    }

    fn proof_for(core: &BindingCore) -> CrossSignatureProof {
        let transport_sk = transport_sk();
        let agent_sk = agent_sk();
        CrossSignatureProof {
            scheme_version: SCHEME_VERSION,
            transport_kind: core.transport_kind,
            transport_pubkey: transport_sk.verifying_key().to_bytes(),
            transport_signature: transport_sk
                .sign(&canonical_bytes(TRANSPORT_DOMAIN, core))
                .to_bytes(),
            agent_pubkey: agent_sk.verifying_key().to_bytes(),
            agent_signature: agent_sk
                .sign(&canonical_bytes(AGENT_DOMAIN, core))
                .to_bytes(),
            nonce: core.nonce.clone(),
            issued_at: core.issued_at.clone(),
        }
    }

    fn classify(signature: &str) -> BindingProofStatus {
        classify_binding_signature(
            &agent_cid(),
            &transport_id(),
            TRANSPORT_KIND_LIBP2P,
            FROM,
            Some(UNTIL),
            signature,
        )
    }

    // ── C2-S2: identity derivation ──────────────────────────────────────────

    /// The transport half's public key must DERIVE to the claimed libp2p
    /// `PeerId`. Without this, a valid signature by a key unrelated to the
    /// claimed endpoint passed the algebra (C2-S1's own module doc names this
    /// as the hole S2 closes).
    #[test]
    fn a_transport_pubkey_that_does_not_derive_the_peer_id_classifies_unverified() {
        let mut wrong = core();
        // A well-formed PeerId — just not the one this transport key derives.
        wrong.transport_id = libp2p_peer_id_from_ed25519_pubkey(
            &SigningKey::from_bytes(&[42u8; 32])
                .verifying_key()
                .to_bytes(),
        )
        .expect("derive");
        let signature = encode_proof(&proof_for(&wrong));
        let status = classify_binding_signature(
            &agent_cid(),
            &wrong.transport_id,
            TRANSPORT_KIND_LIBP2P,
            FROM,
            Some(UNTIL),
            &signature,
        );
        assert!(
            !status.is_cross_signed(),
            "a signature by a key that is not this endpoint's key must not certify it"
        );
    }

    #[test]
    fn a_transport_pubkey_that_derives_the_peer_id_classifies_cross_signed() {
        assert!(classify(&encode_proof(&proof_for(&core()))).is_cross_signed());
    }

    /// The agent half's public key must be the one `agent_cid` names. The
    /// degenerate-R1 check: `agent_cid` is a HoloHash `AgentPubKey`, so its
    /// 32-byte core IS the ed25519 key, no resolver needed.
    #[test]
    fn an_agent_pubkey_that_is_not_the_agent_cid_classifies_unverified() {
        let mut wrong = core();
        wrong.agent_cid = agent_cid_from_agent_pubkey(
            &SigningKey::from_bytes(&[43u8; 32])
                .verifying_key()
                .to_bytes(),
        );
        let signature = encode_proof(&proof_for(&wrong));
        let status = classify_binding_signature(
            &wrong.agent_cid,
            &transport_id(),
            TRANSPORT_KIND_LIBP2P,
            FROM,
            Some(UNTIL),
            &signature,
        );
        assert!(
            !status.is_cross_signed(),
            "a key that is not the one agent_cid names must not vouch as that agent"
        );
    }

    /// An `agent_cid` in a namespace we cannot bind to a key (the genesis
    /// seeder's human slug, an EPR CIDv1) is fail-closed until C2-S6's
    /// resolver — never silently admitted on signature algebra alone.
    #[test]
    fn an_unresolvable_agent_cid_namespace_classifies_unverified() {
        for foreign in [
            "matthew",
            "bafyreiexampleagentepridentity",
            "",
            "uhCAkTESTAGENT",
        ] {
            let mut wrong = core();
            wrong.agent_cid = foreign.to_string();
            let signature = encode_proof(&proof_for(&wrong));
            let status = classify_binding_signature(
                foreign,
                &transport_id(),
                TRANSPORT_KIND_LIBP2P,
                FROM,
                Some(UNTIL),
                &signature,
            );
            assert!(
                !status.is_cross_signed(),
                "agent_cid '{foreign}' cannot be bound to a key without C2-S6"
            );
        }
    }

    /// No iroh classification site exists today, so an iroh-kind row is
    /// fail-closed rather than accepted on an unchecked derivation.
    #[test]
    fn a_non_libp2p_transport_kind_classifies_unverified() {
        let mut iroh = core();
        iroh.transport_kind = TRANSPORT_KIND_IROH;
        let signature = encode_proof(&proof_for(&iroh));
        let status = classify_binding_signature(
            &agent_cid(),
            &iroh.transport_id,
            TRANSPORT_KIND_IROH,
            FROM,
            Some(UNTIL),
            &signature,
        );
        assert!(!status.is_cross_signed());
    }

    #[test]
    fn derivation_helpers_are_panic_free_on_hostile_input() {
        for hostile in [
            "",
            "u",
            "uhCAk",
            "uhCAk!!!!",
            "\u{1F4A3}",
            &"u".repeat(100_000),
            "uhCEk_wrong_prefix_kind_here_padding_padding_padding_x",
        ] {
            let _ = agent_pubkey_from_agent_cid(hostile);
            let _ = transport_id_derives_from(TRANSPORT_KIND_LIBP2P, hostile, &[0u8; 32]);
            let _ = transport_id_derives_from(0xFF, hostile, &[0xFFu8; 32]);
        }
        // A non-canonical ed25519 point must not panic the derivation. libp2p
        // does NOT decompress the point here, so it yields *a* PeerId rather
        // than `None` — which is safe: that PeerId will not equal the claimed
        // `transport_id` unless the attacker also controls the claim, and
        // `verify_binding_signatures` rejects the non-canonical key regardless.
        // The derivation check is a NARROWING of the algebra, never a
        // replacement for it.
        let garbage = libp2p_peer_id_from_ed25519_pubkey(&[0xFFu8; 32]);
        assert!(
            garbage.as_deref() != Some(&transport_id()[..]),
            "a garbage key must never derive the honest endpoint's id"
        );
    }

    #[test]
    fn agent_cid_roundtrips_through_the_holo_hash_encoding() {
        let pk = agent_sk().verifying_key().to_bytes();
        let cid = agent_cid_from_agent_pubkey(&pk);
        assert!(
            cid.starts_with("uhCAk"),
            "AgentPubKey multibase form: {cid}"
        );
        assert_eq!(agent_pubkey_from_agent_cid(&cid), Some(pk));
    }

    // ── envelope codec ──────────────────────────────────────────────────────

    #[test]
    fn proof_roundtrips_through_the_envelope() {
        let core = core();
        let proof = proof_for(&core);
        let encoded = encode_proof(&proof);
        assert!(encoded.starts_with(PROOF_ENVELOPE_PREFIX));
        assert_eq!(decode_proof(&encoded), Ok(proof));
    }

    #[test]
    fn the_sentinel_is_not_an_envelope() {
        assert_eq!(
            decode_proof(STAGE1_SIGNATURE_SENTINEL),
            Err(ProofDecodeError::NotAnEnvelope)
        );
    }

    #[test]
    fn an_empty_signature_decodes_as_empty() {
        assert_eq!(decode_proof(""), Err(ProofDecodeError::Empty));
    }

    /// The poisoned-row class: attacker-controlled bytes in the signature field
    /// must never panic a projection sweep, whatever shape they take.
    #[test]
    fn hostile_signature_bodies_never_panic() {
        let hostile = [
            "elohim:apb:1:".to_string(),
            "elohim:apb:1:!!!not-base64!!!".to_string(),
            format!(
                "{PROOF_ENVELOPE_PREFIX}{}",
                URL_SAFE_NO_PAD.encode([0xFFu8; 64])
            ),
            format!(
                "{PROOF_ENVELOPE_PREFIX}{}",
                URL_SAFE_NO_PAD.encode(b"\x00\x01\x02")
            ),
            "elohim:apb:2:AAAA".to_string(),
            "\u{1F4A3}".to_string(),
            format!("{PROOF_ENVELOPE_PREFIX}{}", "A".repeat(100_000)),
        ];
        for s in hostile {
            assert!(decode_proof(&s).is_err(), "should not decode: {s:.40}");
            assert!(
                !classify(&s).is_cross_signed(),
                "hostile input must classify unverified: {s:.40}"
            );
        }
    }

    /// A proof whose fixed-width fields are the wrong length is rejected by
    /// length check, not by an unwrap.
    #[test]
    fn truncated_key_material_is_rejected_by_length() {
        let wire = ProofWire {
            v: SCHEME_VERSION,
            tk: TRANSPORT_KIND_LIBP2P,
            tpk: vec![0u8; 31],
            tsg: vec![0u8; 64],
            apk: vec![0u8; 32],
            asg: vec![0u8; 64],
            n: "n".into(),
            ia: FROM.into(),
        };
        let encoded = format!(
            "{PROOF_ENVELOPE_PREFIX}{}",
            URL_SAFE_NO_PAD.encode(rmp_serde::to_vec(&wire).unwrap())
        );
        assert_eq!(
            decode_proof(&encoded),
            Err(ProofDecodeError::FieldLength {
                field: "transport_pubkey",
                expected: 32,
                got: 31
            })
        );
    }

    // ── classification chokepoint ───────────────────────────────────────────

    #[test]
    fn a_real_cross_signature_classifies_cross_signed() {
        let core = core();
        assert!(classify(&encode_proof(&proof_for(&core))).is_cross_signed());
    }

    #[test]
    fn the_stage1_sentinel_classifies_unverified() {
        assert_eq!(
            classify(STAGE1_SIGNATURE_SENTINEL),
            BindingProofStatus::unverified()
        );
    }

    /// A proof lifted from a DIFFERENT binding must not verify against this row —
    /// the signed core carries both ids, so a lift is a signature failure.
    #[test]
    fn a_proof_lifted_from_another_binding_classifies_unverified() {
        let mut other = core();
        other.transport_id = "12D3KooWATTACKER".to_string();
        let lifted = encode_proof(&proof_for(&other));
        assert!(
            !classify(&lifted).is_cross_signed(),
            "a proof minted for another transport id must not attach to this row"
        );
    }

    /// The verifier assigns the transport kind from the namespace the id came
    /// from; a payload claiming the other kind cannot talk it into agreeing.
    #[test]
    fn a_kind_mismatch_classifies_unverified() {
        let mut iroh_core = core();
        iroh_core.transport_kind = TRANSPORT_KIND_IROH;
        let signature = encode_proof(&proof_for(&iroh_core));
        assert!(!classify(&signature).is_cross_signed());
    }

    #[test]
    fn an_open_ended_window_classifies_unverified() {
        let mut open = core();
        open.valid_until = None;
        let signature = encode_proof(&proof_for(&open));
        let status = classify_binding_signature(
            &agent_cid(),
            &transport_id(),
            TRANSPORT_KIND_LIBP2P,
            FROM,
            None,
            &signature,
        );
        assert!(
            !status.is_cross_signed(),
            "every credential must expire and be re-minted under present key control"
        );
    }

    #[test]
    fn an_over_long_window_classifies_unverified() {
        let far = "2027-08-01T00:00:00Z";
        let mut long = core();
        long.valid_until = Some(far.to_string());
        let signature = encode_proof(&proof_for(&long));
        let status = classify_binding_signature(
            &agent_cid(),
            &transport_id(),
            TRANSPORT_KIND_LIBP2P,
            FROM,
            Some(far),
            &signature,
        );
        assert!(!status.is_cross_signed());
    }

    #[test]
    fn window_bounds_are_ordered_and_parsed() {
        assert!(window_is_bounded(FROM, Some(UNTIL)));
        assert!(!window_is_bounded(FROM, None));
        assert!(!window_is_bounded(UNTIL, Some(FROM)), "reversed window");
        assert!(!window_is_bounded("not-a-date", Some(UNTIL)));
        assert!(!window_is_bounded(FROM, Some("not-a-date")));
        assert!(!window_is_bounded(FROM, Some(FROM)), "zero-length window");
    }

    #[test]
    fn status_column_values_are_stable() {
        assert_eq!(BindingProofStatus::unverified().as_str(), "unverified");
        let core = core();
        assert_eq!(
            classify(&encode_proof(&proof_for(&core))).as_str(),
            "cross_signed"
        );
    }
}
