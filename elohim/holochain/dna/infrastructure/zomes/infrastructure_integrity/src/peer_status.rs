use hdi::prelude::*;

/// Lifecycle state of a peer. Folds periodic status and transition
/// announcements into one enum — see spec §PeerStatus surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerLifecycleState {
    Starting,
    Online,
    Degraded,
    Maintenance,
    Leaving,
}

impl std::fmt::Display for PeerLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerLifecycleState::Starting => write!(f, "starting"),
            PeerLifecycleState::Online => write!(f, "online"),
            PeerLifecycleState::Degraded => write!(f, "degraded"),
            PeerLifecycleState::Maintenance => write!(f, "maintenance"),
            PeerLifecycleState::Leaving => write!(f, "leaving"),
        }
    }
}

/// Capability flags advertised by a peer. v1 keeps this tight; see spec
/// §Evolution for how traffic_class / current_load extend this struct
/// additively.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerCapabilityFlags {
    pub general_pool_member: bool,
    pub accepting_stewardship_reserves: bool,
}

/// A peer's self-authored availability snapshot.
///
/// Validation (added in Task 3):
/// - Author must equal `peer_id` (peers cannot author for others).
/// - Timestamp must be within 5 minutes of DHT validation time.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PeerStatus {
    pub peer_id: AgentPubKey,
    pub status: PeerLifecycleState,
    pub flags: PeerCapabilityFlags,
    pub archetype_class: Option<String>,
    pub timestamp: Timestamp,
}
