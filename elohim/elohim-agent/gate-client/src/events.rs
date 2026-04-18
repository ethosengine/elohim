//! RelationalImpactEvent — the closed enum of event kinds the gate recognizes.
//!
//! Per spec §1.2: "Adding a new variant is a protocol-level change. Non-listed
//! events are either exempt (private-drafting, offline, play-interior) or
//! flagged as 'must declare a variant' during code review. No silent bypass."

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// An event that may carry relational impact on peers and therefore must be
/// judged by the gate before execution.
///
/// See spec §1.2 for semantics per variant. The enum is deliberately closed;
/// extensibility via open types is a v1.1 upgrade path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RelationalImpactEvent {
    /// Publishing content to the DHT for peer consumption.
    ContentPublish {
        content_cid: String,
        declared_reach: String,
        author: String,
    },

    /// Writing an attestation (brit/rakia/mishpat attestation pattern).
    AttestationWrite {
        subject_hash: String,
        claim_kind: String,
        issuer: String,
    },

    /// Emitting an economic event (REA valueflow into shefa economy).
    EconomicEventEmit {
        event_kind: String,
        provider: String,
        receiver: String,
        quantity: String,
    },

    /// Sending a peer-to-peer message via libp2p custom protocol.
    PeerMessage {
        recipient: String,
        payload_kind: String,
    },

    /// Projecting accumulated private state to peers (e.g., sync-after-offline).
    SyncToPeers {
        manifest_cid: String,
        item_count: u32,
    },

    /// User requests advice or reflection from an elohim. The elohim witnesses
    /// the framing; this is itself a relational-impact event.
    AdviceSought {
        requester: String,
        summary_cid: String,
        topic: String,
    },

    /// An ElohimCapability invocation (wraps the existing 28-variant enum).
    CapabilityInvoke {
        capability: String,
        requester: String,
        request_id: String,
    },

    /// Explicit boundary-crossing event (e.g., publishing a private draft,
    /// sharing play-space content externally).
    PrivateToPublicCrossing {
        source_space: String,
        artifact_ref: String,
    },
}

impl RelationalImpactEvent {
    /// The event kind as a kebab-case string, for matching against gate
    /// `handlesEvents` declarations.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ContentPublish { .. } => "content-publish",
            Self::AttestationWrite { .. } => "attestation-write",
            Self::EconomicEventEmit { .. } => "economic-event-emit",
            Self::PeerMessage { .. } => "peer-message",
            Self::SyncToPeers { .. } => "sync-to-peers",
            Self::AdviceSought { .. } => "advice-sought",
            Self::CapabilityInvoke { .. } => "capability-invoke",
            Self::PrivateToPublicCrossing { .. } => "private-to-public-crossing",
        }
    }
}
