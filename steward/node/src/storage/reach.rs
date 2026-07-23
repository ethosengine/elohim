//! Locality/placement-axis seed — NOT the declared-reach vocabulary.
//!
//! This module's `Reach` enum (6 values) and its `replication_policy` matrix
//! (`ReplicationAction::{FullSync, MetadataOnly, OnDemand, Skip}` over
//! content-locality × peer-trust) are the earliest seed of the
//! locality-driven placement engine sequenced *behind* the reach-vocabulary
//! reconciliation — see
//! `genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md`
//! §1 (vocabulary table) and §7 (out-of-scope: "locality-driven placement
//! engine" is explicitly deferred past the reconciliation sprint).
//!
//! Do **not** migrate this enum to schema-8 `Reach` — it answers a different
//! question ("how should this replicate, given content locality and peer
//! trust?") than declared reach ("who may see this?"). Per
//! `steward/node/CLAUDE.md` ("Stubs and not-built"), this is a recorded
//! **dormant definition site**: `replication_policy` and its
//! `ReplicationAction` variants have zero production callers (only the unit
//! tests below), and `can_serve`/`should_replicate` are `#[allow(dead_code)]`.
//! `sync/coordinator.rs:99` constructs a live `Reach::Local` value as a
//! `trust_level` default, so this enum is not fully inert — but nothing here
//! should be read as canon. When the locality/placement engine is actually
//! built, align this enum's variants to the SDK's `LocalityLevel`
//! (`elohim/sdk/storage-client-ts/src/protocol-core.model.ts`, the
//! source-of-record for the locality vocabulary) rather than re-deriving a
//! third geographic ladder.

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
    #[allow(dead_code)]
    pub fn can_serve(&self, requester_reach: Reach) -> bool {
        // More permissive reach can serve less permissive
        (*self as u8) >= (requester_reach as u8)
    }

    /// Should this content replicate to a peer?
    #[allow(dead_code)]
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

/// Replication action determined by content reach and peer trust level.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicationAction {
    /// Replicate content + blobs
    FullSync,
    /// Replicate metadata, fetch blobs on demand
    MetadataOnly,
    /// Don't proactively replicate, serve on request
    OnDemand,
    /// Don't replicate at all
    Skip,
}

/// Determine replication policy based on content reach and peer trust level.
#[allow(dead_code)]
pub fn replication_policy(content_reach: Reach, peer_trust: Reach) -> ReplicationAction {
    match (content_reach, peer_trust) {
        // Private and invited content never auto-replicates
        (Reach::Private, _) => ReplicationAction::Skip,
        (Reach::Invited, _) => ReplicationAction::Skip,

        // Local content: full sync to trusted peers
        (Reach::Local, Reach::Local | Reach::Neighborhood | Reach::Municipal | Reach::Commons) => {
            ReplicationAction::FullSync
        }
        (Reach::Local, _) => ReplicationAction::Skip,

        // Neighborhood content
        (Reach::Neighborhood, Reach::Local | Reach::Neighborhood) => ReplicationAction::FullSync,
        (Reach::Neighborhood, _) => ReplicationAction::MetadataOnly,

        // Municipal content
        (Reach::Municipal, _) => ReplicationAction::MetadataOnly,

        // Commons: replicate to everyone
        (Reach::Commons, _) => ReplicationAction::FullSync,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_never_replicates() {
        assert_eq!(
            replication_policy(Reach::Private, Reach::Commons),
            ReplicationAction::Skip
        );
        assert_eq!(
            replication_policy(Reach::Invited, Reach::Local),
            ReplicationAction::Skip
        );
    }

    #[test]
    fn test_local_syncs_to_trusted() {
        assert_eq!(
            replication_policy(Reach::Local, Reach::Local),
            ReplicationAction::FullSync
        );
        assert_eq!(
            replication_policy(Reach::Local, Reach::Neighborhood),
            ReplicationAction::FullSync
        );
        assert_eq!(
            replication_policy(Reach::Local, Reach::Private),
            ReplicationAction::Skip
        );
    }

    #[test]
    fn test_commons_syncs_to_all() {
        assert_eq!(
            replication_policy(Reach::Commons, Reach::Private),
            ReplicationAction::FullSync
        );
        assert_eq!(
            replication_policy(Reach::Commons, Reach::Commons),
            ReplicationAction::FullSync
        );
    }

    #[test]
    fn test_neighborhood_mixed() {
        assert_eq!(
            replication_policy(Reach::Neighborhood, Reach::Local),
            ReplicationAction::FullSync
        );
        assert_eq!(
            replication_policy(Reach::Neighborhood, Reach::Municipal),
            ReplicationAction::MetadataOnly
        );
    }
}
