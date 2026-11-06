//! Storage actions - blob management operations
//!
//! Actions for replicating, evicting, rebuilding, and rebalancing storage.

use elohim_epr::address::ByteCid;
use tracing::info;

use crate::pod::executor::ActionHandler;
use crate::pod::models::*;

pub struct StorageActionHandler;

#[async_trait::async_trait]
impl ActionHandler for StorageActionHandler {
    async fn execute(&self, action: &Action) -> ActionResult {
        match action.kind {
            ActionKind::ReplicateBlob => self.replicate_blob(action).await,
            ActionKind::EvictBlob => self.evict_blob(action).await,
            ActionKind::RebuildShard => self.rebuild_shard(action).await,
            ActionKind::RebalanceStorage => self.rebalance_storage(action).await,
            _ => ActionResult {
                success: false,
                message: "StorageActionHandler cannot handle this action".to_string(),
                duration_ms: 0,
                details: None,
            },
        }
    }

    fn can_handle(&self, kind: &ActionKind) -> bool {
        matches!(
            kind,
            ActionKind::ReplicateBlob
                | ActionKind::EvictBlob
                | ActionKind::RebuildShard
                | ActionKind::RebalanceStorage
        )
    }
}

/// Parse the action's `blob_cid` parameter at the boundary. The only admitted spelling is a
/// byte CID (`bafkr4i…`); anything else, a legacy sha256 spelling included, is refused here
/// and never reaches the rest of the handler.
fn blob_cid_param(action: &Action) -> Result<ByteCid, ActionResult> {
    let refuse = |message: String| ActionResult {
        success: false,
        message,
        duration_ms: 0,
        details: None,
    };
    let text = action
        .params
        .get("blob_cid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| refuse("Missing 'blob_cid' parameter".to_string()))?;
    ByteCid::parse(text).map_err(|error| refuse(format!("Invalid 'blob_cid' parameter: {error}")))
}

impl StorageActionHandler {
    async fn replicate_blob(&self, action: &Action) -> ActionResult {
        let blob_cid = match blob_cid_param(action) {
            Ok(cid) => cid,
            Err(refusal) => return refusal,
        };

        let target_nodes: Vec<String> = action
            .params
            .get("target_nodes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let priority = action
            .params
            .get("priority")
            .and_then(|v| v.as_str())
            .unwrap_or("normal");

        info!(
            %blob_cid,
            targets = target_nodes.len(),
            priority,
            "Blob replication requested"
        );

        // In a real implementation, this would:
        // 1. Read the blob from local storage
        // 2. Connect to target nodes via P2P
        // 3. Transfer the blob with verification
        // 4. Confirm storage on targets

        ActionResult {
            success: true,
            message: format!(
                "Blob {} replicated to {} nodes",
                blob_cid,
                target_nodes.len()
            ),
            duration_ms: 0,
            details: Some(serde_json::json!({
                "blob_cid": blob_cid.to_string(),
                "targets_requested": target_nodes.len(),
                "targets_completed": target_nodes.len(),
                "bytes_transferred": 0, // Would be actual
            })),
        }
    }

    async fn evict_blob(&self, action: &Action) -> ActionResult {
        let blob_cid = match blob_cid_param(action) {
            Ok(cid) => cid,
            Err(refusal) => return refusal,
        };

        let verify_replicas = action
            .params
            .get("verify_replicas")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let min_replicas = action
            .params
            .get("min_replicas")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        info!(
            %blob_cid,
            verify_replicas, min_replicas, "Blob eviction requested"
        );

        // In a real implementation, this would:
        // 1. If verify_replicas, check that enough replicas exist elsewhere
        // 2. If not enough replicas, fail or replicate first
        // 3. Delete local copy
        // 4. Update indexes

        ActionResult {
            success: true,
            message: format!("Blob {} evicted from local storage", blob_cid),
            duration_ms: 0,
            details: Some(serde_json::json!({
                "blob_cid": blob_cid.to_string(),
                "bytes_freed": 0, // Would be actual
                "remaining_replicas": 3, // Would be actual
            })),
        }
    }

    async fn rebuild_shard(&self, action: &Action) -> ActionResult {
        let blob_cid = match blob_cid_param(action) {
            Ok(cid) => cid,
            Err(refusal) => return refusal,
        };

        let shard_index = action
            .params
            .get("shard_index")
            .and_then(|v| v.as_u64())
            .map(|i| i as usize);

        info!(
            %blob_cid,
            shard_index = ?shard_index,
            "Shard rebuild requested"
        );

        // In a real implementation, this would:
        // 1. Fetch available shards from local and peer nodes
        // 2. Use Reed-Solomon to reconstruct missing shards
        // 3. Store reconstructed shards locally
        // 4. Optionally distribute to maintain redundancy

        ActionResult {
            success: true,
            message: format!(
                "Shard{} for blob {} rebuilt",
                shard_index.map(|i| format!(" {}", i)).unwrap_or_default(),
                blob_cid
            ),
            duration_ms: 0,
            details: Some(serde_json::json!({
                "blob_cid": blob_cid.to_string(),
                "shard_index": shard_index,
                "shards_fetched": 4, // Would be actual
                "shards_reconstructed": 1, // Would be actual
            })),
        }
    }

    async fn rebalance_storage(&self, action: &Action) -> ActionResult {
        let target_usage = action
            .params
            .get("target_usage_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(75.0);

        let dry_run = action
            .params
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        info!(target_usage, dry_run, "Storage rebalance requested");

        // In a real implementation, this would:
        // 1. Calculate current usage across cluster nodes
        // 2. Identify over/under-utilized nodes
        // 3. Plan blob migrations to achieve balance
        // 4. Execute migrations (respecting dry_run)

        ActionResult {
            success: true,
            message: if dry_run {
                "Storage rebalance analyzed (dry run)".to_string()
            } else {
                "Storage rebalanced across cluster".to_string()
            },
            duration_ms: 0,
            details: Some(serde_json::json!({
                "target_usage_percent": target_usage,
                "dry_run": dry_run,
                "blobs_to_move": 0, // Would be actual
                "bytes_to_move": 0, // Would be actual
                "nodes_affected": 0, // Would be actual
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::address::compute_bytes_cid;

    fn action(kind: ActionKind, params: serde_json::Value) -> Action {
        Action::new(kind, "test".to_string(), params)
    }

    #[tokio::test]
    async fn replicate_accepts_a_byte_cid_and_reports_it() {
        let cid = compute_bytes_cid(b"kernel vector").to_string();
        let result = StorageActionHandler
            .execute(&action(
                ActionKind::ReplicateBlob,
                serde_json::json!({"blob_cid": cid}),
            ))
            .await;
        assert!(result.success, "{}", result.message);
        assert_eq!(result.details.unwrap()["blob_cid"], cid);
    }

    #[tokio::test]
    async fn legacy_spellings_and_the_old_key_are_refused() {
        for params in [
            serde_json::json!({"blob_cid": "sha256-".to_string() + &"a".repeat(64)}),
            serde_json::json!({"blob_cid": "sha256:".to_string() + &"a".repeat(64)}),
            serde_json::json!({"blob_cid": elohim_epr::cid::compute_cid(b"x").to_string()}),
            serde_json::json!({"blob_hash": compute_bytes_cid(b"x").to_string()}),
        ] {
            for kind in [
                ActionKind::ReplicateBlob,
                ActionKind::EvictBlob,
                ActionKind::RebuildShard,
            ] {
                let result = StorageActionHandler
                    .execute(&action(kind, params.clone()))
                    .await;
                assert!(!result.success, "admitted {params}");
            }
        }
    }
}
