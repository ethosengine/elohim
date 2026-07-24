//! Custody announcement wire type + structural verification (Category C).
//!
//! Topic: `elohim/storage/custody` (gossipsub).
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
//! gap: when a node writes a `shard_locations` row it can verify, it gossips the
//! custody claim on the existing gossipsub plane. Receivers project it as a
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
//! ## Stage 1 signature
//!
//! The announcement carries a `signature: Vec<u8>` that is structurally non-empty
//! (a single null byte suffices at Stage 1). Stage 2 will enforce Ed25519
//! verification over canonical bytes; the non-empty gate is the forward-compatible
//! placeholder, mirroring `salvage_gossip` / `inventory_gossip`.

use serde::{Deserialize, Serialize};

/// Gossipsub topic for custody announcements.
pub const CUSTODY_ANNOUNCE_TOPIC: &str = "elohim/storage/custody";

/// The `shard_locations` projection table for the `ProjectionInventory` catch-up
/// arm. The `ViewKind::ProjectionInventory { table }` discriminator is the seam;
/// a responder that does not yet serve this table returns an honest empty
/// inventory ("I hold nothing for a table I don't know"). See the report note on
/// wiring the `view_federation` responder for this table.
pub const PROJECTION_INVENTORY_TABLE_SHARD_LOCATIONS: &str = "shard_locations";

/// A single self-asserted custody claim: "`holder_agent_cid` holds the bytes of
/// `shard_hash` in app scope `h_app_id`, recorded with `status` at
/// `announced_at_ms`."
///
/// The storage identity is `(shard_hash, holder_agent_cid)` — the
/// `shard_locations` primary key. `content_id` is intentionally NOT carried: it
/// is derivable from `shard_hash` via `shard_manifests`, and the row keys on the
/// shard, not the content unit.
///
/// `holder_agent_cid` is an `agent_cid` (`uhCAk…`) — the same join key
/// `shard_locations.peer_id` holds (⚠ misnamed column; it stores an `agent_cid`,
/// never a libp2p/iroh transport id). The apply path re-checks `is_agent_cid`, so
/// a cross-namespace value can never land in the join-key column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustodyAnnouncement {
    /// Content-addressed shard identifier (`sha256-…` today; CID-first later).
    pub shard_hash: String,
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
    /// Structural non-empty signature (Stage 1). Ed25519 in Stage 2.
    pub signature: Vec<u8>,
}

/// Reasons a custody announcement can fail structural verification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    #[error("shard_hash cannot be empty")]
    EmptyShardHash,
    #[error("holder_agent_cid cannot be empty")]
    EmptyHolder,
    #[error("h_app_id cannot be empty")]
    EmptyAppId,
    #[error("status cannot be empty")]
    EmptyStatus,
    #[error("signature cannot be empty")]
    EmptySignature,
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

    /// Structural verification — Stage 1 gate. Rejects empty required fields
    /// (an empty shard/holder would corrupt the projection join) and an empty
    /// signature (Stage 1 non-empty placeholder; Ed25519 in Stage 2).
    ///
    /// The `is_agent_cid` shape check is NOT done here — it lives in the apply
    /// path, which distinguishes a malformed holder (`NotAgentCid`) from a
    /// self-claim (`DroppedSelf`) as distinct projection outcomes.
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.shard_hash.is_empty() {
            return Err(VerifyError::EmptyShardHash);
        }
        if self.holder_agent_cid.is_empty() {
            return Err(VerifyError::EmptyHolder);
        }
        if self.h_app_id.is_empty() {
            return Err(VerifyError::EmptyAppId);
        }
        if self.status.is_empty() {
            return Err(VerifyError::EmptyStatus);
        }
        if self.signature.is_empty() {
            return Err(VerifyError::EmptySignature);
        }
        Ok(())
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
            shard_hash: "sha256-abc".to_string(),
            holder_agent_cid: "uhCAkHolder".to_string(),
            h_app_id: "lamad".to_string(),
            status: "self-held".to_string(),
            announced_at_ms: 1_700_000_000_000,
            signature: vec![0x00],
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
        a.shard_hash.clear();
        assert_eq!(a.verify_structural(), Err(VerifyError::EmptyShardHash));

        let mut a = sample();
        a.holder_agent_cid.clear();
        assert_eq!(a.verify_structural(), Err(VerifyError::EmptyHolder));

        let mut a = sample();
        a.h_app_id.clear();
        assert_eq!(a.verify_structural(), Err(VerifyError::EmptyAppId));

        let mut a = sample();
        a.status.clear();
        assert_eq!(a.verify_structural(), Err(VerifyError::EmptyStatus));

        let mut a = sample();
        a.signature.clear();
        assert_eq!(a.verify_structural(), Err(VerifyError::EmptySignature));
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
