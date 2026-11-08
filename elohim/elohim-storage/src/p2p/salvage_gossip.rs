//! Salvage-capacity advertisement: the signed wire envelope and its admission.
//!
//! Topic: `elohim/storage/salvage` (gossipsub and iroh-gossip; a dual node fans out
//! to both).
//! Wire format: MessagePack via `rmp_serde` (map-keyed via `to_vec_named`).
//!
//! ## What this is
//!
//! An opt-in, always-on peer (`node`/`steward`) periodically gossips a
//! [`SalvageCapacityAd`] declaring its spare capacity. Receivers project it
//! into the `salvage_capacity` reality table (Category C operational, like
//! `peer_blob_inventory`). The Phase-3 salvage candidate pool = fresh, opted-in
//! entries; `salvage_pass` ranks that pool with the placement-strategy seam to
//! self-select the deterministic closest-N holders for an under-replicated blob.
//!
//! Identity namespace is `agent_cid` throughout — the candidate pool the XOR
//! placement metric ranks over never crosses identity namespaces.
//!
//! ## Authentication: the ad is signed; the advertised agent is bound to its signer
//!
//! On the wire an ad travels as a [`SignedSalvageAd`]: the ad body, the transport peer
//! that authored it (`author_peer`) and a [`SignatureEnvelope`] made with that peer's
//! transport key over transcript v1, exactly as a custody announcement is
//! (`p2p::custody_announce`): a dual node signs with its iroh key, the identity
//! a verified two-key transport link ties to its cross-signed libp2p PeerId; a libp2p-only
//! node with its libp2p identity key.
//!
//! ```text
//! u16be(len(domain)) ‖ b"elohim:salvage-ad:v2:" ‖ u16be(epoch=0) ‖ u16be(scheme)
//!   ‖ u16be(len(key)) ‖ key ‖ dag-cbor({seq, epoch, agent_cid, archetype, author_peer,
//!                                        spare_bytes, issued_at_ms})
//! ```
//!
//! so the claimed agent, the per-peer `seq` and the issue time are inside the
//! signature. The signature is the end-to-end authentication on BOTH planes:
//! iroh-gossip's `delivered_from` is only the neighbour.
//!
//! ## Admission ([`admit`])
//!
//! 1. At most [`SALVAGE_AD_LIMIT`] bytes, else refused before decoding.
//! 2. No envelope header (an unsigned ad, including the retired Stage-1 shape with its
//!    placeholder `signature` byte) → refused, on either plane.
//! 3. An epoch or signature scheme this build does not know is **Held**: retained
//!    opaque and bounded (`held_envelopes`), never projected.
//! 4. Full decode, then structural verification of the ad body.
//! 5. The signature verifies over the transcript under the envelope's own key.
//! 6. The author is derived from that key: `author_peer` must be one of the key's
//!    transport spellings (the libp2p PeerId it makes, or the iroh NodeId it is).
//! 7. The advertised `agent_cid` must be the agent the cross-signed chain (`p2p::transport_authority`) binds to
//!    the signing key's transport identity. An unbound signer is refused.
//! 8. On libp2p the gossipsub `message.source` must not contradict the ad: a source
//!    this node binds to a DIFFERENT agent than the signer's refuses; an unbound or
//!    absent source does not — the signature and the binding are the authority.
//! 9. Freshness: `issued_at_ms` is no older than [`SALVAGE_AD_MAX_AGE_MS`] and no more
//!    than [`SALVAGE_AD_MAX_SKEW_MS`] ahead of the receiver's clock, and the boot epoch
//!    in `seq`'s high bits is not later than `issued_at_ms` (a seq cannot be minted to
//!    run ahead of every honest one).
//!
//! Replay is judged at projection: an admitted ad applies only when its `seq` is
//! strictly above the `seq` already projected for that agent
//! (`db::salvage_capacity::apply_capacity`), so a replayed or reordered older ad
//! changes nothing. `seq` carries the publisher's boot epoch in its high 32 bits
//! (`p2p::sync_state`), so a restarted publisher is strictly ahead of its old run.

use elohim_epr::proof::AgentKeypair;
use elohim_epr::signature::{SignatureCheck, SignatureEnvelope, SignatureScheme};
use serde::{Deserialize, Serialize};

use crate::p2p::custody_announce::{transport_ids_of_key, BindingError};
use crate::p2p::inventory_gossip::AuthorWitness;

/// Gossipsub topic for salvage-capacity advertisements.
pub const SALVAGE_CAPACITY_TOPIC: &str = "elohim/storage/salvage";

/// Signature domain of a salvage-capacity ad.
pub const SALVAGE_AD_DOMAIN: &[u8] = b"elohim:salvage-ad:v2:";

/// The ad epoch this build reads and writes.
pub const SALVAGE_AD_EPOCH: u16 = 0;

/// Largest encoded ad accepted (iroh-gossip caps a message at 4096 bytes).
pub const SALVAGE_AD_LIMIT: usize = 4096;

/// Oldest `issued_at_ms` admitted, relative to the receiver's clock. An ad older than
/// this is stale evidence of spare capacity and is refused, so a captured ad replayed
/// to a receiver that never projected its agent cannot place a ghost in the pool.
pub const SALVAGE_AD_MAX_AGE_MS: i64 = 60 * 60 * 1000;

/// How far ahead of the receiver's clock an `issued_at_ms` may be (clock skew).
pub const SALVAGE_AD_MAX_SKEW_MS: i64 = 5 * 60 * 1000;

/// An opt-in peer's advertisement of spare salvage capacity. Class C
/// operational; gossiped like inventory. Populates the candidate pool. This is the
/// signed body; on the wire it travels inside a [`SignedSalvageAd`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalvageCapacityAd {
    /// Canonical agent identity (`uhCAk…`) of the advertising peer. The pool is
    /// `agent_cid`-keyed so the XOR placement metric never crosses namespaces.
    pub agent_cid: String,
    /// Best-effort spare bytes this peer offers for salvage custody.
    pub spare_bytes: u64,
    /// Always-on class (`node` / `steward`). Only always-on archetypes advertise
    /// by default; carried so diversity/standing strategies (P3-8) can read it.
    pub archetype: String,
    /// Per-peer monotonic counter (boot epoch in the high 32 bits): replay ordering.
    pub seq: u64,
    /// When the ad was issued (epoch milliseconds): freshness.
    pub issued_at_ms: i64,
}

/// Reasons a salvage-capacity ad body can fail structural verification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    #[error("agent_cid cannot be empty")]
    EmptyAgentCid,
    #[error("archetype cannot be empty")]
    EmptyArchetype,
}

impl SalvageCapacityAd {
    /// Structural verification: rejects an empty `agent_cid` (would corrupt the
    /// candidate pool / XOR metric) and an empty archetype.
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.agent_cid.is_empty() {
            return Err(VerifyError::EmptyAgentCid);
        }
        if self.archetype.is_empty() {
            return Err(VerifyError::EmptyArchetype);
        }
        Ok(())
    }
}

/// A salvage-capacity ad as it travels on [`SALVAGE_CAPACITY_TOPIC`]: the ad, the
/// transport peer that authored it, and that peer's signature over both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedSalvageAd {
    /// Signature by the author's transport key over [`Self::signed_body`].
    pub sig: SignatureEnvelope,
    /// The ad.
    pub body: SalvageCapacityAd,
    /// Envelope epoch ([`SALVAGE_AD_EPOCH`]); also framed into the transcript.
    pub epoch: u16,
    /// The transport peer (libp2p PeerId or iroh NodeId text) that authored it — one
    /// of the signing key's own spellings.
    pub author_peer: String,
}

/// The signed portion, in dag-cbor canonical key order (length, then bytes): every ad
/// field, the author and the epoch.
#[derive(Serialize)]
struct SignedSalvage<'a> {
    seq: u64,
    epoch: u16,
    agent_cid: &'a str,
    archetype: &'a str,
    author_peer: &'a str,
    spare_bytes: u64,
    issued_at_ms: i64,
}

/// Just enough of an ad to decide whether this build can read the rest.
#[derive(Deserialize)]
struct Header {
    epoch: u16,
    sig: SchemeHeader,
}

#[derive(Deserialize)]
struct SchemeHeader {
    scheme: u16,
}

impl SignedSalvageAd {
    /// The bytes the signature covers (after the transcript's domain/epoch/scheme/key
    /// prefix).
    pub fn signed_body(body: &SalvageCapacityAd, epoch: u16, author_peer: &str) -> Vec<u8> {
        let signed = SignedSalvage {
            seq: body.seq,
            epoch,
            agent_cid: &body.agent_cid,
            archetype: &body.archetype,
            author_peer,
            spare_bytes: body.spare_bytes,
            issued_at_ms: body.issued_at_ms,
        };
        // Plain data (strings and integers): encoding cannot fail.
        serde_ipld_dagcbor::to_vec(&signed).unwrap_or_default()
    }

    /// Sign `body` as `author_peer` with the author's transport `keypair`.
    pub fn sign(keypair: &AgentKeypair, author_peer: &str, body: SalvageCapacityAd) -> Self {
        let signed = Self::signed_body(&body, SALVAGE_AD_EPOCH, author_peer);
        let sig =
            SignatureEnvelope::sign_ed25519(keypair, SALVAGE_AD_DOMAIN, SALVAGE_AD_EPOCH, &signed);
        Self {
            sig,
            body,
            epoch: SALVAGE_AD_EPOCH,
            author_peer: author_peer.to_string(),
        }
    }

    /// Encode to MessagePack bytes (map-keyed).
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Decode from MessagePack bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// The signature check under the envelope's own key.
    pub fn check(&self) -> SignatureCheck {
        let signed = Self::signed_body(&self.body, self.epoch, &self.author_peer);
        self.sig.check(SALVAGE_AD_DOMAIN, self.epoch, &signed)
    }
}

/// Why an ad was held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalvageHeldReason {
    Epoch(u16),
    Scheme(u16),
}

impl SalvageHeldReason {
    /// The `held_envelopes.reason` spelling.
    pub fn as_reason(&self) -> String {
        match self {
            Self::Epoch(epoch) => format!("epoch:{epoch}"),
            Self::Scheme(scheme) => format!("scheme:{scheme}"),
        }
    }
}

/// Why an ad was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SalvageRefusal {
    #[error("ad is {0} bytes; at most {SALVAGE_AD_LIMIT}")]
    Oversize(usize),
    #[error("ad carries no signature envelope: {0}")]
    Unsigned(String),
    #[error("signed ad does not decode: {0}")]
    Malformed(String),
    #[error("ad body failed structural verification: {0}")]
    Structural(VerifyError),
    #[error("signature does not verify")]
    BadSignature,
    #[error("author_peer {0} is not a transport identity of the signing key")]
    AuthorNotSigner(String),
    #[error(
        "gossipsub author {source_peer:?} is bound to another agent than author_peer {author_peer}"
    )]
    AuthorMismatch {
        source_peer: Option<String>,
        author_peer: String,
    },
    #[error(transparent)]
    Binding(BindingError),
    #[error("ad issued at {issued_at_ms} ms is older than the freshness window")]
    Stale { issued_at_ms: i64 },
    #[error("ad issued at {issued_at_ms} ms is ahead of this clock beyond the skew bound")]
    FromTheFuture { issued_at_ms: i64 },
    #[error("seq {seq} claims a boot epoch after the ad's own issue time {issued_at_ms} ms")]
    SeqAheadOfIssue { seq: u64, issued_at_ms: i64 },
}

impl SalvageRefusal {
    /// A short, stable label for logs and counters.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Structural(_) => "refused_structural",
            Self::AuthorMismatch { .. } | Self::Binding(_) => "refused_binding",
            Self::Stale { .. } | Self::FromTheFuture { .. } | Self::SeqAheadOfIssue { .. } => {
                "refused_freshness"
            }
            Self::Oversize(_)
            | Self::Unsigned(_)
            | Self::Malformed(_)
            | Self::BadSignature
            | Self::AuthorNotSigner(_) => "refused_signature",
        }
    }
}

/// The admission verdict for one received ad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SalvageAdmission {
    /// Signed by `author_peer`'s key, whose bound agent is the advertised agent, and
    /// fresh. Replay is judged at projection by `seq`.
    Admitted {
        body: SalvageCapacityAd,
        author_peer: String,
    },
    /// An epoch or scheme this build cannot read; keep the bytes opaque, bounded.
    Held {
        reason: SalvageHeldReason,
        bytes: Vec<u8>,
    },
    Refused(SalvageRefusal),
}

/// Judge one received ad (see the module docs for the order of checks).
/// `bound_agent` answers the agent the cross-signed chain (`p2p::transport_authority`) binds to a transport id
/// (the signer's binding); `source_agent` answers the agent any binding this node
/// holds attributes to a relaying gossipsub source (the same chain).
/// `now_ms` is the receiver's clock (epoch milliseconds).
pub fn admit(
    raw: &[u8],
    witness: AuthorWitness<'_>,
    bound_agent: &mut dyn FnMut(&str) -> Option<String>,
    source_agent: &mut dyn FnMut(&str) -> Option<String>,
    now_ms: i64,
) -> SalvageAdmission {
    if raw.len() > SALVAGE_AD_LIMIT {
        return SalvageAdmission::Refused(SalvageRefusal::Oversize(raw.len()));
    }
    let header: Header = match rmp_serde::from_slice(raw) {
        Ok(header) => header,
        Err(error) => {
            return SalvageAdmission::Refused(SalvageRefusal::Unsigned(error.to_string()))
        }
    };
    let held = |reason| SalvageAdmission::Held {
        reason,
        bytes: raw.to_vec(),
    };
    if header.epoch != SALVAGE_AD_EPOCH {
        return held(SalvageHeldReason::Epoch(header.epoch));
    }
    if SignatureScheme::from_code(header.sig.scheme).is_none() {
        return held(SalvageHeldReason::Scheme(header.sig.scheme));
    }
    let signed = match SignedSalvageAd::from_bytes(raw) {
        Ok(signed) => signed,
        Err(error) => {
            return SalvageAdmission::Refused(SalvageRefusal::Malformed(error.to_string()))
        }
    };
    if let Err(error) = signed.body.verify_structural() {
        return SalvageAdmission::Refused(SalvageRefusal::Structural(error));
    }
    match signed.check() {
        SignatureCheck::Verified => {}
        SignatureCheck::Invalid => return SalvageAdmission::Refused(SalvageRefusal::BadSignature),
        SignatureCheck::Held { scheme } => return held(SalvageHeldReason::Scheme(scheme)),
    }
    // The author is the signing key: `author_peer` must be one of its spellings.
    let Ok(key) = signed.sig.multikey() else {
        return SalvageAdmission::Refused(SalvageRefusal::BadSignature);
    };
    let ids = transport_ids_of_key(&key);
    if !ids.iter().any(|id| id == &signed.author_peer) {
        return SalvageAdmission::Refused(SalvageRefusal::AuthorNotSigner(signed.author_peer));
    }
    // Freshness and seq sanity are judged BEFORE the binding, so a stale or seq-ahead
    // ad is refused outright rather than held as unbound for later replay.
    let issued_at_ms = signed.body.issued_at_ms;
    if issued_at_ms < now_ms.saturating_sub(SALVAGE_AD_MAX_AGE_MS) {
        return SalvageAdmission::Refused(SalvageRefusal::Stale { issued_at_ms });
    }
    if issued_at_ms > now_ms.saturating_add(SALVAGE_AD_MAX_SKEW_MS) {
        return SalvageAdmission::Refused(SalvageRefusal::FromTheFuture { issued_at_ms });
    }
    // The seq carries the advertiser's boot epoch in its high 32 bits
    // (`p2p::sync_state`). A process cannot have booted after it signed, so an epoch
    // past the ad's own issue time is a seq minted to run ahead of every honest one
    // (a single such ad would freeze the agent's row: replay is judged by seq).
    let seq = signed.body.seq;
    let issued_epoch =
        crate::p2p::sync_state::boot_epoch(u64::try_from(issued_at_ms.max(0) / 1000).unwrap_or(0));
    if crate::p2p::sync_state::split_sequence(seq).0 > issued_epoch {
        return SalvageAdmission::Refused(SalvageRefusal::SeqAheadOfIssue { seq, issued_at_ms });
    }
    // The advertised agent is the agent the cross-signed chain binds to the signer's transport
    // identity (the claimed spelling first, then the key's other spelling).
    let bound = std::iter::once(signed.author_peer.as_str())
        .chain(ids.iter().map(String::as_str))
        .find_map(&mut *bound_agent);
    match bound.as_deref() {
        None => {
            return SalvageAdmission::Refused(SalvageRefusal::Binding(BindingError::UnboundAuthor))
        }
        Some(agent) if agent != signed.body.agent_cid => {
            return SalvageAdmission::Refused(SalvageRefusal::Binding(
                BindingError::HolderMismatch {
                    holder: signed.body.agent_cid,
                    bound: agent.to_string(),
                },
            ))
        }
        Some(_) => {}
    }
    // On libp2p the gossipsub author is a relay identity: it need not match the
    // signer, but it must not contradict the ad — a source this node binds to a
    // DIFFERENT agent than the signer's refuses. An unbound or absent source does not.
    if let AuthorWitness::Gossipsub(Some(source)) = witness {
        if source != signed.author_peer {
            if let Some(source_agent) = source_agent(source) {
                if bound.as_deref() != Some(source_agent.as_str()) {
                    return SalvageAdmission::Refused(SalvageRefusal::AuthorMismatch {
                        source_peer: Some(source.to_string()),
                        author_peer: signed.author_peer,
                    });
                }
            }
        }
    }
    SalvageAdmission::Admitted {
        body: signed.body,
        author_peer: signed.author_peer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::signature::Multikey;

    const SECRET: [u8; 32] = [31u8; 32];
    const NOW_MS: i64 = 1_800_000_000_000;

    fn signer() -> (AgentKeypair, String) {
        let peer = libp2p::identity::Keypair::ed25519_from_bytes(SECRET)
            .unwrap()
            .public()
            .to_peer_id()
            .to_string();
        (AgentKeypair::from_secret(&SECRET).unwrap(), peer)
    }

    fn sample(agent: &str) -> SalvageCapacityAd {
        SalvageCapacityAd {
            agent_cid: agent.to_string(),
            spare_bytes: 8_192,
            archetype: "node".to_string(),
            seq: 7,
            issued_at_ms: NOW_MS,
        }
    }

    fn signed(agent: &str) -> Vec<u8> {
        let (keypair, peer) = signer();
        SignedSalvageAd::sign(&keypair, &peer, sample(agent))
            .to_bytes()
            .unwrap()
    }

    /// Binds every transport spelling of `SECRET`'s key to `agent`.
    fn bound_to(agent: &'static str) -> impl FnMut(&str) -> Option<String> {
        let ids = transport_ids_of_key(&Multikey::ed25519(
            &AgentKeypair::from_secret(&SECRET)
                .unwrap()
                .public_key_bytes(),
        ));
        move |id: &str| ids.iter().any(|i| i == id).then(|| agent.to_string())
    }

    fn nobody(_: &str) -> Option<String> {
        None
    }

    fn judge(raw: &[u8], bound: &mut dyn FnMut(&str) -> Option<String>) -> SalvageAdmission {
        admit(raw, AuthorWitness::EndToEnd, bound, &mut nobody, NOW_MS)
    }

    #[test]
    fn a_bound_signers_ad_is_admitted_and_roundtrips() {
        let raw = signed("uhCAkSalvager");
        let decoded = SignedSalvageAd::from_bytes(&raw).unwrap();
        assert_eq!(decoded.body, sample("uhCAkSalvager"));
        assert!(matches!(
            judge(&raw, &mut bound_to("uhCAkSalvager")),
            SalvageAdmission::Admitted { .. }
        ));
    }

    #[test]
    fn every_signed_field_is_covered() {
        let (keypair, peer) = signer();
        let honest = SignedSalvageAd::sign(&keypair, &peer, sample("uhCAkSalvager"));
        for tamper in [
            |ad: &mut SignedSalvageAd| ad.body.seq += 1,
            |ad: &mut SignedSalvageAd| ad.body.spare_bytes += 1,
            |ad: &mut SignedSalvageAd| ad.body.issued_at_ms += 1,
            |ad: &mut SignedSalvageAd| ad.body.archetype = "steward".into(),
            |ad: &mut SignedSalvageAd| ad.body.agent_cid = "uhCAkOther".into(),
        ] {
            let mut forged = honest.clone();
            tamper(&mut forged);
            assert_eq!(forged.check(), SignatureCheck::Invalid);
        }
    }

    #[test]
    fn the_retired_stage1_shape_is_refused_as_unsigned() {
        #[derive(Serialize)]
        struct Stage1 {
            agent_cid: String,
            spare_bytes: u64,
            archetype: String,
            seq: u64,
            signature: Vec<u8>,
        }
        let raw = rmp_serde::to_vec_named(&Stage1 {
            agent_cid: "uhCAkSalvager".into(),
            spare_bytes: 1,
            archetype: "node".into(),
            seq: 1,
            signature: vec![0],
        })
        .unwrap();
        assert!(matches!(
            judge(&raw, &mut bound_to("uhCAkSalvager")),
            SalvageAdmission::Refused(SalvageRefusal::Unsigned(_))
        ));
    }

    #[test]
    fn an_unbound_signer_and_a_mismatched_agent_are_refused() {
        assert!(matches!(
            judge(&signed("uhCAkSalvager"), &mut nobody),
            SalvageAdmission::Refused(SalvageRefusal::Binding(BindingError::UnboundAuthor))
        ));
        assert!(matches!(
            judge(&signed("uhCAkVictim"), &mut bound_to("uhCAkSalvager")),
            SalvageAdmission::Refused(SalvageRefusal::Binding(BindingError::HolderMismatch { .. }))
        ));
    }

    #[test]
    fn freshness_bounds_both_ends() {
        let (keypair, peer) = signer();
        for (issued, stale) in [
            (NOW_MS - SALVAGE_AD_MAX_AGE_MS - 1, true),
            (NOW_MS + SALVAGE_AD_MAX_SKEW_MS + 1, false),
        ] {
            let mut body = sample("uhCAkSalvager");
            body.issued_at_ms = issued;
            let raw = SignedSalvageAd::sign(&keypair, &peer, body)
                .to_bytes()
                .unwrap();
            let verdict = judge(&raw, &mut bound_to("uhCAkSalvager"));
            if stale {
                assert!(matches!(
                    verdict,
                    SalvageAdmission::Refused(SalvageRefusal::Stale { .. })
                ));
            } else {
                assert!(matches!(
                    verdict,
                    SalvageAdmission::Refused(SalvageRefusal::FromTheFuture { .. })
                ));
            }
        }
    }

    #[test]
    fn unknown_epoch_and_scheme_are_held_before_any_other_check() {
        let (keypair, peer) = signer();
        let mut ad = SignedSalvageAd::sign(&keypair, &peer, sample("uhCAkSalvager"));
        ad.epoch = 9;
        assert!(matches!(
            judge(&ad.to_bytes().unwrap(), &mut nobody),
            SalvageAdmission::Held {
                reason: SalvageHeldReason::Epoch(9),
                ..
            }
        ));
        let mut ad = SignedSalvageAd::sign(&keypair, &peer, sample("uhCAkSalvager"));
        ad.sig.scheme = 77;
        assert!(matches!(
            judge(&ad.to_bytes().unwrap(), &mut nobody),
            SalvageAdmission::Held {
                reason: SalvageHeldReason::Scheme(77),
                ..
            }
        ));
    }

    #[test]
    fn oversize_is_refused_before_decoding() {
        let raw = vec![0u8; SALVAGE_AD_LIMIT + 1];
        assert!(matches!(
            judge(&raw, &mut nobody),
            SalvageAdmission::Refused(SalvageRefusal::Oversize(_))
        ));
    }

    #[test]
    fn seq_ahead_of_issue_time_is_refused() {
        let (keypair, peer) = signer();
        let issued_epoch = crate::p2p::sync_state::boot_epoch((NOW_MS / 1000) as u64);
        // Booted at the issue second: admitted.
        let mut honest = sample("uhCAkSalvager");
        honest.seq = crate::p2p::sync_state::epoch_base(issued_epoch) + 3;
        let raw = SignedSalvageAd::sign(&keypair, &peer, honest)
            .to_bytes()
            .unwrap();
        assert!(matches!(
            judge(&raw, &mut bound_to("uhCAkSalvager")),
            SalvageAdmission::Admitted { .. }
        ));
        // A boot epoch after the ad was signed, up to u64::MAX: refused.
        for seq in [
            crate::p2p::sync_state::epoch_base(issued_epoch + 1),
            u64::MAX,
        ] {
            let mut ahead = sample("uhCAkSalvager");
            ahead.seq = seq;
            let raw = SignedSalvageAd::sign(&keypair, &peer, ahead)
                .to_bytes()
                .unwrap();
            assert!(matches!(
                judge(&raw, &mut bound_to("uhCAkSalvager")),
                SalvageAdmission::Refused(SalvageRefusal::SeqAheadOfIssue { .. })
            ));
        }
    }
}
