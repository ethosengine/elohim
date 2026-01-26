//! Stream position tracking for sync

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Sync state for a peer relationship
#[derive(Debug, Clone)]
pub struct SyncState {
    /// My position in my stream (monotonically increasing)
    pub local_position: u64,

    /// Last known position of each peer
    pub peer_positions: HashMap<String, u64>,

    /// Pending events to send
    pub outbox: VecDeque<SyncEvent>,

    /// Recent events for quick replay
    pub recent_events: VecDeque<SyncEvent>,

    /// Maximum events to keep in memory
    pub max_recent: usize,
}

/// A sync event representing a document change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    /// Position in this agent's stream
    pub position: u64,

    /// Document that changed
    pub doc_id: String,

    /// Automerge change hash
    pub change_hash: String,

    /// Kind of event
    pub kind: EventKind,

    /// Timestamp (Unix millis)
    pub timestamp: u64,
}

/// Event classification for prioritization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    /// Created locally - highest priority
    Local,

    /// Just received from peer - high priority
    New,

    /// Historical catchup - normal priority
    Backfill,

    /// Reference before content (DAG gap) - resolve later
    Outlier,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            local_position: 0,
            peer_positions: HashMap::new(),
            outbox: VecDeque::new(),
            recent_events: VecDeque::new(),
            max_recent: 1000,
        }
    }

    /// Record a local change
    pub fn record_local(&mut self, doc_id: String, change_hash: String) -> SyncEvent {
        self.local_position += 1;
        let event = SyncEvent {
            position: self.local_position,
            doc_id,
            change_hash,
            kind: EventKind::Local,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        self.outbox.push_back(event.clone());
        self.add_to_recent(event.clone());
        event
    }

    /// Get events since a position for a peer
    pub fn events_since(&self, since: u64) -> Vec<SyncEvent> {
        self.recent_events
            .iter()
            .filter(|e| e.position > since)
            .cloned()
            .collect()
    }

    fn add_to_recent(&mut self, event: SyncEvent) {
        self.recent_events.push_back(event);
        while self.recent_events.len() > self.max_recent {
            self.recent_events.pop_front();
        }
    }
}

impl Default for SyncState {
    fn default() -> Self {
        Self::new()
    }
}
