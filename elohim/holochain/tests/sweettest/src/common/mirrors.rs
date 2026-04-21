//! Mirrors — DHT propagation / sync wait helpers.
//!
//! Sweettest conductors share a test network in-process, but DHT writes still
//! propagate asynchronously. These helpers wrap the polling patterns so test
//! files don't reinvent them.

use anyhow::Result;
use holochain::sweettest::SweetCell;
use std::time::Duration;
use tokio::time::sleep;

/// Poll a zome-call predicate until it returns true or the deadline elapses.
///
/// Default polling cadence is 100ms; callers that hit timeouts can increase
/// the max wait via `max_wait_ms`.
pub async fn wait_for<F, Fut>(
    max_wait_ms: u64,
    mut check: F,
) -> Result<()>
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

/// Wait for DHT gossip between two cells to stabilize.
///
/// Simple cadence: short sleep to let the in-process network settle. Callers
/// expecting a specific entry should prefer [`wait_for`] with a predicate
/// that fetches the entry.
pub async fn settle_dht(_cells: &[&SweetCell]) {
    sleep(Duration::from_millis(250)).await;
}
