use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    address::{BlobCid, EprRef, ProjectionId},
    error::Result,
    projection::{ProjectionPath, ProjectionSource},
};

/// Protocol-facing summary for an EPR or projection subject.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprCard {
    pub subject: EprRef,
    pub projection: Option<ProjectionId>,
    pub title: Option<String>,
    pub source: Option<ProjectionSource>,
    pub byte_presence: BytePresence,
    pub resiliency: EprResiliency,
    pub peer_visibility: PeerVisibility,
    pub verification: VerificationStatus,
    pub local_overlay: LocalOverlayStatus,
    pub metadata: Value,
}

/// Projection-level awareness that sits beside, not inside, the static manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionAwareness {
    pub projection: ProjectionId,
    pub root: EprCard,
    pub entries: Vec<ProjectionEntryAwareness>,
    pub metadata: Value,
}

/// Entry-level awareness for filesystem status badges, sidecars, and UIs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionEntryAwareness {
    pub path: ProjectionPath,
    pub subject: Option<EprRef>,
    pub blob: Option<BlobCid>,
    pub byte_presence: BytePresence,
    pub resiliency: EprResiliency,
    pub peer_visibility: PeerVisibility,
    pub verification: VerificationStatus,
    pub local_overlay: LocalOverlayStatus,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BytePresence {
    Remote,
    Hydrating,
    Local,
    Pinned,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocalOverlayStatus {
    Clean,
    Dirty,
    Divergent,
    Conflict,
    UnsupportedByHost,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprResiliency {
    pub desired_replication: Option<u32>,
    pub verified_replicas: Option<u32>,
    pub custody_peers: Option<u32>,
    pub status: ResiliencyStatus,
}

impl EprResiliency {
    pub fn unknown() -> Self {
        Self {
            desired_replication: None,
            verified_replicas: None,
            custody_peers: None,
            status: ResiliencyStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResiliencyStatus {
    Healthy,
    Degraded,
    UnderReplicated,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerVisibility {
    pub visible_peers: Option<u32>,
    pub available_peers: Option<u32>,
    pub last_seen_unix_ms: Option<u64>,
}

impl PeerVisibility {
    pub fn unknown() -> Self {
        Self {
            visible_peers: None,
            available_peers: None,
            last_seen_unix_ms: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationStatus {
    Verified,
    Unverified,
    Failed,
    Unknown,
}

#[async_trait]
pub trait ProjectionAwarenessProvider: Send + Sync {
    async fn projection_awareness(
        &self,
        projection: &ProjectionId,
    ) -> Result<Option<ProjectionAwareness>>;

    async fn entry_awareness(
        &self,
        projection: &ProjectionId,
        path: &ProjectionPath,
    ) -> Result<Option<ProjectionEntryAwareness>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectionSourceKind;

    #[test]
    fn epr_card_serializes_protocol_awareness() {
        let card = EprCard {
            subject: EprRef::new("epr:root"),
            projection: Some(ProjectionId::new("projection:root")),
            title: Some("Root Projection".into()),
            source: Some(ProjectionSource::new(
                "git",
                ProjectionSourceKind::Container,
                "abc123",
            )),
            byte_presence: BytePresence::Local,
            resiliency: EprResiliency {
                desired_replication: Some(5),
                verified_replicas: Some(3),
                custody_peers: Some(4),
                status: ResiliencyStatus::Degraded,
            },
            peer_visibility: PeerVisibility {
                visible_peers: Some(7),
                available_peers: Some(6),
                last_seen_unix_ms: Some(42),
            },
            verification: VerificationStatus::Verified,
            local_overlay: LocalOverlayStatus::Clean,
            metadata: Value::Null,
        };

        let serialized = serde_json::to_value(&card).unwrap();

        assert_eq!(serialized["subject"], "epr:root");
        assert_eq!(serialized["bytePresence"], "local");
        assert_eq!(serialized["resiliency"]["verifiedReplicas"], 3);
        assert_eq!(serialized["localOverlay"], "clean");
    }
}
