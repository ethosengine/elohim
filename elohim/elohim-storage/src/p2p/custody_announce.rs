//! Custody announcement wire type + structural verification (Category C).
//!
//! Topic: `elohim/storage/custody/2` (gossipsub and iroh-gossip; a dual node fans out
//! to both).
//! Wire format: MessagePack via `rmp_serde` (map-keyed via `to_vec_named`, so
//! additive fields stay backward-compatible — an old peer decodes a new field to
//! its default, a new field a new peer emits is ignored by an old peer).
//!
//! ## Why this exists
//!
//! `shard_locations` is written only LOCALLY today: a node records its own
//! self-held shards (`services::self_stewardship::record_self_held_shard`) and
//! push-acked placements (`P2PNode::distribute_shards`, the `"announced"` status
//! write). Neither leaves the node. So doorway A can show "the Dowell household
//! holds this" while doorway B has no way to know it — two truths for one commons
//! artifact, and the resilience card reads a different network footprint through
//! each doorway.
//!
//! A [`CustodyAnnouncement`] is the small, on-change broadcast that closes that
//! gap: when a node writes a `shard_locations` row about its OWN holding (self-held
//! after placement, or after storing a pushed shard), it signs and gossips the
//! custody claim. Receivers project it as a
//! `peer-announced` row (see `db::shard_locations::apply_peer_announced`) that
//! NEVER overwrites a locally-witnessed row — a peer's claim is weaker evidence
//! than a node's own self-held / push-ack / verified record.
//!
//! ## Not a source of truth
//!
//! Custody is Category C (operational). There is NO DHT entry for who-holds-what
//! — the announcing peer's own projection IS the heal source (unlike REA/content
//! reconcile, which heals from the OWN conductor's notary view). A peer-announced
//! row is a self-asserted custody claim, kept distinguishable by its `status` so
//! future resilience folds can weight by evidence class.
//!
//! ## Authentication: the announcement is signed; the holder is bound to its signer
//!
//! On the wire a claim travels as a [`SignedCustodyAnnouncement`]: the claim body, the
//! transport peer that authored it (`author_peer`) and a [`SignatureEnvelope`] made with
//! that peer's transport key over transcript v1 (a dual node signs with its iroh key,
//! the identity `peer_transport_manifest` binds from the signed transport manifest;
//! a libp2p-only node with its libp2p identity key)
//!
//! ```text
//! u16be(len(domain)) ‖ b"elohim:custody-announce:v2:" ‖ u16be(epoch=0) ‖ u16be(scheme)
//!   ‖ u16be(len(key)) ‖ key ‖ dag-cbor({status, h_app_id, shard_cid, author_peer,
//!                                        announced_at_ms, holder_agent_cid})
//! ```
//!
//! so every consequential field is inside the signature. The signature is the
//! end-to-end authentication on BOTH planes: iroh-gossip's `delivered_from` is only
//! the neighbour, and gossipsub's `message.source` is no longer the only authority.
//!
//! ## Admission ([`admit`])
//!
//! 1. At most [`CUSTODY_ANNOUNCE_LIMIT`] bytes, else refused before decoding.
//! 2. No envelope header (an unsigned announcement) → refused, on either plane.
//! 3. An epoch or signature scheme this build does not know is **Held**: retained
//!    opaque, never projected.
//! 4. Full decode, then structural verification of the claim body.
//! 5. The signature verifies over the transcript under the envelope's own key.
//! 6. The author is derived from that key: `author_peer` must be one of the key's
//!    transport spellings (the libp2p PeerId it makes, or the iroh NodeId it is).
//! 7. C1: `holder_agent_cid` must be the agent `peer_transport_manifest` binds to the
//!    signing key's transport identity ([`CustodyAnnouncement::check_binding`]).
//! 8. On libp2p the gossipsub `message.source` is a relay identity: it must not
//!    contradict the claim. A source this node binds (manifest or identity bindings)
//!    to a DIFFERENT agent than the signer's refuses; an unbound or absent source
//!    does not — the signature and the holder binding are the authority.
//!
//! The claim body ([`CustodyAnnouncement`]) stays the unit the projection and the
//! catch-up inventory carry; only the gossip wire is signed.

use elohim_epr::address::ByteCid;
use elohim_epr::proof::AgentKeypair;
use elohim_epr::signature::{Multikey, SignatureCheck, SignatureEnvelope, SignatureScheme};
use serde::{Deserialize, Serialize};

use crate::p2p::inventory_gossip::AuthorWitness;

/// Gossipsub topic for custody announcements (v2: byte-CID shards, signed, author-bound).
pub const CUSTODY_ANNOUNCE_TOPIC: &str = "elohim/storage/custody/2";

/// Signature domain of a custody announcement.
pub const CUSTODY_ANNOUNCE_DOMAIN: &[u8] = b"elohim:custody-announce:v2:";

/// The announcement epoch this build reads and writes.
pub const CUSTODY_ANNOUNCE_EPOCH: u16 = 0;

/// Largest encoded announcement accepted (iroh-gossip caps a message at 4096 bytes).
pub const CUSTODY_ANNOUNCE_LIMIT: usize = 4096;

/// The `shard_locations` projection table for the `ProjectionInventory` catch-up
/// arm. The `ViewKind::ProjectionInventory { table }` discriminator is the seam;
/// a responder that does not yet serve this table returns an honest empty
/// inventory ("I hold nothing for a table I don't know"). See the report note on
/// wiring the `view_federation` responder for this table.
pub const PROJECTION_INVENTORY_TABLE_SHARD_LOCATIONS: &str = "shard_locations";

/// A single self-asserted custody claim: "`holder_agent_cid` holds the bytes of
/// `shard_cid` in app scope `h_app_id`, recorded with `status` at
/// `announced_at_ms`."
///
/// The storage identity is `(shard_cid, holder_agent_cid)` — the
/// `shard_locations` primary key. `content_id` is intentionally NOT carried: it
/// is derivable from `shard_cid` via `shard_manifests`, and the row keys on the
/// shard, not the content unit.
///
/// `holder_agent_cid` is an `agent_cid` (`uhCAk…`) — the same join key
/// `shard_locations.peer_id` holds (⚠ misnamed column; it stores an `agent_cid`,
/// never a libp2p/iroh transport id). The apply path re-checks `is_agent_cid`, so
/// a cross-namespace value can never land in the join-key column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustodyAnnouncement {
    /// The custody unit's byte CID (a shard, or a chunk of an unstriped payload),
    /// parsed by full decode on read.
    #[serde(with = "crate::blob_store::byte_cid_serde")]
    pub shard_cid: ByteCid,
    /// Canonical agent identity (`uhCAk…`) of the holder. The resilience join is
    /// `humans.agent_pub_key == shard_locations.peer_id`, both `agent_cid`.
    pub holder_agent_cid: String,
    /// App scope (e.g. `lamad`) — matches `shard_locations.h_app_id`.
    pub h_app_id: String,
    /// The source-honest status the announcer recorded locally: `self-held`,
    /// `announced` (push-acked), `verified`, or `seeded`. Carried through so the
    /// receiver's `peer-announced` row can preserve the announcer's evidence class
    /// for future weighting — the announcement itself is always projected as
    /// `peer-announced` regardless (a claim, not a witnessed fact).
    pub status: String,
    /// Announcement timestamp (epoch milliseconds). Drives the monotonic
    /// `upsert_if_newer` guard so a stale re-announce never regresses a fresher
    /// peer-announced row.
    pub announced_at_ms: i64,
}

/// Reasons a custody announcement can fail structural verification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    #[error("holder_agent_cid cannot be empty")]
    EmptyHolder,
    #[error("h_app_id cannot be empty")]
    EmptyAppId,
    #[error("status cannot be empty")]
    EmptyStatus,
}

/// Why an announcement is not bound to its author.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BindingError {
    #[error("no authenticated author on this plane")]
    NoAuthenticatedAuthor,
    #[error("the author is not bound to an agent in peer_transport_manifest")]
    UnboundAuthor,
    #[error("holder {holder} is not the author's bound agent {bound}")]
    HolderMismatch { holder: String, bound: String },
}

impl CustodyAnnouncement {
    /// Encode to MessagePack bytes (map-keyed; additive-field compatible).
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Decode from MessagePack bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Structural verification: rejects empty required fields (an empty holder would
    /// corrupt the projection join). The shard CID was validated at decode.
    ///
    /// The `is_agent_cid` shape check is NOT done here — it lives in the apply
    /// path, which distinguishes a malformed holder (`NotAgentCid`) from a
    /// self-claim (`DroppedSelf`) as distinct projection outcomes.
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.holder_agent_cid.is_empty() {
            return Err(VerifyError::EmptyHolder);
        }
        if self.h_app_id.is_empty() {
            return Err(VerifyError::EmptyAppId);
        }
        if self.status.is_empty() {
            return Err(VerifyError::EmptyStatus);
        }
        Ok(())
    }

    /// Bind the claim to its author: `author` is the authenticated gossip author (if
    /// the plane has one) and `bound_agent` the agent `peer_transport_manifest` binds
    /// to it. The holder must be exactly that agent.
    pub fn check_binding(
        &self,
        author: Option<&str>,
        bound_agent: Option<&str>,
    ) -> Result<(), BindingError> {
        if author.is_none() {
            return Err(BindingError::NoAuthenticatedAuthor);
        }
        let Some(bound) = bound_agent else {
            return Err(BindingError::UnboundAuthor);
        };
        if bound != self.holder_agent_cid {
            return Err(BindingError::HolderMismatch {
                holder: self.holder_agent_cid.clone(),
                bound: bound.to_string(),
            });
        }
        Ok(())
    }
}

/// A custody claim as it travels on [`CUSTODY_ANNOUNCE_TOPIC`]: the claim, the transport
/// peer that authored it, and that peer's signature over both. Wire: MessagePack
/// (map-keyed), the same framing the unsigned v2 announcement used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedCustodyAnnouncement {
    /// Signature by the author's transport key over [`Self::signed_body`].
    pub sig: SignatureEnvelope,
    /// The custody claim.
    pub body: CustodyAnnouncement,
    /// Envelope epoch ([`CUSTODY_ANNOUNCE_EPOCH`]); also framed into the transcript.
    pub epoch: u16,
    /// The transport peer (libp2p PeerId or iroh NodeId text) that authored it — one
    /// of the signing key's own spellings.
    pub author_peer: String,
}

/// The signed portion, in dag-cbor canonical key order (length, then bytes): every
/// claim field, the author and the epoch.
#[derive(Serialize)]
struct SignedCustody<'a> {
    epoch: u16,
    status: &'a str,
    h_app_id: &'a str,
    shard_cid: String,
    author_peer: &'a str,
    announced_at_ms: i64,
    holder_agent_cid: &'a str,
}

/// Just enough of an announcement to decide whether this build can read the rest.
#[derive(Deserialize)]
struct Header {
    epoch: u16,
    sig: SchemeHeader,
}

#[derive(Deserialize)]
struct SchemeHeader {
    scheme: u16,
}

impl SignedCustodyAnnouncement {
    /// The bytes the signature covers (after the transcript's domain/epoch/scheme/key
    /// prefix).
    pub fn signed_body(body: &CustodyAnnouncement, epoch: u16, author_peer: &str) -> Vec<u8> {
        let signed = SignedCustody {
            epoch,
            status: &body.status,
            h_app_id: &body.h_app_id,
            shard_cid: body.shard_cid.to_string(),
            author_peer,
            announced_at_ms: body.announced_at_ms,
            holder_agent_cid: &body.holder_agent_cid,
        };
        // Plain data (strings and an integer): encoding cannot fail.
        serde_ipld_dagcbor::to_vec(&signed).unwrap_or_default()
    }

    /// Sign `body` as `author_peer` with the author's transport `keypair`.
    pub fn sign(keypair: &AgentKeypair, author_peer: &str, body: CustodyAnnouncement) -> Self {
        let signed = Self::signed_body(&body, CUSTODY_ANNOUNCE_EPOCH, author_peer);
        let sig = SignatureEnvelope::sign_ed25519(
            keypair,
            CUSTODY_ANNOUNCE_DOMAIN,
            CUSTODY_ANNOUNCE_EPOCH,
            &signed,
        );
        Self {
            sig,
            body,
            epoch: CUSTODY_ANNOUNCE_EPOCH,
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
        self.sig.check(CUSTODY_ANNOUNCE_DOMAIN, self.epoch, &signed)
    }
}

/// The transport identities an ed25519 key spells: the libp2p PeerId it makes and the
/// iroh NodeId it is (64 lowercase hex). Empty for any other key.
pub fn transport_ids_of_key(key: &Multikey) -> Vec<String> {
    let Some(raw) = key.ed25519_key() else {
        return Vec::new();
    };
    let mut ids = Vec::with_capacity(2);
    if let Ok(public) = libp2p::identity::ed25519::PublicKey::try_from_bytes(&raw) {
        ids.push(
            libp2p::identity::PublicKey::from(public)
                .to_peer_id()
                .to_string(),
        );
    }
    ids.push(hex::encode(raw));
    ids
}

/// Why an announcement was held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustodyHeldReason {
    Epoch(u16),
    Scheme(u16),
}

impl CustodyHeldReason {
    /// The `held_envelopes.reason` spelling.
    pub fn as_reason(&self) -> String {
        match self {
            Self::Epoch(epoch) => format!("epoch:{epoch}"),
            Self::Scheme(scheme) => format!("scheme:{scheme}"),
        }
    }
}

/// Why an announcement was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CustodyRefusal {
    #[error("announcement is {0} bytes; at most {CUSTODY_ANNOUNCE_LIMIT}")]
    Oversize(usize),
    #[error("announcement carries no signature envelope: {0}")]
    Unsigned(String),
    #[error("signed announcement does not decode: {0}")]
    Malformed(String),
    #[error("claim body failed structural verification: {0}")]
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
}

impl CustodyRefusal {
    /// The `CUSTODY_ANNOUNCE_TOTAL` direction this refusal counts under.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Structural(_) => "refused_structural",
            Self::AuthorMismatch { .. } | Self::Binding(_) => "refused_binding",
            Self::Oversize(_)
            | Self::Unsigned(_)
            | Self::Malformed(_)
            | Self::BadSignature
            | Self::AuthorNotSigner(_) => "refused_signature",
        }
    }
}

/// The admission verdict for one received announcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustodyAdmission {
    /// Signed by `author_peer`'s key, whose bound agent is the holder.
    Admitted {
        body: CustodyAnnouncement,
        author_peer: String,
    },
    /// An epoch or scheme this build cannot read; keep the bytes opaque.
    Held {
        reason: CustodyHeldReason,
        bytes: Vec<u8>,
    },
    Refused(CustodyRefusal),
}

/// Judge one received announcement (see the module docs for the order of checks).
/// `bound_agent` answers the agent `peer_transport_manifest` binds to a transport id
/// (the signer's binding, C1); `source_agent` answers the agent any binding this node
/// holds attributes to a relaying gossipsub source (manifest or identity bindings).
pub fn admit(
    raw: &[u8],
    witness: AuthorWitness<'_>,
    bound_agent: &mut dyn FnMut(&str) -> Option<String>,
    source_agent: &mut dyn FnMut(&str) -> Option<String>,
) -> CustodyAdmission {
    if raw.len() > CUSTODY_ANNOUNCE_LIMIT {
        return CustodyAdmission::Refused(CustodyRefusal::Oversize(raw.len()));
    }
    let header: Header = match rmp_serde::from_slice(raw) {
        Ok(header) => header,
        Err(error) => {
            return CustodyAdmission::Refused(CustodyRefusal::Unsigned(error.to_string()))
        }
    };
    let held = |reason| CustodyAdmission::Held {
        reason,
        bytes: raw.to_vec(),
    };
    if header.epoch != CUSTODY_ANNOUNCE_EPOCH {
        return held(CustodyHeldReason::Epoch(header.epoch));
    }
    if SignatureScheme::from_code(header.sig.scheme).is_none() {
        return held(CustodyHeldReason::Scheme(header.sig.scheme));
    }
    let signed = match SignedCustodyAnnouncement::from_bytes(raw) {
        Ok(signed) => signed,
        Err(error) => {
            return CustodyAdmission::Refused(CustodyRefusal::Malformed(error.to_string()))
        }
    };
    if let Err(error) = signed.body.verify_structural() {
        return CustodyAdmission::Refused(CustodyRefusal::Structural(error));
    }
    match signed.check() {
        SignatureCheck::Verified => {}
        SignatureCheck::Invalid => return CustodyAdmission::Refused(CustodyRefusal::BadSignature),
        SignatureCheck::Held { scheme } => return held(CustodyHeldReason::Scheme(scheme)),
    }
    // The author is the signing key: `author_peer` must be one of its spellings.
    let Ok(key) = signed.sig.multikey() else {
        return CustodyAdmission::Refused(CustodyRefusal::BadSignature);
    };
    let ids = transport_ids_of_key(&key);
    if !ids.iter().any(|id| id == &signed.author_peer) {
        return CustodyAdmission::Refused(CustodyRefusal::AuthorNotSigner(signed.author_peer));
    }
    // C1: the holder is the agent the manifest binds to the signer's transport identity
    // (the claimed spelling first, then the key's other spelling).
    let bound = std::iter::once(signed.author_peer.as_str())
        .chain(ids.iter().map(String::as_str))
        .find_map(&mut *bound_agent);
    if let Err(error) = signed
        .body
        .check_binding(Some(signed.author_peer.as_str()), bound.as_deref())
    {
        return CustodyAdmission::Refused(CustodyRefusal::Binding(error));
    }
    // The signature and the holder's binding to the signing key are the authority.
    // On libp2p the gossipsub author is a relay identity: it need not match the
    // signer (a dual node signs with its iroh key and publishes under its libp2p
    // PeerId), but it must not contradict the claim — a source this node binds to a
    // DIFFERENT agent than the signer's refuses. An unbound or absent source does not.
    if let AuthorWitness::Gossipsub(Some(source)) = witness {
        if source != signed.author_peer {
            if let Some(source_agent) = source_agent(source) {
                if bound.as_deref() != Some(source_agent.as_str()) {
                    return CustodyAdmission::Refused(CustodyRefusal::AuthorMismatch {
                        source_peer: Some(source.to_string()),
                        author_peer: signed.author_peer,
                    });
                }
            }
        }
    }
    CustodyAdmission::Admitted {
        body: signed.body,
        author_peer: signed.author_peer,
    }
}

/// The holder's own claim after it stored pushed bytes it verified against their CID:
/// record the `self-held` row (only when the local pantry holds the bytes and the
/// identity is an `agent_cid`), then return the claim the holder signs and gossips.
/// The distributor never announces a remote holder's custody — under C1 only the
/// holder can author that claim.
pub fn holder_claim_after_store(
    conn: &mut diesel::SqliteConnection,
    local_store: &dyn crate::reconcile::custody::LocalBlobStore,
    shard: &ByteCid,
    self_agent_cid: &str,
    h_app_id: &str,
    announced_at_ms: i64,
) -> Result<Option<CustodyAnnouncement>, crate::error::StorageError> {
    use crate::services::self_stewardship::{record_self_held_shard, SelfHeldOutcome};
    match record_self_held_shard(conn, local_store, shard, self_agent_cid, h_app_id)? {
        SelfHeldOutcome::Recorded => Ok(Some(CustodyAnnouncement {
            shard_cid: *shard,
            holder_agent_cid: self_agent_cid.to_string(),
            h_app_id: h_app_id.to_string(),
            status: crate::services::self_stewardship::SELF_HELD_STATUS.to_string(),
            announced_at_ms,
        })),
        _ => Ok(None),
    }
}

/// Sign `body` with this node's transport key as `author_peer` and publish it on
/// [`CUSTODY_ANNOUNCE_TOPIC`] through `publisher` (the fan-out publisher in dual mode, so
/// the claim reaches both planes). Best-effort: a node without a signer announces
/// nothing. Returns whether the claim was handed to the transport.
pub fn publish_custody_claim(
    signer: Option<&AgentKeypair>,
    author_peer: &str,
    publisher: &dyn crate::services::gossip_flood::GossipPublisher,
    body: CustodyAnnouncement,
) -> bool {
    let Some(signer) = signer else {
        tracing::debug!(
            target: "elohim_storage::custody",
            "no transport signer; custody claim not announced"
        );
        return false;
    };
    let signed = SignedCustodyAnnouncement::sign(signer, author_peer, body);
    let bytes = match signed.to_bytes() {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(
                target: "elohim_storage::custody",
                %error,
                "failed to serialize custody announcement"
            );
            return false;
        }
    };
    match publisher.publish(CUSTODY_ANNOUNCE_TOPIC, bytes) {
        Ok(()) => {
            crate::metrics::inc_custody_announce("sent");
            tracing::debug!(
                target: "elohim_storage::custody",
                shard_cid = %signed.body.shard_cid,
                holder = %signed.body.holder_agent_cid,
                status = %signed.body.status,
                "custody claim signed and published"
            );
            true
        }
        Err(error) => {
            tracing::debug!(
                target: "elohim_storage::custody",
                ?error,
                "custody announcement publish failed (often: no subscribed peers yet)"
            );
            false
        }
    }
}

/// Catch-up inventory payload for the `ProjectionInventory { table:
/// "shard_locations" }` federation arm. Unlike the REA/content inventory (which
/// carries only `(id, anchor)` for discovery and heals row content from the OWN
/// conductor), a custody inventory entry IS the projectable claim — there is no
/// conductor to heal from, so each entry carries the full [`CustodyAnnouncement`]
/// and the requester applies it directly via the same never-overwrite-local rule.
///
/// `total` reports the responder's true whole-corpus row count so the requester
/// can detect truncation on the wire (same honesty contract as
/// `ProjectionInventoryPayload`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustodyInventoryPayload {
    /// Echoes the requested table (`shard_locations`).
    pub table: String,
    /// True whole-corpus row count on the responder (offset-invariant).
    pub total: usize,
    /// The custody claims this peer holds for this page.
    pub entries: Vec<CustodyAnnouncement>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CustodyAnnouncement {
        CustodyAnnouncement {
            shard_cid: elohim_epr::address::compute_bytes_cid(b"shard"),
            holder_agent_cid: "uhCAkHolder".to_string(),
            h_app_id: "lamad".to_string(),
            status: "self-held".to_string(),
            announced_at_ms: 1_700_000_000_000,
        }
    }

    const SECRET: [u8; 32] = [11u8; 32];

    fn signer() -> (AgentKeypair, String) {
        let peer = libp2p::identity::Keypair::ed25519_from_bytes(SECRET)
            .unwrap()
            .public()
            .to_peer_id()
            .to_string();
        (AgentKeypair::from_secret(&SECRET).unwrap(), peer)
    }

    fn signed_sample() -> (SignedCustodyAnnouncement, String) {
        let (keypair, peer) = signer();
        (
            SignedCustodyAnnouncement::sign(&keypair, &peer, sample()),
            peer,
        )
    }

    /// Binds every transport spelling of `SECRET`'s key to `agent`.
    fn bound_to(agent: &'static str) -> impl FnMut(&str) -> Option<String> {
        let (_, peer) = signer();
        let ids = transport_ids_of_key(&Multikey::ed25519(
            &AgentKeypair::from_secret(&SECRET)
                .unwrap()
                .public_key_bytes(),
        ));
        assert!(
            ids.contains(&peer),
            "the key's libp2p spelling is the author"
        );
        move |id: &str| {
            ids.iter()
                .any(|known| known == id)
                .then(|| agent.to_string())
        }
    }

    #[test]
    fn signed_announcement_verifies_and_a_tampered_field_does_not() {
        let (signed, _) = signed_sample();
        assert_eq!(signed.check(), SignatureCheck::Verified);
        let bytes = signed.to_bytes().unwrap();
        assert_eq!(
            SignedCustodyAnnouncement::from_bytes(&bytes).unwrap(),
            signed
        );

        let mut forged = signed.clone();
        forged.body.holder_agent_cid = "uhCAkSomeoneElse".into();
        assert_eq!(forged.check(), SignatureCheck::Invalid);
        let mut forged = signed.clone();
        forged.body.h_app_id = "elohim".into();
        assert_eq!(forged.check(), SignatureCheck::Invalid);
        let mut forged = signed;
        forged.author_peer = "12D3KooWSomebodyElse".into();
        assert_eq!(forged.check(), SignatureCheck::Invalid);
    }

    #[test]
    fn signed_announcement_bound_to_its_signer_is_admitted_end_to_end() {
        let (signed, peer) = signed_sample();
        let raw = signed.to_bytes().unwrap();
        match admit(
            &raw,
            AuthorWitness::EndToEnd,
            &mut bound_to("uhCAkHolder"),
            &mut unbound,
        ) {
            CustodyAdmission::Admitted { body, author_peer } => {
                assert_eq!(body, sample());
                assert_eq!(author_peer, peer);
            }
            other => panic!("expected admission, got {other:?}"),
        }
    }

    #[test]
    fn announcement_with_unknown_scheme_is_held() {
        let (mut signed, _) = signed_sample();
        signed.sig.scheme = 9;
        let raw = signed.to_bytes().unwrap();
        assert_eq!(
            admit(
                &raw,
                AuthorWitness::EndToEnd,
                &mut bound_to("uhCAkHolder"),
                &mut unbound
            ),
            CustodyAdmission::Held {
                reason: CustodyHeldReason::Scheme(9),
                bytes: raw.clone(),
            }
        );
        let (mut signed, _) = signed_sample();
        signed.epoch = 1;
        let raw = signed.to_bytes().unwrap();
        assert!(matches!(
            admit(
                &raw,
                AuthorWitness::EndToEnd,
                &mut bound_to("uhCAkHolder"),
                &mut unbound
            ),
            CustodyAdmission::Held {
                reason: CustodyHeldReason::Epoch(1),
                ..
            }
        ));
    }

    #[test]
    fn holder_not_bound_to_signing_key_is_refused() {
        let (signed, _) = signed_sample();
        let raw = signed.to_bytes().unwrap();
        // The signer's key is bound to a different agent than the claimed holder.
        let verdict = admit(
            &raw,
            AuthorWitness::EndToEnd,
            &mut bound_to("uhCAkSomeoneElse"),
            &mut unbound,
        );
        assert!(matches!(
            verdict,
            CustodyAdmission::Refused(CustodyRefusal::Binding(BindingError::HolderMismatch { .. }))
        ));
        // The signer's key is bound to nobody.
        let verdict = admit(&raw, AuthorWitness::EndToEnd, &mut unbound, &mut unbound);
        assert_eq!(
            verdict,
            CustodyAdmission::Refused(CustodyRefusal::Binding(BindingError::UnboundAuthor))
        );
        // A signature by one key claiming another peer as author.
        let (keypair, _) = signer();
        let other = libp2p::identity::Keypair::ed25519_from_bytes([12u8; 32])
            .unwrap()
            .public()
            .to_peer_id()
            .to_string();
        let raw = SignedCustodyAnnouncement::sign(&keypair, &other, sample())
            .to_bytes()
            .unwrap();
        assert!(matches!(
            admit(
                &raw,
                AuthorWitness::EndToEnd,
                &mut bound_to("uhCAkHolder"),
                &mut unbound
            ),
            CustodyAdmission::Refused(CustodyRefusal::AuthorNotSigner(_))
        ));
    }

    /// A source no binding attributes to anyone.
    fn unbound(_: &str) -> Option<String> {
        None
    }

    /// A dual node signs with its iroh key as its NodeId: the claim binds through the
    /// manifest's iroh column; on gossipsub the libp2p author is a relay identity that
    /// may differ from the signer, and refuses only when bound to another agent.
    #[test]
    fn an_iroh_signed_claim_binds_through_its_node_id() {
        let keypair = AgentKeypair::from_secret(&SECRET).unwrap();
        let node = hex::encode(keypair.public_key_bytes());
        let raw = SignedCustodyAnnouncement::sign(&keypair, &node, sample())
            .to_bytes()
            .unwrap();
        let bindings = [
            (node.clone(), "uhCAkHolder"),
            ("12D3KooWSameAgent".to_string(), "uhCAkHolder"),
            ("12D3KooWOtherAgent".to_string(), "uhCAkOther"),
        ];
        let lookup = |id: &str| {
            bindings
                .iter()
                .find(|(known, _)| known == id)
                .map(|(_, agent)| agent.to_string())
        };
        let (mut signer, mut source) = (lookup, lookup);
        assert!(matches!(
            admit(&raw, AuthorWitness::EndToEnd, &mut signer, &mut source),
            CustodyAdmission::Admitted { .. }
        ));
        for admitted in [Some("12D3KooWSameAgent"), Some("12D3KooWUnbound"), None] {
            let verdict = admit(
                &raw,
                AuthorWitness::Gossipsub(admitted),
                &mut signer,
                &mut source,
            );
            assert!(
                matches!(verdict, CustodyAdmission::Admitted { .. }),
                "{admitted:?}: {verdict:?}"
            );
        }
    }

    /// The ruling: an unbound relay source does not refuse a validly signed,
    /// holder-bound claim.
    #[test]
    fn libp2p_source_unbound_does_not_refuse_a_signed_bound_claim() {
        let (signed, _) = signed_sample();
        let raw = signed.to_bytes().unwrap();
        let verdict = admit(
            &raw,
            AuthorWitness::Gossipsub(Some("12D3KooWUnknownRelay")),
            &mut bound_to("uhCAkHolder"),
            &mut unbound,
        );
        assert!(
            matches!(verdict, CustodyAdmission::Admitted { .. }),
            "{verdict:?}"
        );
    }

    /// A relay source this node binds to a different agent than the signer's
    /// contradicts the claim and refuses it (`refused_binding`).
    #[test]
    fn libp2p_source_bound_to_another_agent_refuses() {
        let (signed, _) = signed_sample();
        let raw = signed.to_bytes().unwrap();
        let mut other = |id: &str| (id == "12D3KooWOtherAgent").then(|| "uhCAkOther".to_string());
        let verdict = admit(
            &raw,
            AuthorWitness::Gossipsub(Some("12D3KooWOtherAgent")),
            &mut bound_to("uhCAkHolder"),
            &mut other,
        );
        match verdict {
            CustodyAdmission::Refused(reason @ CustodyRefusal::AuthorMismatch { .. }) => {
                assert_eq!(reason.label(), "refused_binding");
            }
            other => panic!("expected AuthorMismatch, got {other:?}"),
        }
    }

    #[test]
    fn an_unsigned_announcement_is_refused_before_any_binding() {
        let raw = sample().to_bytes().unwrap();
        for witness in [
            AuthorWitness::EndToEnd,
            AuthorWitness::Gossipsub(Some("12D3KooWa")),
        ] {
            let verdict = admit(&raw, witness, &mut bound_to("uhCAkHolder"), &mut unbound);
            assert!(
                matches!(
                    verdict,
                    CustodyAdmission::Refused(CustodyRefusal::Unsigned(_))
                ),
                "{verdict:?}"
            );
            if let CustodyAdmission::Refused(reason) = verdict {
                assert_eq!(reason.label(), "refused_signature");
            }
        }
    }

    #[test]
    fn roundtrips_through_to_bytes_from_bytes() {
        let ann = sample();
        let bytes = ann.to_bytes().expect("encode");
        let decoded = CustodyAnnouncement::from_bytes(&bytes).expect("decode");
        assert_eq!(ann, decoded);
    }

    #[test]
    fn verify_structural_accepts_well_formed() {
        assert!(sample().verify_structural().is_ok());
    }

    #[test]
    fn verify_structural_rejects_each_empty_field() {
        let mut a = sample();
        a.holder_agent_cid.clear();
        assert_eq!(a.verify_structural(), Err(VerifyError::EmptyHolder));

        let mut a = sample();
        a.h_app_id.clear();
        assert_eq!(a.verify_structural(), Err(VerifyError::EmptyAppId));

        let mut a = sample();
        a.status.clear();
        assert_eq!(a.verify_structural(), Err(VerifyError::EmptyStatus));
    }

    #[test]
    fn a_sha256_shard_spelling_does_not_decode() {
        #[derive(Serialize)]
        struct Legacy {
            shard_cid: String,
            holder_agent_cid: String,
            h_app_id: String,
            status: String,
            announced_at_ms: i64,
        }
        let bytes = rmp_serde::to_vec_named(&Legacy {
            shard_cid: format!("sha256-{}", "a".repeat(64)),
            holder_agent_cid: "uhCAkHolder".into(),
            h_app_id: "lamad".into(),
            status: "self-held".into(),
            announced_at_ms: 1,
        })
        .unwrap();
        assert!(CustodyAnnouncement::from_bytes(&bytes).is_err());
    }

    #[test]
    fn holder_must_be_the_authors_bound_agent() {
        let ann = sample();
        assert_eq!(
            ann.check_binding(Some("12D3KooWa"), Some("uhCAkHolder")),
            Ok(())
        );
        assert_eq!(
            ann.check_binding(None, Some("uhCAkHolder")),
            Err(BindingError::NoAuthenticatedAuthor)
        );
        assert_eq!(
            ann.check_binding(Some("12D3KooWa"), None),
            Err(BindingError::UnboundAuthor)
        );
        assert!(matches!(
            ann.check_binding(Some("12D3KooWa"), Some("uhCAkSomeoneElse")),
            Err(BindingError::HolderMismatch { .. })
        ));
    }

    /// Additive-compat: a map-keyed encoding of the CURRENT struct decodes on a
    /// struct that has (hypothetically) gained a new optional field — proven here
    /// by decoding old-format bytes lacking a field the reader tolerates via serde
    /// defaults. We assert the named-field map shape so the additive contract holds.
    #[test]
    fn inventory_payload_roundtrips_and_reports_total() {
        let payload = CustodyInventoryPayload {
            table: PROJECTION_INVENTORY_TABLE_SHARD_LOCATIONS.to_string(),
            total: 3,
            entries: vec![sample()],
        };
        let bytes = rmp_serde::to_vec_named(&payload).expect("encode");
        let back: CustodyInventoryPayload = rmp_serde::from_slice(&bytes).expect("decode");
        assert_eq!(back.table, "shard_locations");
        assert_eq!(back.total, 3, "honest whole-corpus total survives the wire");
        assert_eq!(back.entries, payload.entries);
    }
}
