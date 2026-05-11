//! `IrohObservationBackend` — wraps an `ObservationLog` and serves segments
//! filtered by `(observer_cid, from_offset, to_offset)`.
//!
//! In production this will publish the log into iroh-blobs and respond to
//! `OBSERVATION_LOG_ALPN` fetch requests over QUIC. The in-memory backend
//! lets Stage 4 components be tested without an iroh endpoint.
//!
//! See spec §5.1.

use crate::observation::log::{ObservationLog, ObservationLogError};
use crate::observation::wire::Observation;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct IrohObservationBackend {
    log: Arc<RwLock<ObservationLog>>,
}

impl IrohObservationBackend {
    pub fn new_in_memory(log: ObservationLog) -> Self {
        Self {
            log: Arc::new(RwLock::new(log)),
        }
    }

    pub async fn append(&self, obs: Observation) -> Result<(), ObservationLogError> {
        self.log.write().await.append(obs).await
    }

    /// Fetch observations from this observer's log in `[from_offset, to_offset)`.
    /// Returns empty Vec if observer_cid doesn't match this backend's local log.
    pub async fn fetch_segment(
        &self,
        observer_cid: &str,
        from_offset: u64,
        to_offset: u64,
    ) -> Result<Vec<Observation>, ObservationLogError> {
        let guard = self.log.read().await;
        if guard.observer_cid() != observer_cid {
            return Ok(vec![]);
        }
        let all = guard.read_from(from_offset).await?;
        let span = (to_offset.saturating_sub(from_offset)) as usize;
        Ok(all.into_iter().take(span).collect())
    }
}
