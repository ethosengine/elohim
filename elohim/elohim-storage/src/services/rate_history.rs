//! RateHistory — sliding-window event-count query over `economic_events`.
//!
//! The trait is the seam between bounds-validation logic and the database.
//! Tests run against [`MockRateHistory`]; the production [`DieselRateHistory`]
//! queries the `economic_events` table filtered by `bounded_by` (the
//! commitment CID back-reference added in migration
//! `2026-05-28-050000_economic_events_add_bounded_by`) and
//! `has_point_in_time` (the ValueFlows ontology temporal field).
//!
//! ## Care-class / compute-class isolation
//!
//! This module operates exclusively on compute-class signals:
//! `bounded_by` is populated only for EconomicEvents emitted under a
//! Mishpat::Commitment (compute-class action `delegates-compute`).  It must
//! never be used to debit care-class attributions.  See
//! `project_compute_commitments_bounded` in MEMORY.md.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::db::DbPool;
use crate::StorageError;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Counts economic events whose `bounded_by` field equals `commitment_cid`
/// and whose `has_point_in_time` falls within the sliding window
/// `[now - window_minutes, now]`.
///
/// Used by `bounds_validator` to enforce per-commitment rate limits without
/// loading the full event list into the validator.
#[async_trait]
pub trait RateHistory: Send + Sync {
    async fn count_in_window(
        &self,
        commitment_cid: &str,
        now_iso: &str,
        window_minutes: u32,
    ) -> Result<u32, StorageError>;
}

// ---------------------------------------------------------------------------
// Production implementation
// ---------------------------------------------------------------------------

/// Diesel-backed production impl.
///
/// Queries the `economic_events` table via a `bounded_by` + time-range
/// filter.  The query is offloaded to a `spawn_blocking` thread so callers
/// get a non-blocking `async fn`.
pub struct DieselRateHistory {
    pub pool: DbPool,
}

#[async_trait]
impl RateHistory for DieselRateHistory {
    async fn count_in_window(
        &self,
        commitment_cid: &str,
        now_iso: &str,
        window_minutes: u32,
    ) -> Result<u32, StorageError> {
        use crate::db::diesel_schema::economic_events::dsl;
        use diesel::prelude::*;

        let pool = self.pool.clone();
        let cid_owned = commitment_cid.to_string();
        let now_owned = now_iso.to_string();

        tokio::task::spawn_blocking(move || -> Result<u32, StorageError> {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Database(e.to_string()))?;

            let cutoff = chrono::DateTime::parse_from_rfc3339(&now_owned)
                .map_err(|e| StorageError::InvalidInput(format!("bad now_iso: {e}")))?
                - chrono::Duration::minutes(window_minutes as i64);
            let cutoff_iso = cutoff.to_rfc3339();

            let n: i64 = dsl::economic_events
                .filter(dsl::bounded_by.eq(&cid_owned))
                .filter(dsl::has_point_in_time.ge(&cutoff_iso))
                .count()
                .get_result::<i64>(&mut conn)
                .map_err(|e| StorageError::Database(e.to_string()))?;

            Ok(n as u32)
        })
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?
    }
}

// ---------------------------------------------------------------------------
// Test mock
// ---------------------------------------------------------------------------

/// In-memory mock that supports seeding a count per commitment CID.
///
/// The `_at_iso` parameter is accepted for API symmetry with future mocks
/// that may want time-bucketed seeding, but this simple impl ignores it and
/// returns the total seeded count regardless of window.  That is intentional
/// for Sprint 2 unit tests where the exact window arithmetic is not under
/// test — the validator logic is.
///
/// `std::sync::Mutex` is intentional: the lock is never held across an
/// `.await` boundary (seed and count both acquire and release within a single
/// synchronous block), so the std variant is simpler and cheaper than
/// `tokio::sync::Mutex`.
pub struct MockRateHistory {
    inner: Arc<Mutex<HashMap<String, u32>>>,
}

impl MockRateHistory {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Seed a count for a given commitment CID.
    ///
    /// The `_at_iso` parameter is reserved for future time-bucketed mocks;
    /// this impl accumulates the count regardless of the timestamp.
    pub fn seed(&self, cid: &str, _at_iso: &str, count: u32) {
        self.inner
            .lock()
            .unwrap()
            .insert(cid.to_string(), count);
    }
}

impl Default for MockRateHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RateHistory for MockRateHistory {
    async fn count_in_window(
        &self,
        commitment_cid: &str,
        _now_iso: &str,
        _window_minutes: u32,
    ) -> Result<u32, StorageError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .get(commitment_cid)
            .copied()
            .unwrap_or(0))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_returns_seeded_count() {
        let mock = MockRateHistory::new();
        mock.seed("commitment-cid-abc", "2026-05-28T12:00:00Z", 7);
        let count = mock
            .count_in_window("commitment-cid-abc", "2026-05-28T12:30:00Z", 60)
            .await
            .unwrap();
        assert_eq!(count, 7);
    }

    #[tokio::test]
    async fn mock_returns_zero_for_unseeded() {
        let mock = MockRateHistory::new();
        let count = mock
            .count_in_window("commitment-cid-never-seeded", "2026-05-28T12:00:00Z", 60)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
