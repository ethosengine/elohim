//! In-memory append-only log of observations per observer.
//!
//! The log is the substrate's source of truth for an observer's emitted
//! observations. Each append advances a rolling BLAKE3 root (`log_cid`).
//! In production the log will back onto iroh-blobs for content-addressed
//! durability (Task 4.5); this in-memory primitive is the foundation that
//! lets Stage 4 components be unit-tested without iroh.
//!
//! See genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md §5.1.

use crate::observation::wire::Observation;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObservationLogError {
    #[error("encoding error: {0}")]
    Encoding(String),
    /// The SQL projection or the persisted log head could not be written.
    #[error("persistence error: {0}")]
    Persistence(String),
}

/// Per-observer append-only log. Maintains a rolling BLAKE3 root over the
/// MessagePack encoding of each appended observation.
///
/// A log is either fresh ([`ObservationLog::new_in_memory`], offset 0) or
/// resumed from a persisted head ([`ObservationLog::resume`]). A resumed log
/// holds only the observations appended since it resumed; `base_offset` is the
/// number of observations that precede them.
pub struct ObservationLog {
    observer_cid: String,
    base_offset: u64,
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
            base_offset: 0,
            entries: Vec::new(),
            rolling_hasher: hasher,
            current_root: initial,
        }
    }

    /// Resume a log from its persisted head (`observation_logs.latest_offset`,
    /// `observation_logs.latest_log_cid`), so offsets continue instead of
    /// re-minting 0 after a restart.
    ///
    /// A `blake3::Hasher` is not serialisable, so the rolling state cannot be
    /// restored. Instead the resumed hasher is seeded with the stored root
    /// (its `blake3:<hex>` text) and every later append is hashed after it.
    /// From a resume onwards, `log_cid` therefore means a **chained** root —
    /// BLAKE3 over (previous root ‖ observations since) — not the root of one
    /// uninterrupted stream over every observation: a log resumed at offset N
    /// and a log that never stopped reach different roots for the same bytes.
    /// Both are stable, and each still commits to everything before it through
    /// the stored root. The signed, persisted iroh-backed log named in the
    /// `attention-witnessed-privately` habit's retire-when replaces both this
    /// in-memory log and the chaining.
    ///
    /// The observations before `latest_offset` are not held in memory: the SQL
    /// projection keeps them, and `read_from` serves only the resumed tail.
    pub fn resume(observer_cid: String, latest_offset: u64, latest_log_cid: String) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(latest_log_cid.as_bytes());
        Self {
            observer_cid,
            base_offset: latest_offset,
            entries: Vec::new(),
            rolling_hasher: hasher,
            current_root: latest_log_cid,
        }
    }

    pub fn observer_cid(&self) -> &str {
        &self.observer_cid
    }

    pub fn current_log_cid(&self) -> String {
        self.current_root.clone()
    }

    /// The next offset to be written (= the number of observations in the log,
    /// including those before a resume).
    pub fn latest_offset(&self) -> u64 {
        self.base_offset + self.entries.len() as u64
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

    /// Read all observations held in memory at or after the given offset, in
    /// append order. For a resumed log, offsets before the resume point are not
    /// held and are skipped.
    pub async fn read_from(&self, offset: u64) -> Result<Vec<Observation>, ObservationLogError> {
        let skip = offset.saturating_sub(self.base_offset) as usize;
        Ok(self.entries.iter().skip(skip).cloned().collect())
    }
}
