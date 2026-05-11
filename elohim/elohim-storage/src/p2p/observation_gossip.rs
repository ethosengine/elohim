//! Cursor announcement wire format + topic mapping for the observation plane.
//!
//! Cursor announcements are gossiped via libp2p gossipsub when an observer's
//! iroh-blob log advances. Receivers pull the new log segment via the iroh
//! observation-log ALPN (Task 4.5) — only metadata (~200 bytes) crosses
//! gossipsub, not payloads. See spec §5.2.

use crate::p2p::topics::OBSERVATION_GOSSIP_TOPIC_PREFIX;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CursorAnnouncement {
    pub observer_cid: String,
    pub kind: String,
    pub log_cid: String,
    pub latest_offset: u64,
    pub subject_cid: Option<String>,
    pub observed_at_window: i64,
}

/// Map an observation_kind to its gossipsub topic.
/// `"<namespace>:<name>"` → `"elohim/observations/<namespace>"`.
/// Malformed kinds (no colon) fall back to using the whole string as the namespace.
pub fn observation_topic(kind: &str) -> String {
    let namespace = kind.split(':').next().unwrap_or("unknown");
    format!("{}{}", OBSERVATION_GOSSIP_TOPIC_PREFIX, namespace)
}
