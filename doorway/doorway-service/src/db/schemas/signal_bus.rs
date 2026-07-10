//! Signal-bus message schema (shared cross-relay signal backing).
//!
//! One ephemeral SBD frame that an origin relay could not deliver locally and
//! published to the shared bus so a sibling relay holding the dest's live
//! connection can deliver it. Lives in a FIXED shared database (independent of
//! the per-doorway projection DB) so every doorway relay in a domain drains ONE
//! bus — the cross-relay-forward analog of the shared bootstrap table.
//!
//! Cat-C operational: ephemeral relay routing state, TTL-bounded (a handshake
//! frame older than the TTL is stale — WebRTC retries). NOT notarized, no
//! `dht_anchor_hash`. Keyed only by the ed25519 SIGNAL pubkey (a transport-plane
//! identity — never joined against `agent_cid`).

use bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use super::metadata::Metadata;
use crate::db::mongo::IntoIndexes;

/// Collection name for signal-bus messages (in the shared domain DB).
pub const SIGNAL_BUS_COLLECTION: &str = "signal_bus";

/// TTL for an undelivered bus frame. A WebRTC handshake frame this old is stale;
/// the peer retries, so dropping it is safe and bounds the collection's growth.
pub const SIGNAL_BUS_TTL_SECS: u64 = 60;

/// One published cross-relay signal frame.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignalBusDoc {
    /// MongoDB document ID (also the monotonic-ish ordering key).
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Standard metadata (created_at, updated_at, is_deleted).
    #[serde(default)]
    pub metadata: Metadata,

    /// Destination signal pubkey (ed25519, 32 bytes) — whom to deliver to if
    /// connected locally on the draining relay.
    #[serde(with = "serde_bytes", default)]
    pub dest: Vec<u8>,

    /// The verbatim SBD frame (sender header already applied by the origin relay).
    #[serde(with = "serde_bytes", default)]
    pub payload: Vec<u8>,

    /// The relay that published this frame — a draining relay filters out its own
    /// publishes so it never reprocesses what it emitted (loop-free).
    #[serde(default)]
    pub origin_relay: String,

    /// TTL key — publish wall-clock. The mongo TTL index drops the row
    /// `SIGNAL_BUS_TTL_SECS` after this; it is also the drain ordering/cursor key.
    #[serde(default = "DateTime::now")]
    pub created_at: DateTime,
}

impl Default for SignalBusDoc {
    fn default() -> Self {
        Self {
            id: None,
            metadata: Metadata::default(),
            dest: Vec::new(),
            payload: Vec::new(),
            origin_relay: String::new(),
            created_at: DateTime::now(),
        }
    }
}

impl IntoIndexes for SignalBusDoc {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![(
            // TTL index — bus frames auto-drop SIGNAL_BUS_TTL_SECS after publish.
            // Also the drain ordering key.
            doc! { "created_at": 1 },
            Some(
                IndexOptions::builder()
                    .expire_after(std::time::Duration::from_secs(SIGNAL_BUS_TTL_SECS))
                    .name("signal_bus_created_at_ttl".to_string())
                    .build(),
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_declares_ttl_index_on_created_at() {
        let names: Vec<_> = SignalBusDoc::into_indices()
            .into_iter()
            .filter_map(|(_, o)| o.and_then(|x| x.name))
            .collect();
        assert!(
            names.iter().any(|n| n == "signal_bus_created_at_ttl"),
            "missing TTL index, got {names:?}"
        );
    }
}
