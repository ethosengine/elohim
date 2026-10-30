//! Per-plane reach policy rows.

use super::{EgressPlane, Proof};

pub(crate) mod eprfs_tier;
pub(crate) mod gossip_announce;
pub(crate) mod gossip_inventory;
pub(crate) mod holochain_dht;
pub(crate) mod http_account_export;
pub(crate) mod http_app;
pub(crate) mod http_blob;
pub(crate) mod http_edge;
pub(crate) mod http_epr_api;
pub(crate) mod http_epr_head;
pub(crate) mod http_list;
pub(crate) mod http_raw;
pub(crate) mod http_row;
pub(crate) mod http_sync;
pub(crate) mod iroh_blobs;
pub(crate) mod kad_head;
pub(crate) mod kad_inbound_get_value;
pub(crate) mod kad_provider;
pub(crate) mod p2p_blob;
pub(crate) mod p2p_delivery_query;
pub(crate) mod p2p_epr_atom;
pub(crate) mod p2p_epr_batch;
pub(crate) mod p2p_epr_document;
pub(crate) mod p2p_epr_resolve;
pub(crate) mod p2p_shard;
pub(crate) mod p2p_sync;
pub(crate) mod p2p_view_fed;
pub(crate) mod projection_sync;
pub(crate) mod provide_reconcile;

/// Coarse reach classes derived from recognized words, never an ordinal alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierClass {
    /// `commons` or `public`.
    Open,
    /// `community`.
    Community,
    /// `familiar`, `trusted`, `intimate`, or `self`.
    Bound,
    /// Exactly `private`.
    Private,
    /// Any unrecognized word.
    Unknown,
}

impl TierClass {
    /// Classify a declared reach without conflating `self` and `private`.
    pub fn classify(reach: &str) -> Self {
        match reach {
            "commons" | "public" => Self::Open,
            "community" => Self::Community,
            "familiar" | "trusted" | "intimate" | "self" => Self::Bound,
            "private" => Self::Private,
            _ => Self::Unknown,
        }
    }
}

/// The action a plane takes for one tier class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// Serve without another standing check.
    Permit,
    /// Require a boundary-presented identity claim.
    RequireIdentityPresent,
    /// Require the claim to resolve to a human row.
    RequireResolvedHuman,
    /// Run the content reach authorizer.
    Authorize,
    /// Run the custody-standing verifier.
    Custody,
    /// Serve only to the atom signer.
    AuthorOnly,
    /// Publish on this broadcast plane.
    Broadcast,
    /// Refuse on this plane.
    Refuse,
}

/// Behavior when required projection records cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordsUnavailable {
    /// Refuse until authority becomes readable.
    Refuse,
    /// Preserve the plane's declared fail-open residual.
    ServeResidual,
}

/// Behavior when no row declares the subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoRecord {
    /// Honest absence carries no audience and serves.
    Permit,
    /// Unknown authority refuses.
    Refuse,
}

/// Adoption state of one independently owned plane row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The named later slice owns enforcement.
    Pending(&'static str),
    /// Every serving call for the plane uses this row.
    Enforced,
    /// The plane has no enforceable storage hook.
    Residual(&'static str),
}

/// One plane's complete rule row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSet {
    /// The plane this file owns.
    pub plane: EgressPlane,
    /// Rule for open reaches.
    pub open: Rule,
    /// Rule for community reach.
    pub community: Rule,
    /// Rule for bound reaches.
    pub bound: Rule,
    /// Rule for private reach.
    pub private: Rule,
    /// Rule for unrecognized reaches.
    pub unknown: Rule,
    /// Minimum binding evidence for standing-bearing rules.
    pub min_proof: Proof,
    /// Behavior when no local record declares the subject.
    pub no_record: NoRecord,
    /// Behavior when required records are unreadable.
    pub records_unavailable: RecordsUnavailable,
    /// Whether this plane is enforced, pending, or residual.
    pub status: Status,
}

impl RuleSet {
    /// The rule for a raw reach word.
    pub fn rule_for(self, reach: &str) -> Rule {
        match TierClass::classify(reach) {
            TierClass::Open => self.open,
            TierClass::Community => self.community,
            TierClass::Bound => self.bound,
            TierClass::Private => self.private,
            TierClass::Unknown => self.unknown,
        }
    }

    /// A pending plane that serves every tier as its un-gated baseline does.
    pub const fn pending_permit(plane: EgressPlane, status: Status) -> Self {
        Self {
            plane,
            open: Rule::Permit,
            community: Rule::Permit,
            bound: Rule::Permit,
            private: Rule::Permit,
            unknown: Rule::Permit,
            min_proof: Proof::SelfAssertedBinding,
            no_record: NoRecord::Permit,
            records_unavailable: RecordsUnavailable::ServeResidual,
            status,
        }
    }
}

/// Return the independently owned rule row for a plane.
pub fn for_plane(plane: EgressPlane) -> RuleSet {
    match plane {
        EgressPlane::HttpRow => http_row::RULES,
        EgressPlane::HttpList => http_list::RULES,
        EgressPlane::HttpEdge => http_edge::RULES,
        EgressPlane::HttpBlob => http_blob::RULES,
        EgressPlane::HttpApp => http_app::RULES,
        EgressPlane::HttpEprApi => http_epr_api::RULES,
        EgressPlane::HttpEprHead => http_epr_head::RULES,
        EgressPlane::HttpRaw => http_raw::RULES,
        EgressPlane::HttpSync => http_sync::RULES,
        EgressPlane::HttpAccountExport => http_account_export::RULES,
        EgressPlane::P2pEprResolve => p2p_epr_resolve::RULES,
        EgressPlane::P2pEprBatch => p2p_epr_batch::RULES,
        EgressPlane::P2pEprDocument => p2p_epr_document::RULES,
        EgressPlane::P2pDeliveryQuery => p2p_delivery_query::RULES,
        EgressPlane::P2pBlob => p2p_blob::RULES,
        EgressPlane::IrohBlobs => iroh_blobs::RULES,
        EgressPlane::P2pShard => p2p_shard::RULES,
        EgressPlane::P2pSync => p2p_sync::RULES,
        EgressPlane::P2pEprAtom => p2p_epr_atom::RULES,
        EgressPlane::P2pViewFed => p2p_view_fed::RULES,
        EgressPlane::KadHead => kad_head::RULES,
        EgressPlane::KadProvider => kad_provider::RULES,
        EgressPlane::KadInboundGetValue => kad_inbound_get_value::RULES,
        EgressPlane::GossipInventory => gossip_inventory::RULES,
        EgressPlane::GossipAnnounce => gossip_announce::RULES,
        EgressPlane::ProjectionSync => projection_sync::RULES,
        EgressPlane::ProvideReconcile => provide_reconcile::RULES,
        EgressPlane::EprfsTier => eprfs_tier::RULES,
        EgressPlane::HolochainDht => holochain_dht::RULES,
    }
}
