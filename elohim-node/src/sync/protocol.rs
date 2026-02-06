//! Sync wire protocol

use serde::{Deserialize, Serialize};
use super::SyncEvent;

/// Messages in the sync protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Request events since position
    SyncRequest {
        since: u64,
        limit: Option<u32>,
    },

    /// Response with events
    SyncResponse {
        events: Vec<SyncEvent>,
        has_more: bool,
    },

    /// Request document changes
    DocRequest {
        doc_id: String,
        /// Heads we have (for incremental sync)
        heads: Vec<String>,
    },

    /// Response with document changes
    DocResponse {
        doc_id: String,
        /// Automerge changes we don't have
        changes: Vec<Vec<u8>>,
    },

    /// Announce new local event
    Announce {
        event: SyncEvent,
    },
}
