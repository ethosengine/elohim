//! Disaster recovery coordination
//!
//! ## Overview
//!
//! Coordinates content replication when nodes fail:
//! 1. Subscribes to ReplicateContent signals from Node Registry DNA
//! 2. Finds source custodians that have the content
//! 3. Coordinates blob transfer to replacement custodian
//! 4. Emits progress updates for Shefa dashboard
//!
//! ## Recovery Flow
//!
//! ```text
//! Node fails -> DNA emits ReplicateContent signal
//!            -> Coordinator receives signal via NATS
//!            -> Finds healthy custodians with content
//!            -> Initiates blob transfer
//!            -> Updates assignment status
//!            -> Emits progress to Shefa dashboard
//! ```

use super::OrchestratorState;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Disaster recovery coordinator
pub struct DisasterRecoveryCoordinator {
    state: Arc<OrchestratorState>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl DisasterRecoveryCoordinator {
    /// Create new coordinator
    pub fn new(state: Arc<OrchestratorState>) -> Self {
        Self {
            state,
            shutdown_tx: None,
        }
    }

    /// Start the disaster recovery listener
    pub async fn start(&self) -> Result<()> {
        info!("Starting disaster recovery coordinator");

        let state = Arc::clone(&self.state);

        // Spawn NATS listener for ReplicateContent signals
        tokio::spawn(async move {
            if let Err(e) = run_recovery_listener(state).await {
                error!(error = %e, "Disaster recovery listener failed");
            }
        });

        Ok(())
    }

    /// Stop the coordinator
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping disaster recovery coordinator");
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    /// Process a replication request
    pub async fn process_replication(&self, request: ReplicateContentRequest) -> Result<()> {
        process_replication_request(request, Arc::clone(&self.state)).await
    }
}

/// ReplicateContent signal from Node Registry DNA
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicateContentRequest {
    /// Content identifier
    pub content_id: String,
    /// Content hash for verification
    pub content_hash: String,
    /// Node IDs that have the content (source custodians)
    pub from_custodians: Vec<String>,
    /// Node ID to replicate to (replacement custodian)
    pub to_custodian: String,
    /// Replication strategy: full_replica, erasure_shard, etc.
    pub strategy: String,
    /// Priority: high, normal, low
    pub priority: Option<String>,
}

/// Replication progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicationProgress {
    /// Content being replicated
    pub content_id: String,
    /// Status: pending, transferring, verifying, complete, failed
    pub status: ReplicationStatus,
    /// Bytes transferred so far
    pub bytes_transferred: u64,
    /// Total bytes to transfer
    pub total_bytes: u64,
    /// Progress percentage (0-100)
    pub percent_complete: f64,
    /// Source custodian being used
    pub source_custodian: String,
    /// Target custodian
    pub target_custodian: String,
    /// Error message if failed
    pub error: Option<String>,
}

/// Replication status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicationStatus {
    Pending,
    Transferring,
    Verifying,
    Complete,
    Failed,
}

/// Recovery summary for Shefa dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySummary {
    /// Node that failed
    pub failed_node_id: String,
    /// Total content items needing recovery
    pub total_items: u32,
    /// Items successfully recovered
    pub recovered_items: u32,
    /// Items still pending
    pub pending_items: u32,
    /// Items that failed recovery
    pub failed_items: u32,
    /// Overall recovery percentage
    pub recovery_percent: f64,
    /// Estimated time remaining (seconds)
    pub eta_seconds: Option<u64>,
}

/// Run the NATS listener for recovery signals
async fn run_recovery_listener(_state: Arc<OrchestratorState>) -> Result<()> {
    info!("Recovery listener started - waiting for ReplicateContent signals");

    // In production, subscribe to NATS subject for DNA signals:
    // let nats_client = state.nats_client.as_ref()?;
    // let mut subscription = nats_client.subscribe("SIGNAL.node_registry.replicate_content").await?;
    //
    // while let Some(msg) = subscription.next().await {
    //     let request: ReplicateContentRequest = serde_json::from_slice(&msg.payload)?;
    //     process_replication_request(request, Arc::clone(&state)).await?;
    // }

    // Keep the task alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

/// Process a replication request
async fn process_replication_request(
    request: ReplicateContentRequest,
    state: Arc<OrchestratorState>,
) -> Result<()> {
    info!(
        content_id = %request.content_id,
        from = ?request.from_custodians,
        to = %request.to_custodian,
        strategy = %request.strategy,
        "Processing replication request"
    );

    // Find a healthy source custodian
    let source = find_healthy_source(&request.from_custodians, &state).await?;

    if source.is_none() {
        error!(
            content_id = %request.content_id,
            "No healthy source custodians available"
        );
        emit_progress(ReplicationProgress {
            content_id: request.content_id.clone(),
            status: ReplicationStatus::Failed,
            bytes_transferred: 0,
            total_bytes: 0,
            percent_complete: 0.0,
            source_custodian: "none".to_string(),
            target_custodian: request.to_custodian.clone(),
            error: Some("No healthy source custodians available".to_string()),
        })
        .await?;
        return Ok(());
    }

    let source_id = source.unwrap();

    // Emit initial progress
    emit_progress(ReplicationProgress {
        content_id: request.content_id.clone(),
        status: ReplicationStatus::Pending,
        bytes_transferred: 0,
        total_bytes: 0, // Will be updated when we know the size
        percent_complete: 0.0,
        source_custodian: source_id.clone(),
        target_custodian: request.to_custodian.clone(),
        error: None,
    })
    .await?;

    // Initiate the transfer
    match initiate_blob_transfer(&request, &source_id, &state).await {
        Ok(()) => {
            emit_progress(ReplicationProgress {
                content_id: request.content_id.clone(),
                status: ReplicationStatus::Complete,
                bytes_transferred: 0, // Would be actual size
                total_bytes: 0,
                percent_complete: 100.0,
                source_custodian: source_id,
                target_custodian: request.to_custodian,
                error: None,
            })
            .await?;
        }
        Err(e) => {
            error!(
                content_id = %request.content_id,
                error = %e,
                "Replication failed"
            );
            emit_progress(ReplicationProgress {
                content_id: request.content_id,
                status: ReplicationStatus::Failed,
                bytes_transferred: 0,
                total_bytes: 0,
                percent_complete: 0.0,
                source_custodian: source_id,
                target_custodian: request.to_custodian,
                error: Some(e.to_string()),
            })
            .await?;
        }
    }

    Ok(())
}

/// Find a healthy source custodian
async fn find_healthy_source(
    candidates: &[String],
    state: &OrchestratorState,
) -> Result<Option<String>> {
    for node_id in candidates {
        if let Some(node) = state.get_node(node_id).await {
            if node.status == super::NodeHealthStatus::Online {
                return Ok(Some(node_id.clone()));
            }
        }
    }
    Ok(None)
}

/// Initiate blob transfer between custodians
async fn initiate_blob_transfer(
    request: &ReplicateContentRequest,
    source_id: &str,
    _state: &OrchestratorState,
) -> Result<()> {
    info!(
        content_id = %request.content_id,
        source = %source_id,
        target = %request.to_custodian,
        "Initiating blob transfer"
    );

    // In production:
    // 1. Send NATS message to source node requesting blob push
    // 2. Source node streams blob to target via Doorway or direct P2P
    // 3. Target node verifies hash and confirms receipt
    // 4. Update assignment in DNA

    // Example NATS message to source:
    // let msg = TransferCommand {
    //     content_id: request.content_id.clone(),
    //     content_hash: request.content_hash.clone(),
    //     target_node: request.to_custodian.clone(),
    //     strategy: request.strategy.clone(),
    // };
    // nats_client.publish(
    //     &format!("WORKLOAD.{}.transfer", source_id),
    //     &serde_json::to_vec(&msg)?
    // ).await?;

    debug!(
        content_id = %request.content_id,
        "Blob transfer initiated (simulated)"
    );

    Ok(())
}

/// Emit progress update to Shefa dashboard
async fn emit_progress(progress: ReplicationProgress) -> Result<()> {
    debug!(
        content_id = %progress.content_id,
        status = ?progress.status,
        percent = %progress.percent_complete,
        "Emitting replication progress"
    );

    // In production, publish to NATS for Shefa dashboard:
    // nats_client.publish(
    //     "RECOVERY.progress",
    //     &serde_json::to_vec(&progress)?
    // ).await?;

    Ok(())
}

/// Calculate recovery summary for a failed node
pub async fn calculate_recovery_summary(
    failed_node_id: &str,
    state: &OrchestratorState,
) -> RecoverySummary {
    // In production, query DNA for assignment status
    // let assignments = get_assignments_for_node(failed_node_id).await;
    // Count items by recovery status

    RecoverySummary {
        failed_node_id: failed_node_id.to_string(),
        total_items: 0,
        recovered_items: 0,
        pending_items: 0,
        failed_items: 0,
        recovery_percent: 0.0,
        eta_seconds: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replication_request_serialization() {
        let request = ReplicateContentRequest {
            content_id: "content-123".to_string(),
            content_hash: "sha256-abc".to_string(),
            from_custodians: vec!["node-a".to_string(), "node-b".to_string()],
            to_custodian: "node-c".to_string(),
            strategy: "full_replica".to_string(),
            priority: Some("high".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: ReplicateContentRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.content_id, "content-123");
        assert_eq!(parsed.from_custodians.len(), 2);
    }

    #[test]
    fn test_progress_serialization() {
        let progress = ReplicationProgress {
            content_id: "content-123".to_string(),
            status: ReplicationStatus::Transferring,
            bytes_transferred: 1024 * 1024,
            total_bytes: 10 * 1024 * 1024,
            percent_complete: 10.0,
            source_custodian: "node-a".to_string(),
            target_custodian: "node-c".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("transferring"));
    }

    #[tokio::test]
    async fn test_coordinator_creation() {
        let state = Arc::new(OrchestratorState::new(Default::default()));
        let _coordinator = DisasterRecoveryCoordinator::new(state);
    }
}
