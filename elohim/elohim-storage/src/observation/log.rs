//! In-memory append-only log of observations per observer.
//!
//! The log is the substrate's source of truth for an observer's emitted
//! observations. Each append advances a rolling BLAKE3 root (`log_cid`).
//! In production the log will back onto iroh-blobs for content-addressed
//! durability (Task 4.5); this in-memory primitive is the foundation that
//! lets Stage 4 components be unit-tested without iroh.
//!
//! See genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md §5.1.

use crate::observation::wire::Observation;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObservationLogError {
    #[error("encoding error: {0}")]
    Encoding(String),
}

/// Per-observer append-only log. Maintains a rolling BLAKE3 root over the
/// MessagePack encoding of each appended observation.
pub struct ObservationLog {
    observer_cid: String,
    entries: Vec<Observation>,
    rolling_hasher: blake3::Hasher,
    current_root: String,
}

impl ObservationLog {
    /// Create an empty log keyed by observer_cid.
    pub fn new_in_memory(observer_cid: String) -> Self {
        let hasher = blake3::Hasher::new();
        let initial = format!("blake3:{}", hasher.finalize().to_hex());
        Self {
            observer_cid,
            entries: Vec::new(),
            rolling_hasher: hasher,
            current_root: initial,
        }
    }

    pub fn observer_cid(&self) -> &str {
        &self.observer_cid
    }

    pub fn current_log_cid(&self) -> String {
        self.current_root.clone()
    }

    pub fn latest_offset(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Append an observation. Hashes its MessagePack encoding into the rolling
    /// root and stores the row in order.
    pub async fn append(&mut self, obs: Observation) -> Result<(), ObservationLogError> {
        let bytes =
            rmp_serde::to_vec(&obs).map_err(|e| ObservationLogError::Encoding(e.to_string()))?;
        self.rolling_hasher.update(&bytes);
        self.current_root = format!("blake3:{}", self.rolling_hasher.finalize().to_hex());
        self.entries.push(obs);
        Ok(())
    }

    /// Read all observations at or after the given offset, in append order.
    pub async fn read_from(&self, offset: u64) -> Result<Vec<Observation>, ObservationLogError> {
        Ok(self.entries.iter().skip(offset as usize).cloned().collect())
    }
}
