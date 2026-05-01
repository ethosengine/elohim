//! Mirrors — DHT propagation / sync wait helpers.
//!
//! Sweettest conductors share a test network in-process, but DHT writes still
//! propagate asynchronously. Cross-agent assertions must poll the actual
//! zome predicate they expect — gossip quiescence is not a hard guarantee
//! for link traversal, so a fixed sleep races the read path. Use [`wait_for`]
//! for predicate-shaped waits, or inline a poll loop modeled on
//! `tests/node_registry.rs::admission_visible_across_agents`.

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

/// Poll a zome-call predicate until it returns true or the deadline elapses.
///
/// Default polling cadence is 100ms; callers that hit timeouts can increase
/// the max wait via `max_wait_ms`.
pub async fn wait_for<F, Fut>(max_wait_ms: u64, mut check: F) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<bool>>,
{
    let deadline = std::time::Instant::now() + Duration::from_millis(max_wait_ms);
    loop {
        if check().await? {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(anyhow::anyhow!(
                "mirrors::wait_for timed out after {max_wait_ms}ms"
            ));
        }
        sleep(Duration::from_millis(100)).await;
    }
}
