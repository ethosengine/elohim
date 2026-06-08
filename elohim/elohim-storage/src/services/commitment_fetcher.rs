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

use crate::db::DbPool;

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
    /// The row exists in the projection table but carries a NULL
    /// `dht_anchor_hash`, meaning it has not yet been notarized on the DHT.
    ///
    /// `bounds_validator` maps this to `ViolationKind::CommitmentNotFound`
    /// (fail-closed): a bounds-gate is NEVER cleared on un-notarized provenance.
    #[error("commitment not notarized (null dht_anchor_hash): {0}")]
    NotarizedRequired(String),
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
/// Reads a notarized `Commitment` back through the `mishpat::get_commitment`
/// coordinator (Slice 2b T1). The coordinator returns `action`, `payload_json`,
/// and `signed_at`; the policy envelope (provider/recipient/scope/bounds/
/// validity) lives inside `payload_json` and is parsed here into a
/// [`CommitmentRecord`] the bounds validator can walk.
pub struct ConductorCommitmentFetcher {
    pub hc_client: Arc<crate::hc_client::HcClient>,
}

#[async_trait]
impl CommitmentFetcher for ConductorCommitmentFetcher {
    async fn fetch(&self, cid: &str) -> Result<Option<CommitmentRecord>, FetchError> {
        let out = crate::services::conductor_writes::get_commitment(&self.hc_client, cid)
            .await
            .map_err(|e| FetchError::ConductorUnreachable(format!("get_commitment({cid}): {e}")))?;
        let Some(out) = out else {
            // Not on this conductor's DHT view → Ok(None); validator maps to CommitmentNotFound.
            return Ok(None);
        };

        // Parse the inner policy envelope. provider/recipient/scope/bounds/validity
        // live in payload_json (the wire shape is action + payload_json + signed_at).
        let payload: serde_json::Value = serde_json::from_str(&out.payload_json)
            .map_err(|e| FetchError::MalformedRecord(format!("payload_json parse: {e}")))?;

        let str_field = |k: &str| {
            payload
                .get(k)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        Ok(Some(CommitmentRecord {
            cid: out.entry_hash,
            action: out.action,
            scope: str_field("scope"),
            provider: str_field("provider"),
            recipient: str_field("recipient"),
            bounds: payload
                .get("bounds")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            valid_from: str_field("valid_from"),
            valid_until: str_field("valid_until"),
            revoked_at: payload
                .get("revoked_at")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        }))
    }
}

// ---------------------------------------------------------------------------
// Production implementation: projection-table backed (Slice 2a T6)
// ---------------------------------------------------------------------------

/// Production `CommitmentFetcher`: reads the `mishpat_commitments` projection
/// table (the read-optimised cache of DHT-notarised compute-bounds commitments —
/// P1: storage is the projection of DHT truth). Bounds-checks read here, NOT
/// from a live conductor (`depin_contracts_are_policy`: operational loops READ
/// bounds).
///
/// **GUARD**: a row with `dht_anchor_hash IS NULL` is un-notarized/storage-only.
/// The fetcher returns [`FetchError::NotarizedRequired`] for such rows so a
/// bounds-gate is NEVER cleared on un-notarized provenance (fail-closed).
/// Spec §6.5.
pub struct ProjectionCommitmentFetcher {
    pool: DbPool,
}

impl ProjectionCommitmentFetcher {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

/// Pure mapping from a `MishpatCommitment` row to a `CommitmentRecord`.
///
/// Factored out so the null-anchor guard and field mapping can be unit-tested
/// without a live DB pool.  Returns `Err(FetchError::NotarizedRequired)` when
/// `row.dht_anchor_hash` is `None`.
pub fn record_from_row(
    row: crate::db::models::MishpatCommitment,
) -> Result<CommitmentRecord, FetchError> {
    if row.dht_anchor_hash.is_none() {
        return Err(FetchError::NotarizedRequired(format!(
            "commitment {} is storage-only (null dht_anchor_hash) — not notarized",
            row.cid
        )));
    }
    let bounds: serde_json::Value = serde_json::from_str(&row.bounds_json)
        .map_err(|e| FetchError::ConductorUnreachable(format!("bounds_json parse: {e}")))?;
    Ok(CommitmentRecord {
        cid: row.cid,
        action: row.action,
        scope: row.scope,
        provider: row.provider,
        recipient: row.recipient,
        bounds,
        valid_from: row.valid_from,
        valid_until: row.valid_until,
        revoked_at: row.revoked_at,
    })
}

#[async_trait]
impl CommitmentFetcher for ProjectionCommitmentFetcher {
    async fn fetch(&self, cid: &str) -> Result<Option<CommitmentRecord>, FetchError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| FetchError::ConductorUnreachable(format!("db pool: {e}")))?;
        let row = crate::db::mishpat_commitments::get_by_cid(&mut conn, cid)
            .map_err(|e| FetchError::ConductorUnreachable(format!("query: {e}")))?;
        let Some(row) = row else {
            // Not found → Ok(None); bounds_validator maps this to CommitmentNotFound.
            return Ok(None);
        };
        record_from_row(row).map(Some)
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

    // -----------------------------------------------------------------------
    // record_from_row pure-fn tests (guard logic + field mapping)
    // -----------------------------------------------------------------------

    fn sample_mishpat_row(cid: &str, anchor: Option<&str>) -> crate::db::models::MishpatCommitment {
        crate::db::models::MishpatCommitment {
            cid: cid.to_string(),
            action: "delegates-compute".to_string(),
            scope: "republish-epr".to_string(),
            provider: "agent:matthew-steward".to_string(),
            recipient: "agent:deploy-svc".to_string(),
            bounds_json: r#"{"epr_scope":["epr:lamad-spa"],"reach_ceiling":"commons","rate_per_hour":30,"rotation_ttl_days":90}"#.to_string(),
            valid_from: "2026-06-01T00:00:00Z".to_string(),
            valid_until: "2026-09-01T00:00:00Z".to_string(),
            revoked_at: None,
            state: "active".to_string(),
            dht_anchor_hash: anchor.map(str::to_string),
            created_at: "2026-06-07T00:00:00Z".to_string(),
            updated_at: "2026-06-07T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn record_from_row_maps_notarized_row_to_record() {
        let row = sample_mishpat_row("cid:abc", Some("anchor:hash1"));
        let record = record_from_row(row).expect("notarized row must succeed");
        assert_eq!(record.cid, "cid:abc");
        assert_eq!(record.action, "delegates-compute");
        assert_eq!(record.scope, "republish-epr");
        assert_eq!(record.provider, "agent:matthew-steward");
        assert_eq!(record.recipient, "agent:deploy-svc");
        assert_eq!(record.valid_from, "2026-06-01T00:00:00Z");
        assert_eq!(record.valid_until, "2026-09-01T00:00:00Z");
        assert!(record.revoked_at.is_none());
        // bounds parsed to Value
        assert_eq!(record.bounds["rate_per_hour"], 30);
        assert_eq!(record.bounds["reach_ceiling"], "commons");
    }

    #[test]
    fn record_from_row_refuses_null_anchor_row() {
        // GUARD: un-notarized row must never clear a bounds-gate (spec §6.5).
        let row = sample_mishpat_row("cid:unanchored", None);
        let err = record_from_row(row).expect_err("null dht_anchor_hash must return Err");
        assert!(
            matches!(err, FetchError::NotarizedRequired(_)),
            "expected NotarizedRequired, got: {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // ProjectionCommitmentFetcher pool-backed tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn fetches_notarized_commitment() {
        let pool = crate::test_util::test_pool();
        // Seed a notarized row directly via upsert.
        {
            let mut conn = pool.get().expect("pool conn");
            crate::db::mishpat_commitments::upsert_with_anchor(
                &mut conn,
                crate::db::models::NewMishpatCommitment {
                    cid: "cid:notarized-1".to_string(),
                    action: "delegates-compute".to_string(),
                    scope: "republish-epr".to_string(),
                    provider: "agent:matthew-steward".to_string(),
                    recipient: "agent:deploy-svc".to_string(),
                    bounds_json: r#"{"epr_scope":["epr:lamad-spa"],"reach_ceiling":"commons","rate_per_hour":30,"rotation_ttl_days":90}"#.to_string(),
                    valid_from: "2026-06-01T00:00:00Z".to_string(),
                    valid_until: "2026-09-01T00:00:00Z".to_string(),
                    revoked_at: None,
                    state: "active".to_string(),
                    dht_anchor_hash: Some("anchor:dht-hash-abc".to_string()),
                },
            )
            .expect("upsert");
        }

        let fetcher = ProjectionCommitmentFetcher::new(pool);
        let record = fetcher
            .fetch("cid:notarized-1")
            .await
            .expect("fetch must succeed")
            .expect("notarized row must be present");

        assert_eq!(record.cid, "cid:notarized-1");
        assert_eq!(record.action, "delegates-compute");
        assert_eq!(record.bounds["rate_per_hour"], 30);
        assert!(record.revoked_at.is_none());
    }

    #[tokio::test]
    async fn refuses_null_anchor_row() {
        // GUARD: storage-only row (no dht_anchor_hash) must fail closed.
        let pool = crate::test_util::test_pool();
        {
            let mut conn = pool.get().expect("pool conn");
            crate::db::mishpat_commitments::upsert_with_anchor(
                &mut conn,
                crate::db::models::NewMishpatCommitment {
                    cid: "cid:unanchored-1".to_string(),
                    action: "delegates-compute".to_string(),
                    scope: "republish-epr".to_string(),
                    provider: "agent:matthew".to_string(),
                    recipient: "agent:deploy-svc".to_string(),
                    bounds_json: r#"{"rate_per_hour":10}"#.to_string(),
                    valid_from: "2026-06-01T00:00:00Z".to_string(),
                    valid_until: "2026-09-01T00:00:00Z".to_string(),
                    revoked_at: None,
                    state: "proposed".to_string(),
                    dht_anchor_hash: None, // un-notarized
                },
            )
            .expect("upsert");
        }

        let fetcher = ProjectionCommitmentFetcher::new(pool);
        let err = fetcher
            .fetch("cid:unanchored-1")
            .await
            .expect_err("un-notarized row must return Err");
        assert!(
            matches!(err, FetchError::NotarizedRequired(_)),
            "expected NotarizedRequired, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn missing_cid_returns_none() {
        let pool = crate::test_util::test_pool();
        let fetcher = ProjectionCommitmentFetcher::new(pool);
        let result = fetcher
            .fetch("cid:does-not-exist")
            .await
            .expect("fetch must not error for missing cid");
        assert!(result.is_none(), "unknown cid must return Ok(None)");
    }
}
