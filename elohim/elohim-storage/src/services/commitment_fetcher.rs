//! Substrate-side fetch of Mishpat::Commitment records; used by bounds_validator
//! and republish_epr_validator to walk bounded_by references.
//!
//! The [`CommitmentFetcher`] trait is the seam between validation logic and
//! the conductor that holds the commitment entries. Tests run against
//! [`MockCommitmentFetcher`]; the production [`ConductorCommitmentFetcher`]
//! is a stub until Sprint 1 lands the Mishpat::Commitment entry type and
//! the `get_commitment` coordinator function.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A Mishpat::Commitment record fetched from the conductor.
///
/// Mirrors the REA Commitment shape bounded to a compute-class scope.
/// `bounds` carries structured policy (scope, rate, rotation TTL, etc.)
/// as a JSON value; the validator is responsible for interpreting it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitmentRecord {
    /// Content-addressed identifier for this commitment entry.
    pub cid: String,
    /// REA action (e.g. `"delegates-compute"`).
    pub action: String,
    /// Scope this commitment covers (e.g. `"republish-epr"`).
    pub scope: String,
    /// Agent DID or identifier of the resource provider.
    pub provider: String,
    /// Agent DID or identifier of the resource recipient.
    pub recipient: String,
    /// Structured policy bounds (rate limits, TTL, reach ceiling, etc.).
    pub bounds: serde_json::Value,
    /// ISO-8601 start of the commitment window.
    pub valid_from: String,
    /// ISO-8601 end of the commitment window.
    pub valid_until: String,
    /// ISO-8601 revocation timestamp, if the commitment has been revoked.
    pub revoked_at: Option<String>,
}

/// Error variants for commitment fetch operations.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("conductor unreachable: {0}")]
    ConductorUnreachable(String),
    #[error("malformed commitment record: {0}")]
    MalformedRecord(String),
}

/// Fetches a [`CommitmentRecord`] by its CID from the Mishpat zome.
///
/// Returns `Ok(None)` when no commitment exists for the given CID.
/// Returns `Err(FetchError)` only on infrastructure failure.
#[async_trait]
pub trait CommitmentFetcher: Send + Sync {
    async fn fetch(&self, cid: &str) -> Result<Option<CommitmentRecord>, FetchError>;
}

// ---------------------------------------------------------------------------
// Production implementation (Sprint 1 dependency — stub only)
// ---------------------------------------------------------------------------

/// Production fetcher — delegates to the local Mishpat zome via the conductor.
///
/// **Not yet wired.** Sprint 1 will add `hc_client::call_zome` on
/// `mishpat::get_commitment` once the Mishpat::Commitment entry type and
/// coordinator function exist. Until then this always returns
/// [`FetchError::ConductorUnreachable`] so callers know the dependency is
/// unsatisfied rather than silently receiving `None`.
pub struct ConductorCommitmentFetcher {
    pub hc_client: Arc<crate::hc_client::HcClient>,
}

#[async_trait]
impl CommitmentFetcher for ConductorCommitmentFetcher {
    async fn fetch(&self, _cid: &str) -> Result<Option<CommitmentRecord>, FetchError> {
        // TODO(Sprint 1): wire to hc_client::call_zome on mishpat::get_commitment
        // once Sprint 1 lands the Mishpat::Commitment entry type and the
        // get_commitment coordinator function. For Sprint 2 the production
        // path is a stub; tests run against MockCommitmentFetcher.
        Err(FetchError::ConductorUnreachable(
            "ConductorCommitmentFetcher not yet wired — Sprint 1 dependency".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Test mock
// ---------------------------------------------------------------------------

/// In-memory mock that supports seeding known records for unit tests.
///
/// `std::sync::Mutex` is intentional — the lock is never held across an
/// `.await` boundary (seed and fetch both acquire and release within a
/// single synchronous block), so the std variant is both simpler and
/// cheaper than `tokio::sync::Mutex` here.
pub struct MockCommitmentFetcher {
    inner: Arc<Mutex<HashMap<String, CommitmentRecord>>>,
}

impl MockCommitmentFetcher {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Seed a known commitment so `fetch` can return it.
    pub fn seed(&self, cid: &str, record: CommitmentRecord) {
        self.inner.lock().unwrap().insert(cid.to_string(), record);
    }
}

impl Default for MockCommitmentFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommitmentFetcher for MockCommitmentFetcher {
    async fn fetch(&self, cid: &str) -> Result<Option<CommitmentRecord>, FetchError> {
        Ok(self.inner.lock().unwrap().get(cid).cloned())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_active_commitment() -> CommitmentRecord {
        CommitmentRecord {
            cid: "commitment-cid-abc".into(),
            action: "delegates-compute".into(),
            scope: "republish-epr".into(),
            provider: "agent:matthew-steward".into(),
            recipient: "agent:deploy-svc-matthew".into(),
            bounds: serde_json::json!({
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            }),
            valid_from: "2026-05-01T00:00:00Z".into(),
            valid_until: "2026-08-01T00:00:00Z".into(),
            revoked_at: None,
        }
    }

    #[tokio::test]
    async fn mock_fetcher_returns_seeded_commitment() {
        let mock = MockCommitmentFetcher::new();
        mock.seed("commitment-cid-abc", sample_active_commitment());
        let result = mock
            .fetch("commitment-cid-abc")
            .await
            .unwrap()
            .expect("seeded commitment must be present");
        assert_eq!(result.action, "delegates-compute");
    }

    #[tokio::test]
    async fn mock_fetcher_returns_none_for_unknown_cid() {
        let mock = MockCommitmentFetcher::new();
        let result = mock.fetch("commitment-cid-unknown").await.unwrap();
        assert!(result.is_none());
    }
}
