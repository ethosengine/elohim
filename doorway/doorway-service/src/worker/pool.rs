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

/// Maximum consecutive connection failures before logging at error level
const MAX_RETRIES_BEFORE_UNHEALTHY: u32 = 5;

/// Base delay for exponential backoff on connection retry
const BASE_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Maximum delay between connection retries
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

/// Compute exponential backoff with jitter for connection retries.
/// Uses full-jitter strategy: sleep = random(0, min(cap, base * 2^attempt))
fn backoff_with_jitter(attempt: u32) -> Duration {
    let exp_ms = BASE_RETRY_DELAY.as_millis() as u64 * 2u64.saturating_pow(attempt);
    let capped_ms = exp_ms.min(MAX_RETRY_DELAY.as_millis() as u64);
    // Simple jitter: use attempt + time-based seed for variance without pulling in rand
    let jitter_seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let jitter_ms = if capped_ms > 0 {
        jitter_seed % capped_ms
    } else {
        0
    };
    Duration::from_millis(jitter_ms)
}

/// Worker task that processes requests from the pool
async fn worker_task(
    worker_id: usize,
    conductor_url: String,
    request_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<PoolRequest>>>,
    timeout_ms: u64,
    connected_workers: Arc<AtomicUsize>,
) {
    info!(
        "Worker {} starting, connecting to {}",
        worker_id, conductor_url
    );

    let mut consecutive_failures: u32 = 0;

    loop {
        // Connect to conductor with exponential backoff on failure
        let conductor = match ConductorConnection::connect(&conductor_url).await {
            Ok(c) => {
                // Reset failure counter on successful connect
                consecutive_failures = 0;
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
                consecutive_failures += 1;
                let delay = backoff_with_jitter(consecutive_failures);

                if consecutive_failures >= MAX_RETRIES_BEFORE_UNHEALTHY {
                    error!(
                        "Worker {} failed to connect (attempt {}): {} — conductor may be unhealthy. Retrying in {:?}",
                        worker_id, consecutive_failures, e, delay
                    );
                } else {
                    info!(
                        "Worker {} failed to connect (attempt {}): {}, retrying in {:?}",
                        worker_id, consecutive_failures, e, delay
                    );
                }

                tokio::time::sleep(delay).await;
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

            debug!(
                "Worker {} processing request ({} bytes)",
                worker_id,
                request.payload.len()
            );

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

    #[test]
    fn test_backoff_with_jitter_bounded() {
        // Verify backoff never exceeds MAX_RETRY_DELAY
        for attempt in 0..20 {
            let delay = backoff_with_jitter(attempt);
            assert!(
                delay <= MAX_RETRY_DELAY,
                "Backoff delay {:?} exceeded max {:?} at attempt {}",
                delay,
                MAX_RETRY_DELAY,
                attempt
            );
        }
    }

    #[test]
    fn test_backoff_at_zero_attempt() {
        // At attempt 0, max possible is BASE_RETRY_DELAY (500ms)
        let delay = backoff_with_jitter(0);
        assert!(delay <= BASE_RETRY_DELAY);
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
