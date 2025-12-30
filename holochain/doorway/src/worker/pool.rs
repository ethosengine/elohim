//! In-process worker pool for request routing
//!
//! Provides the same benefits as NATS-based routing but without external dependencies:
//! - Fixed pool of conductor connections (no per-client connections)
//! - Request queuing under load
//! - No thread starvation
//!
//! Use this for single-node deployments. Use NATS for distributed multi-node setups.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Semaphore};
use tracing::{debug, error, info};

use super::conductor::ConductorConnection;
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
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            worker_count: 4,
            conductor_url: "ws://localhost:4444".to_string(),
            request_timeout_ms: 30000,
            max_queue_size: 1000,
        }
    }
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

            tokio::spawn(async move {
                worker_task(i, conductor_url, request_rx, timeout_ms, connected_workers).await;
            });
        }

        info!("Worker pool started with {} workers", config.worker_count);

        Ok(Self {
            request_tx,
            semaphore,
            timeout,
            connected_workers,
            worker_count: config.worker_count,
        })
    }

    /// Send a request through the pool and wait for response
    pub async fn request(&self, payload: Vec<u8>) -> Result<Vec<u8>> {
        // Try to acquire semaphore (limits queue depth)
        let _permit = self.semaphore.clone().acquire_owned().await
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
        match tokio::time::timeout(self.timeout, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(DoorwayError::Internal("Response channel closed".into())),
            Err(_) => Err(DoorwayError::Holochain("Request timeout".into())),
        }
    }

    /// Get current queue depth (approximate)
    pub fn queue_depth(&self) -> usize {
        self.semaphore.available_permits()
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
}

/// Worker task that processes requests from the pool
async fn worker_task(
    worker_id: usize,
    conductor_url: String,
    request_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<PoolRequest>>>,
    timeout_ms: u64,
    connected_workers: Arc<AtomicUsize>,
) {
    info!("Worker {} starting, connecting to {}", worker_id, conductor_url);

    loop {
        // Connect to conductor (with reconnection logic built-in)
        let conductor = match ConductorConnection::connect(&conductor_url).await {
            Ok(c) => {
                // Increment connected counter
                connected_workers.fetch_add(1, Ordering::Relaxed);
                info!(
                    "Worker {} connected to conductor ({} workers now connected)",
                    worker_id,
                    connected_workers.load(Ordering::Relaxed)
                );
                c
            }
            Err(e) => {
                error!("Worker {} failed to connect: {}, retrying in 5s", worker_id, e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        // Process requests
        loop {
            // Get next request
            let request = {
                let mut rx = request_rx.lock().await;
                match rx.recv().await {
                    Some(r) => r,
                    None => {
                        // Channel closed, decrement and shutdown
                        connected_workers.fetch_sub(1, Ordering::Relaxed);
                        info!("Worker {} shutting down (channel closed)", worker_id);
                        return;
                    }
                }
            };

            debug!("Worker {} processing request ({} bytes)", worker_id, request.payload.len());

            // Send to conductor
            let result = conductor.request(request.payload, timeout_ms).await;

            match &result {
                Ok(data) => debug!("Worker {} got response ({} bytes)", worker_id, data.len()),
                Err(e) => {
                    error!("Worker {} request failed: {}", worker_id, e);
                    // Connection likely lost, decrement and reconnect
                    connected_workers.fetch_sub(1, Ordering::Relaxed);
                    info!(
                        "Worker {} disconnected ({} workers now connected)",
                        worker_id,
                        connected_workers.load(Ordering::Relaxed)
                    );
                    // Send error response and break to reconnect
                    let _ = request.response_tx.send(result);
                    break;
                }
            }

            // Send response back
            let _ = request.response_tx.send(result);
        }
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
    }
}
