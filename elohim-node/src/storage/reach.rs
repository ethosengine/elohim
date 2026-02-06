//! Reach-based access control

use serde::{Deserialize, Serialize};

/// Content reach level - determines replication and access
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reach {
    /// Only my devices (encrypted)
    Private,

    /// Named agents with explicit permission
    Invited,

    /// Family cluster members
    Local,

    /// Extended trust network
    Neighborhood,

    /// Regional community
    Municipal,

    /// Anyone willing to store
    Commons,
}

impl Reach {
    /// Can this content be served to a requester?
    pub fn can_serve(&self, requester_reach: Reach) -> bool {
        // More permissive reach can serve less permissive
        (*self as u8) >= (requester_reach as u8)
    }

    /// Should this content replicate to a peer?
    pub fn should_replicate(&self, peer_trust_level: Reach) -> bool {
        match self {
            Reach::Private => false, // Never auto-replicate
            Reach::Invited => false, // Requires explicit setup
            Reach::Local => peer_trust_level as u8 >= Reach::Local as u8,
            Reach::Neighborhood => peer_trust_level as u8 >= Reach::Neighborhood as u8,
            Reach::Municipal => peer_trust_level as u8 >= Reach::Municipal as u8,
            Reach::Commons => true, // Replicate to anyone
        }
    }
}
