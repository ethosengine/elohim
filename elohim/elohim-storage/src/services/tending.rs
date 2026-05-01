//! Tending lifecycle service — TTL enforcement, re-tending, expiry sweep.
//!
//! Category C (operational). The Holochain side (T5 HDI integrity entry,
//! T9 coordinator) is the authoritative source of truth for AttentionTending.
//! This service maintains the local SQLite cache (db::tending) and runs the
//! periodic TTL sweep that keeps the cache aligned with §6.6 lifetime
//! defaults (mindless filter-everything is structurally costly per §6.2).
//!
//! TODO(T19): wire enforce_ttls into ReconcileController in main.rs.
//!   Suggested cadence: every 5 minutes (matches existing reconciliation
//!   intervals — see PubkeyTimelineCache for precedent). On error, log
//!   tracing::error! and continue — sweep failure is recoverable next tick.
//!   Spawn site: in main.rs around the existing tokio::spawn for the
//!   ReconcileController (search "spawn(async move" near line ~985).

use diesel::sqlite::SqliteConnection;

use crate::db::tending::{
    delete_expired, insert, refresh as db_refresh, NewTendingRow, TendingRow,
};
use crate::p2p::attention_tending::{AttentionTending, Classification};

// ============================================================================
// Default TTLs
// ============================================================================

/// Default TTL in seconds for a given classification (brainstorm §6.6).
///
/// Safety returns `i64::MAX` — the sweep function skips Safety rows
/// regardless of their on-wire `ttl_seconds` value. The convention
/// "Safety has no expiry" is enforced structurally by `delete_expired`.
pub fn default_ttl(classification: Classification) -> i64 {
    match classification {
        Classification::Safety => i64::MAX,
        Classification::Fatigue => 604_800,         // 7 days
        Classification::ValuesForward => 2_592_000, // 30 days
        Classification::ScopeMismatch => 7_776_000, // 90 days
    }
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum TendingError {
    #[error("database: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("encode: {0}")]
    Encode(String),
    #[error("invalid: {0}")]
    Invalid(String),
}

// ============================================================================
// Public API
// ============================================================================

/// Insert a new tending record into the local cache from a wire AttentionTending.
///
/// `tending_cid` is the source-chain ActionHash (or wire CID) for the entry.
/// Errors with `TendingError::Invalid` when `tended_at` is empty (structurally
/// meaningless — a tending with no events cannot define created_at or last_tended_at).
pub fn record_tending(
    conn: &mut SqliteConnection,
    tending_cid: &str,
    signer_pubkey: &[u8],
    tending: &AttentionTending,
) -> Result<i32, TendingError> {
    if tending.tended_at.is_empty() {
        return Err(TendingError::Invalid(
            "tended_at must be non-empty".to_string(),
        ));
    }
    // Guard: a ttl_seconds value above i64::MAX would wrap to a negative integer
    // via `as i64`, causing `last_tended_at + ttl_seconds < now` to evaluate true
    // immediately on the next sweep — silent data loss for the signer.
    if tending.ttl_seconds > i64::MAX as u64 {
        return Err(TendingError::Invalid(format!(
            "ttl_seconds {} exceeds i64::MAX; would silently truncate to negative",
            tending.ttl_seconds
        )));
    }
    // Safety: is_empty() guard above ensures both unwrap() calls are safe.
    let created_at = *tending.tended_at.first().unwrap() as i64;
    let last_tended_at = *tending.tended_at.last().unwrap() as i64;

    let tended_at_history_json = serde_json::to_string(&tending.tended_at)
        .map_err(|e| TendingError::Encode(e.to_string()))?;
    let filter_subject_json = serde_json::to_string(&tending.filter_subject)
        .map_err(|e| TendingError::Encode(e.to_string()))?;
    let context_json =
        serde_json::to_string(&tending.context).map_err(|e| TendingError::Encode(e.to_string()))?;

    Ok(insert(
        conn,
        &NewTendingRow {
            tending_cid: tending_cid.to_string(),
            signer_pubkey: signer_pubkey.to_vec(),
            classification: classification_str(tending.classification).to_string(),
            filter_subject_json,
            context_json,
            reason: tending.reason.clone(),
            ttl_seconds: tending.ttl_seconds as i64,
            created_at,
            last_tended_at,
            tended_at_history_json,
        },
    )?)
}

/// Append a new tending timestamp to the cache row, updating `last_tended_at`
/// and the history. Mirrors the source-chain `refresh_tending_ttl` semantics
/// from T9.
pub fn record_retend(
    conn: &mut SqliteConnection,
    id: i32,
    new_tended_at: i64,
) -> Result<TendingRow, TendingError> {
    Ok(db_refresh(conn, id, new_tended_at)?)
}

/// Run the TTL sweep: delete cache rows where the effective expiry has passed.
///
/// Safety classification rows are NEVER swept (no-expiry by convention).
/// Returns the count of rows deleted.
pub fn enforce_ttls(conn: &mut SqliteConnection, now: i64) -> Result<usize, TendingError> {
    Ok(delete_expired(conn, now)?)
}

/// Wall-clock TTL sweep — captures `now` internally and delegates to
/// [`enforce_ttls`].
///
/// Preferred entry point for ReconcileController's periodic sweep task.
/// Uses `Utc::now().timestamp()` (epoch seconds, matching `last_tended_at`
/// column precision).
///
/// Safety classification is NEVER swept regardless of `ttl_seconds` on the
/// row — constitutional floor protection per brainstorm §2.8.
///
/// Idempotent: a second call with no new expired rows returns 0.
///
/// # Returns
///
/// The number of expired non-safety rows that were deleted from
/// `attention_tending`.
pub fn sweep_expired(conn: &mut SqliteConnection) -> Result<usize, TendingError> {
    let now_secs = chrono::Utc::now().timestamp();
    let deleted = enforce_ttls(conn, now_secs)?;
    tracing::debug!(deleted, "tending::sweep_expired completed");
    Ok(deleted)
}

// ============================================================================
// Helpers
// ============================================================================

/// Map Classification enum to its kebab-case SQL TEXT representation.
///
/// Mirrors `#[serde(rename_all = "kebab-case")]` on the Classification wire enum
/// so that TEXT stored in SQLite matches the on-wire form.
fn classification_str(c: Classification) -> &'static str {
    match c {
        Classification::Safety => "safety",
        Classification::Fatigue => "fatigue",
        Classification::ValuesForward => "values-forward",
        Classification::ScopeMismatch => "scope-mismatch",
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tending::{fetch_by_cid, insert_raw, list_all};
    use crate::db::{run_migrations, DbPool};
    use crate::p2p::attention_tending::Classification;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sqlite::SqliteConnection;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:svc_tending_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn make_tending(
        classification: Classification,
        tended_at: Vec<u64>,
        ttl: u64,
    ) -> AttentionTending {
        AttentionTending {
            filter_subject: serde_json::json!({"contentKind": "concept"}),
            classification,
            reason: None,
            ttl_seconds: ttl,
            tended_at,
            context: serde_json::json!({"collective": "household"}),
            signed_by: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
            signature: "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // T1: default_ttl values match brainstorm §6.6
    // -----------------------------------------------------------------------

    #[test]
    fn default_ttl_matches_brainstorm_section_6_6() {
        assert_eq!(
            default_ttl(Classification::Safety),
            i64::MAX,
            "Safety must be i64::MAX (no expiry)"
        );
        assert_eq!(
            default_ttl(Classification::Fatigue),
            604_800,
            "Fatigue must be 7 days = 604_800 seconds"
        );
        assert_eq!(
            default_ttl(Classification::ValuesForward),
            2_592_000,
            "ValuesForward must be 30 days = 2_592_000 seconds"
        );
        assert_eq!(
            default_ttl(Classification::ScopeMismatch),
            7_776_000,
            "ScopeMismatch must be 90 days = 7_776_000 seconds"
        );
    }

    // -----------------------------------------------------------------------
    // T2: record_tending persists all fields
    // -----------------------------------------------------------------------

    #[test]
    fn record_tending_persists_all_fields() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let tending = AttentionTending {
            filter_subject: serde_json::json!({"topic": "biology"}),
            classification: Classification::Fatigue,
            reason: Some("too much content on this topic".to_string()),
            ttl_seconds: 3_600,
            tended_at: vec![1_746_000_000, 1_746_003_600],
            context: serde_json::json!({"mode": "deep-dive"}),
            signed_by: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
            signature: "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=".to_string(),
        };
        let signer = vec![0x55u8; 32];

        let id = record_tending(&mut conn, "uhCkkT2Cid", &signer, &tending).expect("record");
        assert!(id > 0);

        let row = fetch_by_cid(&mut conn, "uhCkkT2Cid")
            .expect("fetch")
            .expect("must be Some");

        assert_eq!(row.tending_cid, "uhCkkT2Cid");
        assert_eq!(row.signer_pubkey, signer);
        assert_eq!(row.classification, "fatigue");
        assert_eq!(
            row.reason,
            Some("too much content on this topic".to_string())
        );
        assert_eq!(row.ttl_seconds, 3_600);
        assert_eq!(
            row.created_at, 1_746_000_000,
            "created_at = first tended_at"
        );
        assert_eq!(
            row.last_tended_at, 1_746_003_600,
            "last_tended_at = last tended_at"
        );

        // Verify history JSON round-trips as a Vec<u64>.
        let history: Vec<u64> =
            serde_json::from_str(&row.tended_at_history_json).expect("parse history");
        assert_eq!(history, vec![1_746_000_000u64, 1_746_003_600u64]);

        // Filter subject and context are faithfully stored.
        let fs: serde_json::Value =
            serde_json::from_str(&row.filter_subject_json).expect("parse filter_subject");
        assert_eq!(fs["topic"], "biology");

        let ctx: serde_json::Value =
            serde_json::from_str(&row.context_json).expect("parse context");
        assert_eq!(ctx["mode"], "deep-dive");
    }

    // -----------------------------------------------------------------------
    // T3: record_retend extends history and last_tended_at
    // -----------------------------------------------------------------------

    #[test]
    fn record_retend_extends_history_and_last_tended_at() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let tending = make_tending(Classification::ValuesForward, vec![1_000_000], 3_600);
        let id = record_tending(&mut conn, "uhCkkRetendCid", &[0x22u8; 32], &tending)
            .expect("record initial");

        // Re-tend with a later timestamp.
        let updated = record_retend(&mut conn, id, 2_000_000).expect("retend");

        assert_eq!(
            updated.last_tended_at, 2_000_000,
            "last_tended_at must reflect the new timestamp"
        );

        let history: Vec<i64> =
            serde_json::from_str(&updated.tended_at_history_json).expect("parse history");
        assert_eq!(history.len(), 2, "history should have both timestamps");
        assert_eq!(history[0], 1_000_000, "original timestamp preserved");
        assert_eq!(history[1], 2_000_000, "new timestamp appended");
    }

    // -----------------------------------------------------------------------
    // T4: enforce_ttls deletes expired non-safety rows
    // -----------------------------------------------------------------------

    #[test]
    fn enforce_ttls_deletes_expired_non_safety() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Expired Fatigue: last_tended=0, ttl=10 → expiry=10 < now=1000
        let fatigue = make_tending(Classification::Fatigue, vec![0], 10);
        record_tending(&mut conn, "cid-fatigue", &[0xAAu8; 32], &fatigue).expect("insert");

        // Alive ValuesForward: last_tended=1000, ttl=10000 → expiry=11000 > now=1000
        let vf = make_tending(Classification::ValuesForward, vec![1_000], 10_000);
        record_tending(&mut conn, "cid-values-forward", &[0xBBu8; 32], &vf).expect("insert");

        // Expired ScopeMismatch: last_tended=0, ttl=5 → expiry=5 < now=1000
        let sm = make_tending(Classification::ScopeMismatch, vec![0], 5);
        record_tending(&mut conn, "cid-scope-mismatch", &[0xCCu8; 32], &sm).expect("insert");

        // Safety with ttl=1 and last_tended=0: MUST survive because Safety is never swept.
        let safety = make_tending(Classification::Safety, vec![0], 3_600);
        record_tending(&mut conn, "cid-safety", &[0xDDu8; 32], &safety).expect("insert");

        let deleted = enforce_ttls(&mut conn, 1_000).expect("enforce");
        assert_eq!(deleted, 2, "only Fatigue + ScopeMismatch should be deleted");

        // Safety must survive.
        assert!(
            fetch_by_cid(&mut conn, "cid-safety")
                .expect("fetch safety")
                .is_some(),
            "Safety row must not be swept"
        );

        // ValuesForward must survive.
        assert!(
            fetch_by_cid(&mut conn, "cid-values-forward")
                .expect("fetch values-forward")
                .is_some(),
            "Alive ValuesForward row must not be swept"
        );

        // Expired rows must be gone.
        assert!(
            fetch_by_cid(&mut conn, "cid-fatigue")
                .expect("fetch")
                .is_none(),
            "Expired Fatigue row must be deleted"
        );
        assert!(
            fetch_by_cid(&mut conn, "cid-scope-mismatch")
                .expect("fetch")
                .is_none(),
            "Expired ScopeMismatch row must be deleted"
        );
    }

    // -----------------------------------------------------------------------
    // T5: record_tending rejects empty tended_at
    // -----------------------------------------------------------------------

    #[test]
    fn record_tending_rejects_empty_tended_at() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let tending = make_tending(Classification::Fatigue, vec![], 3_600);
        let result = record_tending(&mut conn, "cid-empty", &[0xEEu8; 32], &tending);

        assert!(result.is_err(), "empty tended_at must return Err");
        match result.unwrap_err() {
            TendingError::Invalid(msg) => assert!(
                msg.contains("tended_at"),
                "error message must mention tended_at, got: {msg}"
            ),
            other => panic!("expected TendingError::Invalid, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // T6: record_tending rejects ttl_seconds > i64::MAX
    // -----------------------------------------------------------------------

    #[test]
    fn record_tending_rejects_ttl_exceeding_i64_max() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // u64::MAX is well above i64::MAX; the `as i64` cast would wrap to -1,
        // causing the row to expire immediately on first sweep.
        let mut t = make_tending(Classification::Fatigue, vec![1_000_000], 3_600);
        t.ttl_seconds = u64::MAX;
        let result = record_tending(&mut conn, "cid-overflow", &[0u8; 32], &t);

        assert!(
            matches!(result, Err(TendingError::Invalid(_))),
            "ttl_seconds > i64::MAX must return TendingError::Invalid, got: {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // T16 — sweep_expired tests (wall-clock sweep, safety floor protection)
    //
    // Timestamps use epoch seconds (matching last_tended_at column precision).
    // "Now" is approximated by inserting rows with timestamps well in the past
    // (expired) or well in the future (alive), then sweeping.
    // -----------------------------------------------------------------------

    fn test_pool_sweep() -> DbPool {
        let url = format!(
            "file:svc_tending_sweep_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    // Helper: get current epoch seconds
    fn now_secs() -> i64 {
        chrono::Utc::now().timestamp()
    }

    #[test]
    fn sweep_deletes_expired_non_safety_rows() {
        let pool = test_pool_sweep();
        let mut conn = pool.get().unwrap();
        let now = now_secs();

        // Expired fatigue: last_tended = 2 * ttl ago → expiry is well past.
        insert_raw(
            &mut conn,
            "sweep-filter1",
            "fatigue",
            3_600,
            &[now - 7_200],
            "{}",
        )
        .unwrap();
        // Alive values-forward: last_tended recent → expiry is far in future.
        insert_raw(
            &mut conn,
            "sweep-filter2",
            "values-forward",
            86_400,
            &[now - 100],
            "{}",
        )
        .unwrap();
        // Safety: large ttl_seconds but old timestamp — must NOT be deleted.
        insert_raw(
            &mut conn,
            "sweep-filter3",
            "safety",
            3_600,
            &[now - 1_000_000],
            "{}",
        )
        .unwrap();

        let deleted = sweep_expired(&mut conn).expect("sweep");
        assert_eq!(deleted, 1, "only the expired fatigue row should be deleted");

        let remaining = list_all(&mut conn).unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(
            remaining.iter().any(|r| r.classification == "safety"),
            "safety row must survive"
        );
        assert!(
            remaining
                .iter()
                .any(|r| r.classification == "values-forward"),
            "alive values-forward must survive"
        );
    }

    #[test]
    fn sweep_is_idempotent() {
        let pool = test_pool_sweep();
        let mut conn = pool.get().unwrap();
        let now = now_secs();

        // Insert one expired row.
        insert_raw(
            &mut conn,
            "idempotent-filter1",
            "fatigue",
            3_600,
            &[now - 7_200],
            "{}",
        )
        .unwrap();

        let first = sweep_expired(&mut conn).expect("first sweep");
        assert_eq!(first, 1, "first sweep must delete the expired row");

        let second = sweep_expired(&mut conn).expect("second sweep");
        assert_eq!(second, 0, "second sweep must be a no-op");
    }

    #[test]
    fn sweep_never_deletes_safety_classification() {
        let pool = test_pool_sweep();
        let mut conn = pool.get().unwrap();
        let now = now_secs();

        // Safety row with ttl=1 second, last_tended very far in the past.
        // Would expire if sweep ignored the classification guard.
        insert_raw(
            &mut conn,
            "safety-floor",
            "safety",
            1,
            &[now - 1_000_000_000],
            "{}",
        )
        .unwrap();

        sweep_expired(&mut conn).unwrap();

        let remaining = list_all(&mut conn).unwrap();
        assert_eq!(remaining.len(), 1, "safety row must not be deleted");
        assert_eq!(remaining[0].classification, "safety");
    }
}
