//! attention_tending — local cache mirror of source-chain AttentionTending entries.
//!
//! Category C (operational) per P2P design gate. The DHT-side authoritative
//! entry is the Holochain `AttentionTending` HDI integrity entry on the
//! signing agent's source chain (Visibility::Private — see T5/T9). This
//! local table is a fast-read cache used by the aggregator (T16) and the
//! TTL sweep (services::tending::enforce_ttls).
//!
//! Recomputable from the source-chain subgraph at any time.

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::attention_tending;

// ============================================================================
// Row types
// ============================================================================

/// Full row returned by SELECT queries.
#[derive(Debug, Clone, Queryable, Selectable, AsChangeset)]
#[diesel(table_name = attention_tending)]
pub struct TendingRow {
    pub id: i32,
    pub tending_cid: String,
    pub signer_pubkey: Vec<u8>,
    pub classification: String,
    pub filter_subject_json: String,
    pub context_json: String,
    pub reason: Option<String>,
    /// TTL in seconds (i64::MAX for Safety — never swept).
    pub ttl_seconds: i64,
    /// Unix epoch seconds — first tended_at timestamp (denormalized).
    pub created_at: i64,
    /// Unix epoch seconds — most recent tended_at timestamp (denormalized for fast sweep).
    pub last_tended_at: i64,
    /// JSON-serialized Vec<u64> of all tended_at timestamps in order.
    pub tended_at_history_json: String,
}

/// Insertable row (no `id` — AUTOINCREMENT).
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = attention_tending)]
pub struct NewTendingRow {
    pub tending_cid: String,
    pub signer_pubkey: Vec<u8>,
    pub classification: String,
    pub filter_subject_json: String,
    pub context_json: String,
    pub reason: Option<String>,
    pub ttl_seconds: i64,
    pub created_at: i64,
    pub last_tended_at: i64,
    pub tended_at_history_json: String,
}

// ============================================================================
// Queries
// ============================================================================

/// Insert a new tending row. Returns the AUTOINCREMENT id of the inserted row.
///
/// Errors with `DatabaseError(UniqueViolation, ..)` if `tending_cid` already
/// exists — the caller (services::tending) should use `fetch_by_cid` to check
/// for idempotent inserts if needed.
pub fn insert(conn: &mut SqliteConnection, row: &NewTendingRow) -> Result<i32, DieselError> {
    use attention_tending::dsl;
    diesel::insert_into(dsl::attention_tending)
        .values(row)
        .execute(conn)?;
    // SQLite last_insert_rowid() via diesel
    dsl::attention_tending
        .select(dsl::id)
        .order_by(dsl::id.desc())
        .first(conn)
}

/// Fetch a single row by its source-chain CID. Returns `None` if not cached.
pub fn fetch_by_cid(
    conn: &mut SqliteConnection,
    tending_cid: &str,
) -> Result<Option<TendingRow>, DieselError> {
    use attention_tending::dsl;
    dsl::attention_tending
        .filter(dsl::tending_cid.eq(tending_cid))
        .first::<TendingRow>(conn)
        .optional()
}

/// List all cached tending rows for the given signer public key.
pub fn list_by_signer(
    conn: &mut SqliteConnection,
    signer: &[u8],
) -> Result<Vec<TendingRow>, DieselError> {
    use attention_tending::dsl;
    dsl::attention_tending
        .filter(dsl::signer_pubkey.eq(signer))
        .order_by(dsl::created_at.asc())
        .load::<TendingRow>(conn)
}

/// List all cached tending rows for the given classification string (e.g. "safety").
pub fn list_by_classification(
    conn: &mut SqliteConnection,
    classification: &str,
) -> Result<Vec<TendingRow>, DieselError> {
    use attention_tending::dsl;
    dsl::attention_tending
        .filter(dsl::classification.eq(classification))
        .order_by(dsl::created_at.asc())
        .load::<TendingRow>(conn)
}

/// Append `new_tended_at` to the history and update `last_tended_at`.
///
/// Deserializes the existing `tended_at_history_json`, appends the new
/// timestamp, re-serializes, and updates the row. Returns the updated row.
pub fn refresh(
    conn: &mut SqliteConnection,
    id: i32,
    new_tended_at: i64,
) -> Result<TendingRow, DieselError> {
    use attention_tending::dsl;

    // Fetch current row to read history.
    let current: TendingRow = dsl::attention_tending.filter(dsl::id.eq(id)).first(conn)?;

    // Extend the history. Log a warning if the stored JSON is corrupt rather
    // than silently discarding it — same observability principle as T14
    // corrupt-score handling.
    let mut history: Vec<i64> = match serde_json::from_str(&current.tended_at_history_json) {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(
                tending_id = current.id,
                error = %e,
                history_len = current.tended_at_history_json.len(),
                "corrupt tended_at_history_json in attention_tending row, defaulting to empty"
            );
            Vec::new()
        }
    };
    history.push(new_tended_at);
    let new_history_json =
        serde_json::to_string(&history).unwrap_or_else(|_| current.tended_at_history_json.clone());

    diesel::update(dsl::attention_tending.filter(dsl::id.eq(id)))
        .set((
            dsl::last_tended_at.eq(new_tended_at),
            dsl::tended_at_history_json.eq(&new_history_json),
        ))
        .execute(conn)?;

    dsl::attention_tending.filter(dsl::id.eq(id)).first(conn)
}

/// List tending rows whose `last_tended_at >= window_start`.
///
/// Used by the aggregator (T16) to compute classification counts within a
/// context window. Results are ordered by `last_tended_at` ascending so the
/// aggregator can reliably pick the most-recent representative row.
pub fn list_recent_tending(
    conn: &mut SqliteConnection,
    window_start: i64,
) -> Result<Vec<TendingRow>, DieselError> {
    use attention_tending::dsl;
    dsl::attention_tending
        .filter(dsl::last_tended_at.ge(window_start))
        .order_by(dsl::last_tended_at.asc())
        .load::<TendingRow>(conn)
}

/// Delete rows where `last_tended_at + ttl_seconds < now` AND classification != 'safety'.
///
/// Safety classification rows are NEVER swept regardless of `ttl_seconds`.
/// Returns the count of rows deleted.
pub fn delete_expired(conn: &mut SqliteConnection, now: i64) -> Result<usize, DieselError> {
    use attention_tending::dsl;
    diesel::delete(
        dsl::attention_tending
            .filter(dsl::classification.ne("safety"))
            .filter((dsl::last_tended_at + dsl::ttl_seconds).lt(now)),
    )
    .execute(conn)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:tending_db_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn make_row(cid: &str, classification: &str, last_tended_at: i64, ttl: i64) -> NewTendingRow {
        NewTendingRow {
            tending_cid: cid.to_string(),
            signer_pubkey: vec![0xAAu8; 32],
            classification: classification.to_string(),
            filter_subject_json: r#"{"contentKind":"concept"}"#.to_string(),
            context_json: r#"{"collective":"household"}"#.to_string(),
            reason: None,
            ttl_seconds: ttl,
            created_at: 100,
            last_tended_at,
            tended_at_history_json: serde_json::to_string(&vec![last_tended_at]).unwrap(),
        }
    }

    #[test]
    fn insert_and_fetch_by_cid_roundtrip() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let row = NewTendingRow {
            tending_cid: "uhCkkTestCid001".to_string(),
            signer_pubkey: vec![0x11u8; 32],
            classification: "fatigue".to_string(),
            filter_subject_json: r#"{"topic":"history"}"#.to_string(),
            context_json: r#"{"mode":"study"}"#.to_string(),
            reason: Some("too much history content".to_string()),
            ttl_seconds: 604_800,
            created_at: 1_746_000_000,
            last_tended_at: 1_746_000_000,
            tended_at_history_json: "[1746000000]".to_string(),
        };

        let id = insert(&mut conn, &row).expect("insert");
        assert!(id > 0, "returned id should be positive");

        let fetched = fetch_by_cid(&mut conn, "uhCkkTestCid001")
            .expect("fetch")
            .expect("should be Some");

        assert_eq!(fetched.tending_cid, "uhCkkTestCid001");
        assert_eq!(fetched.signer_pubkey, vec![0x11u8; 32]);
        assert_eq!(fetched.classification, "fatigue");
        assert_eq!(fetched.filter_subject_json, r#"{"topic":"history"}"#);
        assert_eq!(fetched.context_json, r#"{"mode":"study"}"#);
        assert_eq!(fetched.reason, Some("too much history content".to_string()));
        assert_eq!(fetched.ttl_seconds, 604_800);
        assert_eq!(fetched.created_at, 1_746_000_000);
        assert_eq!(fetched.last_tended_at, 1_746_000_000);
        assert_eq!(fetched.tended_at_history_json, "[1746000000]");
    }

    #[test]
    fn fetch_by_cid_returns_none_when_absent() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let result = fetch_by_cid(&mut conn, "does-not-exist").expect("fetch");
        assert!(result.is_none(), "unknown CID should return None");
    }

    #[test]
    fn refresh_updates_last_tended_at_and_history() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let row = make_row("uhCkkRefreshCid", "values-forward", 1_000, 2_592_000);
        let id = insert(&mut conn, &row).expect("insert");

        let updated = refresh(&mut conn, id, 2_000).expect("refresh");
        assert_eq!(
            updated.last_tended_at, 2_000,
            "last_tended_at must be updated"
        );

        let history: Vec<i64> =
            serde_json::from_str(&updated.tended_at_history_json).expect("parse history");
        assert_eq!(history.len(), 2, "history should have 2 entries");
        assert_eq!(history[0], 1_000, "first entry preserved");
        assert_eq!(history[1], 2_000, "new entry appended");
    }

    #[test]
    fn list_recent_tending_filters_by_window_start() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Row within window: last_tended_at = 1_000_000
        insert(
            &mut conn,
            &make_row("cid-recent-1", "fatigue", 1_000_000, 604_800),
        )
        .expect("insert recent-1");
        insert(
            &mut conn,
            &make_row("cid-recent-2", "values-forward", 1_100_000, 604_800),
        )
        .expect("insert recent-2");

        // Row outside window: last_tended_at = 100_000 (below window_start = 900_000)
        insert(&mut conn, &make_row("cid-old", "safety", 100_000, i64::MAX)).expect("insert old");

        let rows = list_recent_tending(&mut conn, 900_000).expect("list_recent");
        assert_eq!(rows.len(), 2, "only rows with last_tended_at >= 900_000");

        let cids: Vec<&str> = rows.iter().map(|r| r.tending_cid.as_str()).collect();
        assert!(cids.contains(&"cid-recent-1"));
        assert!(cids.contains(&"cid-recent-2"));
        assert!(!cids.contains(&"cid-old"), "old row must be excluded");
    }

    #[test]
    fn delete_expired_removes_non_safety_expired_rows() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Expired fatigue: last_tended=0, ttl=10, expires at 10 < now=1000
        insert(
            &mut conn,
            &make_row("cid-fatigue-expired", "fatigue", 0, 10),
        )
        .expect("insert");
        // Alive values-forward: last_tended=now, ttl=10000, expires at 11000 > now=1000
        insert(
            &mut conn,
            &make_row("cid-values-alive", "values-forward", 1_000, 10_000),
        )
        .expect("insert");
        // Expired scope-mismatch: last_tended=0, ttl=5, expires at 5 < now=1000
        insert(
            &mut conn,
            &make_row("cid-scope-expired", "scope-mismatch", 0, 5),
        )
        .expect("insert");
        // Safety with ttl=1 and last_tended=0: MUST survive despite ttl=1 < now=1000
        insert(&mut conn, &make_row("cid-safety-survives", "safety", 0, 1)).expect("insert");

        let deleted = delete_expired(&mut conn, 1_000).expect("delete_expired");
        assert_eq!(
            deleted, 2,
            "exactly 2 rows (fatigue + scope-mismatch) should be deleted"
        );

        // Safety row must still exist.
        let safety = fetch_by_cid(&mut conn, "cid-safety-survives")
            .expect("fetch safety")
            .expect("safety must still exist");
        assert_eq!(safety.classification, "safety");

        // Alive values-forward row must still exist.
        let alive = fetch_by_cid(&mut conn, "cid-values-alive")
            .expect("fetch alive")
            .expect("alive values-forward must still exist");
        assert_eq!(alive.classification, "values-forward");

        // Expired rows must be gone.
        assert!(
            fetch_by_cid(&mut conn, "cid-fatigue-expired")
                .expect("fetch")
                .is_none(),
            "expired fatigue should be deleted"
        );
        assert!(
            fetch_by_cid(&mut conn, "cid-scope-expired")
                .expect("fetch")
                .is_none(),
            "expired scope-mismatch should be deleted"
        );
    }
}
