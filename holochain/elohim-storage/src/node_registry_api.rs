use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::error::StorageError;
use crate::hc_client::{HcClient, HcClientConfig};

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

pub struct NodeRegistryApi {
    hc_client: Arc<HcClient>,
}

impl NodeRegistryApi {
    pub async fn connect(
        admin_url: String,
        app_url: String,
        app_id: String,
    ) -> Result<Self, StorageError> {
        let config = HcClientConfig {
            admin_url,
            app_url,
            app_id,
            role: Some("node_registry".to_string()),
        };

        let mut attempt = 0;
        let max_attempts = 5;
        let mut delay = std::time::Duration::from_secs(2);

        loop {
            attempt += 1;
            match HcClient::connect(config.clone()).await {
                Ok(client) => {
                    info!("NodeRegistry API connected");
                    return Ok(Self {
                        hc_client: Arc::new(client),
                    });
                }
                Err(e) => {
                    if attempt >= max_attempts {
                        return Err(e);
                    }
                    warn!("NodeRegistry API connection failed, retrying...");
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, std::time::Duration::from_secs(30));
                }
            }
        }
    }

    pub async fn create_shard_assignment(&self, assignment: ShardAssignment) -> Result<Vec<u8>, StorageError> {
        // Serialize input to MessagePack format using standard holochain ExternIO pattern
        let payload = rmp_serde::to_vec(&assignment).map_err(|e| {
            StorageError::Internal(format!("Failed to serialize ShardAssignment: {}", e))
        })?;

        self.hc_client.call_zome(
            "node_registry_coordinator",
            "create_shard_assignment",
            payload,
        ).await
    }
}
