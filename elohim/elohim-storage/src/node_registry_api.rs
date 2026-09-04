use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::StorageError;
use crate::hc_client_registry::HcClientRegistry;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ShardingStrategy {
    Geographic,
    TrustTier,
    FamilyCluster,
    Manual,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ShardStatus {
    Active,
    Stale,
    Failed,
    Migrating,
    Reconstructing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardAssignment {
    pub assignment_hash: Option<String>,
    pub content_hash: String,
    pub custodian_did: String,
    pub shard_index: u32,
    pub strategy: ShardingStrategy,
    pub status: ShardStatus,
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Registry-backed (Task 6, Holochain Evolution Epic MVP). `NodeRegistryApi`
/// used to own a private, unsupervised `HcClient` it connected once at
/// startup and never re-minted — a conductor restart left it dead for the
/// life of the process. It now holds only the shared
/// [`HcClientRegistry`] handle; `"node_registry"` is one of
/// [`crate::hc_client_registry::SUPERVISED_ROLES`], so connection,
/// reconnect-forever, and liveness supervision are entirely the registry's
/// job. This struct never caches a client across calls — every call
/// snapshots `registry.client("node_registry")` fresh, so a supervisor
/// re-mint after a conductor restart is picked up on the very next call.
pub struct NodeRegistryApi {
    registry: Arc<HcClientRegistry>,
}

impl NodeRegistryApi {
    /// Wrap the shared registry. Never connects or blocks — the registry
    /// owns connection lifecycle for the `"node_registry"` role.
    pub fn new(registry: Arc<HcClientRegistry>) -> Self {
        Self { registry }
    }

    pub async fn create_shard_assignment(
        &self,
        assignment: ShardAssignment,
    ) -> Result<Vec<u8>, StorageError> {
        // Snapshot fresh on every call — see the struct doc comment.
        let client = self.registry.client("node_registry").ok_or_else(|| {
            StorageError::Conductor("node_registry: conductor bridge unavailable".into())
        })?;

        // Serialize input to MessagePack format using standard holochain ExternIO pattern
        let payload = rmp_serde::to_vec(&assignment).map_err(|e| {
            StorageError::Internal(format!("Failed to serialize ShardAssignment: {}", e))
        })?;

        client
            .call_zome(
                "node_registry_coordinator",
                "create_shard_assignment",
                payload,
            )
            .await
    }
}
