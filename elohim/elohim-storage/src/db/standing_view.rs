//! standing_view — per-evaluator projected StandingScore.
//!
//! Category C (operational) per P2P design gate. Recomputable from the
//! FeedbackSignal subgraph; this table is a local cache, not a stored
//! authoritative score (per brainstorm §4.2 pluralism property).
//!
//! Updated by `services::standing_projector::project_signal` on FeedbackSignal
//! arrival. Read by `services::standing::Standing::evaluate`.

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::standing_view;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = standing_view)]
pub struct StandingViewRow {
    pub evaluator_pubkey: Vec<u8>,
    pub subject_pubkey: Vec<u8>,
    pub score: String,          // serialized StandingScore variant
    pub debit_weight_sum: i32,  // SQLite INTEGER → i32 default; widen later if needed
    pub last_signal_at: String, // RFC3339 UTC
    pub manifest_cid: String,
}

pub fn fetch(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
    subject: &[u8],
) -> Result<Option<StandingViewRow>, DieselError> {
    use crate::db::diesel_schema::standing_view::dsl;
    dsl::standing_view
        .filter(dsl::evaluator_pubkey.eq(evaluator))
        .filter(dsl::subject_pubkey.eq(subject))
        .first::<StandingViewRow>(conn)
        .optional()
}

pub fn upsert(conn: &mut SqliteConnection, row: &StandingViewRow) -> Result<(), DieselError> {
    use crate::db::diesel_schema::standing_view::dsl;
    diesel::insert_into(dsl::standing_view)
        .values(row)
        .on_conflict((dsl::evaluator_pubkey, dsl::subject_pubkey))
        .do_update()
        .set(row)
        .execute(conn)?;
    Ok(())
}

pub fn list_subjects_for_evaluator(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
) -> Result<Vec<StandingViewRow>, DieselError> {
    use crate::db::diesel_schema::standing_view::dsl;
    dsl::standing_view
        .filter(dsl::evaluator_pubkey.eq(evaluator))
        .order_by(dsl::last_signal_at.asc())
        .load::<StandingViewRow>(conn)
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
            "file:sv_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn eval() -> Vec<u8> {
        vec![0xEAu8; 32]
    }

    fn subj() -> Vec<u8> {
        vec![0x5Bu8; 32]
    }

    fn row(score: &str, sum: i32) -> StandingViewRow {
        StandingViewRow {
            evaluator_pubkey: eval(),
            subject_pubkey: subj(),
            score: score.to_string(),
            debit_weight_sum: sum,
            last_signal_at: "2026-05-01T04:00:00Z".to_string(),
            manifest_cid: "bafyreimanifest".to_string(),
        }
    }

    #[test]
    fn fetch_returns_none_on_empty_db() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let result = fetch(&mut conn, &eval(), &subj()).expect("fetch");
        assert!(result.is_none(), "empty db should return None");
    }

    #[test]
    fn upsert_then_fetch_roundtrip() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let r = row("neutral", 0);
        upsert(&mut conn, &r).expect("upsert");

        let fetched = fetch(&mut conn, &eval(), &subj())
            .expect("fetch")
            .expect("should be Some");
        assert_eq!(fetched.score, "neutral");
        assert_eq!(fetched.debit_weight_sum, 0);
        assert_eq!(fetched.manifest_cid, "bafyreimanifest");
    }

    #[test]
    fn upsert_updates_existing_row() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        upsert(&mut conn, &row("neutral", 0)).expect("first upsert");
        upsert(&mut conn, &row("low", 5)).expect("second upsert");

        let fetched = fetch(&mut conn, &eval(), &subj())
            .expect("fetch")
            .expect("should be Some");
        assert_eq!(fetched.score, "low", "second upsert must overwrite first");
        assert_eq!(fetched.debit_weight_sum, 5);
    }

    #[test]
    fn list_subjects_for_evaluator_returns_all() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Two subjects for same evaluator.
        let subj2 = vec![0xBBu8; 32];
        let r1 = StandingViewRow {
            evaluator_pubkey: eval(),
            subject_pubkey: subj(),
            score: "neutral".to_string(),
            debit_weight_sum: 0,
            last_signal_at: "2026-05-01T04:00:00Z".to_string(),
            manifest_cid: "bafyreimanifest".to_string(),
        };
        let r2 = StandingViewRow {
            evaluator_pubkey: eval(),
            subject_pubkey: subj2.clone(),
            score: "low".to_string(),
            debit_weight_sum: 4,
            last_signal_at: "2026-05-01T05:00:00Z".to_string(),
            manifest_cid: "bafyreimanifest".to_string(),
        };
        upsert(&mut conn, &r1).expect("upsert r1");
        upsert(&mut conn, &r2).expect("upsert r2");

        let rows = list_subjects_for_evaluator(&mut conn, &eval()).expect("list");
        assert_eq!(rows.len(), 2, "both subjects should appear");
        let scores: Vec<&str> = rows.iter().map(|r| r.score.as_str()).collect();
        assert!(scores.contains(&"neutral"));
        assert!(scores.contains(&"low"));
    }
}
