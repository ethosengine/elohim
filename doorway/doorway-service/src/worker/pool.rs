//! In-process worker pool for request routing
//!
//! Provides the same benefits as NATS-based routing but without external dependencies:
//! - Fixed pool of conductor connections (no per-client connections)
//! - Request queuing under load
//! - No thread starvation
//!
//! Use this for single-node deployments. Use NATS for distributed multi-node setups.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Semaphore};
use tracing::{debug, error, info};

use super::conductor::{ConductorConnection, TokenMinter};
use crate::types::{DoorwayError, Result};

/// Request sent to the worker pool
struct PoolRequest {
    /// Raw Holochain MessagePack payload
    payload: Vec<u8>,
    /// Channel to send response back
    response_tx: oneshot::Sender<Result<Vec<u8>>>,
}

/// Configuration for the worker pool
pub struct PoolConfig {
    /// Number of worker tasks
    pub worker_count: usize,
    /// Conductor URL to connect to
    pub conductor_url: String,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Maximum queued requests
    pub max_queue_size: usize,
    /// App authentication token bytes for the Holochain 0.6 app interface.
    /// `None` for admin-interface pools (admin doesn't require auth);
    /// `Some(token)` for app-interface pools (zome calls), minted via the
    /// admin client's `issue_app_auth_token`.
    pub auth_token: Option<Vec<u8>>,
    /// Re-mints the app auth token when the connection loop detects unstable
    /// sessions (the stale-token signature after a conductor restart).
    /// `None` for admin-interface pools.
    pub token_minter: Option<TokenMinter>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            worker_count: 4,
            conductor_url: "ws://localhost:4444".to_string(),
            request_timeout_ms: 30000,
            max_queue_size: 1000,
            auth_token: None,
            token_minter: None,
        }
    }
}

/// Snapshot of pool metrics for reporting
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    /// Number of workers currently connected to conductor
    pub connected_workers: usize,
    /// Total number of workers
    pub total_workers: usize,
    /// Worker utilization ratio (connected / total), 0.0 - 1.0
    pub utilization: f64,
    /// Approximate queue depth (permits consumed)
    pub queue_depth: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Total requests processed successfully
    pub requests_ok: u64,
    /// Total requests that resulted in errors
    pub requests_err: u64,
    /// Error rate ratio (err / total), 0.0 - 1.0
    pub error_rate: f64,
}

/// In-process worker pool that manages conductor connections
pub struct WorkerPool {
    /// Channel to send requests to workers
    request_tx: mpsc::Sender<PoolRequest>,
    /// Semaphore to limit concurrent requests
    semaphore: Arc<Semaphore>,
    /// Request timeout
    timeout: Duration,
    /// Number of workers currently connected to conductor
    connected_workers: Arc<AtomicUsize>,
    /// Total number of workers
    worker_count: usize,
    /// Maximum queue size (for metrics)
    max_queue_size: usize,
    /// Count of successful requests
    requests_ok: Arc<AtomicU64>,
    /// Count of failed requests
    requests_err: Arc<AtomicU64>,
}

impl WorkerPool {
    /// Create and start a new worker pool
    pub async fn new(config: PoolConfig) -> Result<Self> {
        let (request_tx, request_rx) = mpsc::channel::<PoolRequest>(config.max_queue_size);
        let request_rx = Arc::new(tokio::sync::Mutex::new(request_rx));

        let semaphore = Arc::new(Semaphore::new(config.max_queue_size));
        let timeout = Duration::from_millis(config.request_timeout_ms);
        let connected_workers = Arc::new(AtomicUsize::new(0));

        info!(
            "Starting worker pool with {} workers, connecting to {}",
            config.worker_count, config.conductor_url
        );

        // Spawn worker tasks
        for i in 0..config.worker_count {
            let conductor_url = config.conductor_url.clone();
            let request_rx = Arc::clone(&request_rx);
            let timeout_ms = config.request_timeout_ms;
            let connected_workers = Arc::clone(&connected_workers);
            let auth_token = config.auth_token.clone();
            let token_minter = config.token_minter.clone();

            tokio::spawn(async move {
                worker_task(
                    i,
                    conductor_url,
                    auth_token,
                    token_minter,
                    request_rx,
                    timeout_ms,
                    connected_workers,
                )
                .await;
            });
        }

        info!("Worker pool started with {} workers", config.worker_count);

        Ok(Self {
            request_tx,
            semaphore,
            timeout,
            connected_workers,
            worker_count: config.worker_count,
            max_queue_size: config.max_queue_size,
            requests_ok: Arc::new(AtomicU64::new(0)),
            requests_err: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Send a request through the pool and wait for response
    pub async fn request(&self, payload: Vec<u8>) -> Result<Vec<u8>> {
        // Try to acquire semaphore (limits queue depth)
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| DoorwayError::Internal("Pool semaphore closed".into()))?;

        let (response_tx, response_rx) = oneshot::channel();

        let request = PoolRequest {
            payload,
            response_tx,
        };

        // Send to workers
        self.request_tx
            .send(request)
            .await
            .map_err(|_| DoorwayError::Internal("Worker pool closed".into()))?;

        // Wait for response with timeout
        let result = match tokio::time::timeout(self.timeout, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(DoorwayError::Internal("Response channel closed".into())),
            Err(_) => Err(DoorwayError::Holochain("Request timeout".into())),
        };

        // Track success/error counts
        match &result {
            Ok(_) => {
                self.requests_ok.fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => {
                self.requests_err.fetch_add(1, Ordering::Relaxed);
            }
        }

        result
    }

    /// Get current queue depth (approximate: permits consumed out of max)
    pub fn queue_depth(&self) -> usize {
        self.max_queue_size
            .saturating_sub(self.semaphore.available_permits())
    }

    /// Check if the worker pool is healthy (at least one worker connected to conductor)
    pub fn is_healthy(&self) -> bool {
        self.connected_workers.load(Ordering::Relaxed) > 0
    }

    /// Get the number of workers currently connected to conductor
    pub fn connected_count(&self) -> usize {
        self.connected_workers.load(Ordering::Relaxed)
    }

    /// Get the total number of workers
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Get a snapshot of pool metrics for reporting
    pub fn metrics(&self) -> PoolMetrics {
        let connected = self.connected_workers.load(Ordering::Relaxed);
        let ok = self.requests_ok.load(Ordering::Relaxed);
        let err = self.requests_err.load(Ordering::Relaxed);
        let total_requests = ok + err;
        let error_rate = if total_requests > 0 {
            err as f64 / total_requests as f64
        } else {
            0.0
        };
        let utilization = if self.worker_count > 0 {
            connected as f64 / self.worker_count as f64
        } else {
            0.0
        };

        PoolMetrics {
            connected_workers: connected,
            total_workers: self.worker_count,
            utilization,
            queue_depth: self.queue_depth(),
            max_queue_size: self.max_queue_size,
            requests_ok: ok,
            requests_err: err,
            error_rate,
        }
    }
}

/// How often an idle worker re-checks connectivity to keep the
/// connected-workers gauge (which drives pool health/routing) fresh.
const CONNECTIVITY_RECHECK: Duration = Duration::from_secs(1);

/// How long a disconnected worker waits before re-checking the connection.
const DISCONNECTED_POLL: Duration = Duration::from_millis(250);

/// Worker task that processes requests from the pool.
///
/// Creates its [`ConductorConnection`] exactly ONCE: the connection's internal
/// loop owns all reconnection (stability-gated exponential backoff, token
/// re-mint, shutdown-on-drop). Recreating the handle on failure — what this
/// function used to do — leaked the previous detached connection loop, and the
/// accumulated leaked loops were the 2026-06-10 conductor reconnect storm.
async fn worker_task(
    worker_id: usize,
    conductor_url: String,
    auth_token: Option<Vec<u8>>,
    token_minter: Option<TokenMinter>,
    request_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<PoolRequest>>>,
    timeout_ms: u64,
    connected_workers: Arc<AtomicUsize>,
) {
    info!(
        "Worker {} starting, connecting to {} ({})",
        worker_id,
        conductor_url,
        if auth_token.is_some() {
            "authenticated"
        } else {
            "unauthenticated"
        }
    );

    let conductor =
        ConductorConnection::spawn_with_auth_minter(&conductor_url, auth_token, token_minter);

    let mut counted_connected = false;

    loop {
        // Reconcile the connected-workers gauge with the connection's actual
        // state, and avoid draining requests into a dead connection.
        if !conductor.is_connected().await {
            if counted_connected {
                counted_connected = false;
                connected_workers.fetch_sub(1, Ordering::Relaxed);
                info!(
                    "Worker {} disconnected ({} workers now connected)",
                    worker_id,
                    connected_workers.load(Ordering::Relaxed)
                );
            }
            tokio::time::sleep(DISCONNECTED_POLL).await;
            continue;
        }
        if !counted_connected {
            counted_connected = true;
            connected_workers.fetch_add(1, Ordering::Relaxed);
            info!(
                "Worker {} connected to conductor ({} workers now connected)",
                worker_id,
                connected_workers.load(Ordering::Relaxed)
            );
        }

        // Get next request; time out periodically to refresh the gauge.
        let request = {
            let mut rx = request_rx.lock().await;
            match tokio::time::timeout(CONNECTIVITY_RECHECK, rx.recv()).await {
                Err(_) => continue,
                Ok(Some(r)) => r,
                Ok(None) => {
                    // Channel closed — pool dropped; shut down. Dropping the
                    // conductor handle stops its connection loop too.
                    if counted_connected {
                        connected_workers.fetch_sub(1, Ordering::Relaxed);
                    }
                    info!("Worker {} shutting down (channel closed)", worker_id);
                    return;
                }
            }
        };

        debug!(
            "Worker {} processing request ({} bytes)",
            worker_id,
            request.payload.len()
        );

        // Send to conductor. On failure we do NOT recreate the connection —
        // the internal loop reconnects with backoff; we just report the error.
        let result = conductor.request(request.payload, timeout_ms).await;

        match &result {
            Ok(data) => debug!("Worker {} got response ({} bytes)", worker_id, data.len()),
            Err(e) => {
                error!(
                    "Worker {} request failed: {} (connection loop will reconnect with backoff)",
                    worker_id, e
                );
            }
        }

        // Send response back
        let _ = request.response_tx.send(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PoolConfig::default();
        assert_eq!(config.worker_count, 4);
        assert_eq!(config.max_queue_size, 1000);
        assert!(
            config.auth_token.is_none(),
            "Default config has no auth token; admin pools rely on this"
        );
    }

    #[test]
    fn test_config_with_auth_token() {
        // App-interface pools must be able to carry an auth token
        let token: Vec<u8> = vec![1, 2, 3, 4];
        let config = PoolConfig {
            worker_count: 2,
            conductor_url: "ws://localhost:4445".to_string(),
            request_timeout_ms: 5000,
            max_queue_size: 100,
            auth_token: Some(token.clone()),
            token_minter: None,
        };
        assert_eq!(config.auth_token.as_deref(), Some(token.as_slice()));
    }

    #[test]
    fn test_pool_metrics_initial() {
        // Verify PoolMetrics default construction
        let metrics = PoolMetrics {
            connected_workers: 0,
            total_workers: 4,
            utilization: 0.0,
            queue_depth: 0,
            max_queue_size: 1000,
            requests_ok: 0,
            requests_err: 0,
            error_rate: 0.0,
        };
        assert_eq!(metrics.utilization, 0.0);
        assert_eq!(metrics.error_rate, 0.0);
    }
}
