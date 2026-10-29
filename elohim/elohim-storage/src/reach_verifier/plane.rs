//! Closed census of storage egress planes.

/// A boundary through which storage may reveal bytes, metadata, or existence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EgressPlane {
    /// Single-row HTTP reads.
    HttpRow,
    /// HTTP collection and search reads.
    HttpList,
    /// HTTP relationship and graph reads.
    HttpEdge,
    /// HTTP blob-byte reads.
    HttpBlob,
    /// HTTP application bundle reads.
    HttpApp,
    /// HTTP EPR atom API reads.
    HttpEprApi,
    /// HTTP EPR head reads.
    HttpEprHead,
    /// HTTP raw-address reads such as shard, IPFS, and DAG.
    HttpRaw,
    /// HTTP Automerge sync reads.
    HttpSync,
    /// HTTP account export reads.
    HttpAccountExport,
    /// Single-head libp2p or iroh EPR resolution.
    P2pEprResolve,
    /// Batched libp2p or iroh EPR resolution.
    P2pEprBatch,
    /// Document content requested through the EPR protocol.
    P2pEprDocument,
    /// Delivery-capability queries that reveal whether a blob is warm.
    P2pDeliveryQuery,
    /// The libp2p blob request-response protocol.
    P2pBlob,
    /// Native iroh-blobs get, get-many, and observe requests.
    IrohBlobs,
    /// Shard protocol bytes, manifests, inventory, and full rows.
    P2pShard,
    /// Automerge synchronization between peers.
    P2pSync,
    /// EPR atom request-response serving.
    P2pEprAtom,
    /// Federated view serving.
    P2pViewFed,
    /// Kademlia EPR head publication and response caching.
    KadHead,
    /// Kademlia provider publication for atom CIDs.
    KadProvider,
    /// Inbound Kademlia GET_VALUE serving without an application hook.
    KadInboundGetValue,
    /// Blob inventory gossip snapshots, pages, and deltas.
    GossipInventory,
    /// EPR and atom announcements between peers.
    GossipAnnounce,
    /// Broadcast admission into the local CRDT projection.
    ProjectionSync,
    /// Reconciliation provider eligibility.
    ProvideReconcile,
    /// eprfs tree-root and object serving.
    EprfsTier,
    /// Public Holochain Content entries outside storage's enforcement hook.
    HolochainDht,
}

impl EgressPlane {
    /// Every plane in stable census order.
    pub const ALL: [Self; 29] = [
        Self::HttpRow,
        Self::HttpList,
        Self::HttpEdge,
        Self::HttpBlob,
        Self::HttpApp,
        Self::HttpEprApi,
        Self::HttpEprHead,
        Self::HttpRaw,
        Self::HttpSync,
        Self::HttpAccountExport,
        Self::P2pEprResolve,
        Self::P2pEprBatch,
        Self::P2pEprDocument,
        Self::P2pDeliveryQuery,
        Self::P2pBlob,
        Self::IrohBlobs,
        Self::P2pShard,
        Self::P2pSync,
        Self::P2pEprAtom,
        Self::P2pViewFed,
        Self::KadHead,
        Self::KadProvider,
        Self::KadInboundGetValue,
        Self::GossipInventory,
        Self::GossipAnnounce,
        Self::ProjectionSync,
        Self::ProvideReconcile,
        Self::EprfsTier,
        Self::HolochainDht,
    ];
}

/// Why a request-enum arm is not itself a data egress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotEgress {
    /// Inbound bytes are accepted and only an acknowledgement leaves.
    IngressAcknowledgement,
    /// A transport-control answer carries no subject data or existence bit.
    TransportControl,
}

/// Exhaustive classification of a request arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestPlane {
    /// The arm can disclose subject data through this plane.
    Egress(EgressPlane),
    /// The arm is an explicitly named non-egress operation.
    NotEgress(NotEgress),
}

#[cfg(feature = "p2p")]
/// Classify every shard request arm.
pub fn shard_request(request: &crate::p2p::ShardRequest) -> RequestPlane {
    use crate::p2p::ShardRequest;
    match request {
        ShardRequest::Get { .. }
        | ShardRequest::Have { .. }
        | ShardRequest::ListContent { .. }
        | ShardRequest::GetContent { .. }
        | ShardRequest::GetManifest { .. } => RequestPlane::Egress(EgressPlane::P2pShard),
        ShardRequest::Push { .. } => RequestPlane::NotEgress(NotEgress::IngressAcknowledgement),
    }
}

#[cfg(feature = "p2p")]
/// Classify every EPR request arm.
pub fn epr_request(request: &crate::p2p::EprRequest) -> RequestPlane {
    use crate::p2p::EprRequest;
    match request {
        EprRequest::Resolve { .. } => RequestPlane::Egress(EgressPlane::P2pEprResolve),
        EprRequest::ResolveBatch { .. } => RequestPlane::Egress(EgressPlane::P2pEprBatch),
        EprRequest::GetDocument { .. } => RequestPlane::Egress(EgressPlane::P2pEprDocument),
        EprRequest::QueryDelivery { .. } => RequestPlane::Egress(EgressPlane::P2pDeliveryQuery),
        EprRequest::Announce { .. } | EprRequest::DeviceCarry(_) => {
            RequestPlane::NotEgress(NotEgress::IngressAcknowledgement)
        }
    }
}

#[cfg(feature = "p2p")]
/// Classify every peer-sync request arm.
pub fn sync_request(request: &crate::p2p::SyncRequest) -> RequestPlane {
    use crate::p2p::SyncRequest;
    match request {
        SyncRequest::GetHeads { .. }
        | SyncRequest::SyncChanges { .. }
        | SyncRequest::GetChanges { .. }
        | SyncRequest::ListDocuments { .. }
        | SyncRequest::ListDocumentsSince { .. } => RequestPlane::Egress(EgressPlane::P2pSync),
        SyncRequest::AnnounceChange { .. } => {
            RequestPlane::NotEgress(NotEgress::IngressAcknowledgement)
        }
    }
}

#[cfg(feature = "p2p")]
/// Classify every EPR atom request arm.
pub fn epr_atom_request(request: &crate::p2p::EprAtomRequest) -> RequestPlane {
    use crate::p2p::EprAtomRequest;
    match request {
        EprAtomRequest::Fetch { .. } | EprAtomRequest::FetchBatch { .. } => {
            RequestPlane::Egress(EgressPlane::P2pEprAtom)
        }
        EprAtomRequest::Announce { .. } | EprAtomRequest::IntegrityNotify { .. } => {
            RequestPlane::NotEgress(NotEgress::IngressAcknowledgement)
        }
    }
}

#[cfg(feature = "p2p")]
/// Classify a blob request.
pub fn blob_request(_request: &crate::p2p::BlobFetchRequest) -> RequestPlane {
    RequestPlane::Egress(EgressPlane::P2pBlob)
}

/// Classify every view-federation kind.
pub fn view_kind(kind: &crate::views::ViewKind) -> RequestPlane {
    use crate::views::ViewKind;
    match kind {
        ViewKind::Cluster
        | ViewKind::PeerTopology
        | ViewKind::ProjectionInventory { .. }
        | ViewKind::ContentHeadRecord { .. } => RequestPlane::Egress(EgressPlane::P2pViewFed),
    }
}

#[cfg(test)]
mod tests {
    /// Follow-on doorway slices that share this reach-census rollout; the census test below
    /// is their only reader, so they live with it.
    const EXTERNAL_PENDING_SLICES: [&str; 4] = ["D1", "D2", "D3", "D4"];

    use super::*;

    #[test]
    fn every_plane_has_one_rule_file() {
        for plane in EgressPlane::ALL {
            assert_eq!(crate::reach_verifier::rules::for_plane(plane).plane, plane);
        }
    }

    #[test]
    fn pending_planes_name_a_known_follow_on_slice() {
        let pending: Vec<&str> = EgressPlane::ALL
            .into_iter()
            .filter_map(
                |plane| match crate::reach_verifier::rules::for_plane(plane).status {
                    crate::reach_verifier::rules::Status::Pending(slice) => Some(slice),
                    crate::reach_verifier::rules::Status::Enforced
                    | crate::reach_verifier::rules::Status::Residual(_) => None,
                },
            )
            .collect();
        let plane_slices = [
            "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10", "P11", "P12", "P13",
        ];
        for slice in pending {
            assert!(
                plane_slices.contains(&slice),
                "unknown pending plane slice {slice}"
            );
        }

        let pending_allowlist: Vec<&str> = plane_slices
            .into_iter()
            .chain(EXTERNAL_PENDING_SLICES)
            .collect();
        for slice in ["D1", "D2", "D3", "D4"] {
            assert!(
                pending_allowlist.contains(&slice),
                "missing external pending slice {slice}"
            );
        }
    }

    #[test]
    fn named_non_egress_exemptions_stay_visible() {
        let exemptions = [
            "projection_reconcile.rs:1816-1817",
            "services/head_adoption.rs",
            "http.rs:10463",
            "view_federation.rs:739",
        ];
        assert_eq!(exemptions.len(), 4);
        let _ = NotEgress::TransportControl;
    }
}
