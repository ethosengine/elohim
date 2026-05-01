//! Author-side compose-time StandingQuery API — Phase 3.5 P3.5.9.
//!
//! Read-only. Cheap synchronous query the elohim tender invokes at content
//! compose time to surface: the author's current standing in the local
//! evaluator's view, recent fatigue signals on this author, and the
//! constitutional floor classes. Drives the tender's "do you really want to
//! send this?" conversation without round-tripping to the DHT.
//!
//! Performance target: <50ms p99 at 1000 standing_view rows + 100 tending
//! rows. CI ceiling: 200ms (slow-runner safety margin).
//!
//! See: brainstorm §4.3 compose-time guidance.

use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use crate::db::standing_view::fetch as fetch_standing;
use crate::db::tending::list_recent_tending;
use crate::services::standing::Standing;
use crate::services::standing_projector::deserialize_score;

// ============================================================================
// Types
// ============================================================================

/// Floor class identifier (mirrors the standing-policy-floor schema enum).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StandingFloorClass {
    LocalRelationshipReach,
    CidTargetedLookup,
    ConstitutionalFloorSignatures,
    NewVoiceBaseline,
    VulnerableClassElevation,
}

impl StandingFloorClass {
    pub fn all() -> Vec<Self> {
        vec![
            Self::LocalRelationshipReach,
            Self::CidTargetedLookup,
            Self::ConstitutionalFloorSignatures,
            Self::NewVoiceBaseline,
            Self::VulnerableClassElevation,
        ]
    }
}

/// A summarized fatigue tending signal — what the local evaluator's tending
/// records say about this author. Only `fatigue` and `safety` classification
/// rows are summarized; other classifications drive different code paths.
///
/// Inclusion criteria:
/// - classification is `"fatigue"` or `"safety"`
/// - signer_pubkey is either `subject` (self-tended) or `evaluator`
///   (the local peer's own records about this author)
/// - last_tended_at >= window_start (30-day rolling window)
///
/// The "fatigue + safety" pair is the "did this author trigger any local
/// discernment" pair relevant to compose-time prompting. ValuesForward and
/// ScopeMismatch are author-side self-discernment and are not relevant to the
/// "do you really want to send this?" conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FatigueSignal {
    pub classification: String,
    pub last_tended_at: i64,
    pub ttl_seconds: i64,
    pub reason: Option<String>,
}

/// Compose-time context for an author/subject pair. Returned by the
/// `/api/v1/standing/compose-context` endpoint and consumed by the elohim
/// tender to phrase compose-time guidance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeContext {
    pub author_standing: Standing,
    pub fatigue_signals: Vec<FatigueSignal>,
    pub floor_classes: Vec<StandingFloorClass>,
}

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum StandingQueryError {
    #[error("database: {0}")]
    Database(#[from] diesel::result::Error),
}

// ============================================================================
// Service function (read-only)
// ============================================================================

/// Cheap read of compose-time context. Read-only — never writes.
///
/// `evaluator` is the local agent's pubkey (the viewer); `subject` is the
/// author the elohim tender is evaluating. The two MAY be equal (self-tend)
/// in which case the function returns the local agent's view of itself.
///
/// Returns `Unknown` standing and empty signals on cold-start (no rows yet).
pub fn compose_context(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
    subject: &[u8],
) -> Result<ComposeContext, StandingQueryError> {
    // Standing read — Unknown for cold-start (no projection row yet).
    let standing = match fetch_standing(conn, evaluator, subject)? {
        Some(row) => match deserialize_score(&row.score) {
            Some(score) => Standing::Computed { score },
            None => Standing::Unknown,
        },
        None => Standing::Unknown,
    };

    // Tending read — recent rows from the local cache.
    //
    // Window: last 30 days. Beyond that, signals fade and are not actionable
    // at compose time.
    //
    // Inclusion criteria:
    //   1. classification is "fatigue" or "safety" — these are the "did this
    //      author trigger local discernment" pair that the tender needs to see.
    //   2. signer_pubkey is either the subject (self-tended) OR the evaluator
    //      (things *you* have personally flagged about this author).
    //
    // ValuesForward and ScopeMismatch are author-side self-discernment and are
    // NOT relevant to the compose-time "do you really want to send this?" prompt.
    let now = chrono::Utc::now().timestamp();
    let window_start = now - (30 * 86_400);
    let tending_rows = list_recent_tending(conn, window_start)?;
    let fatigue_signals: Vec<FatigueSignal> = tending_rows
        .into_iter()
        .filter(|row| matches!(row.classification.as_str(), "fatigue" | "safety"))
        .filter(|row| row.signer_pubkey == subject || row.signer_pubkey == evaluator)
        .map(|row| FatigueSignal {
            classification: row.classification,
            last_tended_at: row.last_tended_at,
            ttl_seconds: row.ttl_seconds,
            reason: row.reason,
        })
        .collect();

    Ok(ComposeContext {
        author_standing: standing,
        fatigue_signals,
        floor_classes: StandingFloorClass::all(),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tending::NewTendingRow;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sqlite::SqliteConnection;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:sq_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool")
    }

    fn migrated_pool() -> DbPool {
        let pool = test_pool();
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// Evaluator pubkey bytes (local peer).
    fn evaluator() -> Vec<u8> {
        vec![0xE0u8; 32]
    }

    /// Subject pubkey bytes (the author being evaluated).
    fn subject() -> Vec<u8> {
        vec![0xABu8; 32]
    }

    /// An unrelated third-party signer.
    fn other() -> Vec<u8> {
        vec![0xCCu8; 32]
    }

    fn now_ts() -> i64 {
        chrono::Utc::now().timestamp()
    }

    /// Insert a tending row. Returns the inserted id.
    fn insert_tending(
        conn: &mut SqliteConnection,
        cid: &str,
        signer: &[u8],
        classification: &str,
        last_tended_at: i64,
        ttl: i64,
        reason: Option<&str>,
    ) -> i32 {
        crate::db::tending::insert(
            conn,
            &NewTendingRow {
                tending_cid: cid.to_string(),
                signer_pubkey: signer.to_vec(),
                classification: classification.to_string(),
                filter_subject_json: r#"{"topic":"any"}"#.to_string(),
                context_json: r#"{"mode":"test"}"#.to_string(),
                reason: reason.map(|s| s.to_string()),
                ttl_seconds: ttl,
                created_at: last_tended_at,
                last_tended_at,
                tended_at_history_json: format!("[{}]", last_tended_at),
            },
        )
        .expect("insert tending")
    }

    /// Insert a standing_view row via the standing_projector.
    fn insert_standing(conn: &mut SqliteConnection, evaluator: &[u8], subject: &[u8], score: &str) {
        use crate::db::standing_view::{upsert, StandingViewRow};
        upsert(
            conn,
            &StandingViewRow {
                evaluator_pubkey: evaluator.to_vec(),
                subject_pubkey: subject.to_vec(),
                score: score.to_string(),
                debit_weight_sum: 0,
                last_signal_at: "2026-05-01T00:00:00Z".to_string(),
                manifest_cid: "bafyreitest".to_string(),
            },
        )
        .expect("upsert standing");
    }

    // T1 — cold-start: fresh DB, no rows
    #[test]
    fn compose_context_cold_start_returns_unknown_standing() {
        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("compose_context");

        assert_eq!(
            ctx.author_standing,
            Standing::Unknown,
            "cold-start must return Unknown standing"
        );
        assert!(
            ctx.fatigue_signals.is_empty(),
            "cold-start must return no fatigue signals"
        );
        assert_eq!(
            ctx.floor_classes.len(),
            5,
            "must always return all 5 floor classes"
        );
    }

    // T2 — returns Computed standing when projection exists
    #[test]
    fn compose_context_returns_computed_standing_when_projection_exists() {
        use crate::services::standing::StandingScore;
        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");

        insert_standing(&mut conn, &evaluator(), &subject(), "low");
        let ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("compose_context");

        assert_eq!(
            ctx.author_standing,
            Standing::Computed {
                score: StandingScore::Low
            },
            "should return Computed(Low) after projection insert"
        );
    }

    // T3 — includes fatigue signals for the subject (self-tended)
    #[test]
    fn compose_context_includes_fatigue_signals_for_subject() {
        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");
        let ts = now_ts();

        insert_tending(
            &mut conn,
            "cid-fatigue-subject",
            &subject(),
            "fatigue",
            ts,
            604_800,
            Some("too much history"),
        );

        let ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("compose_context");

        assert_eq!(
            ctx.fatigue_signals.len(),
            1,
            "should include 1 fatigue signal"
        );
        assert_eq!(ctx.fatigue_signals[0].classification, "fatigue");
        assert_eq!(
            ctx.fatigue_signals[0].reason,
            Some("too much history".to_string())
        );
    }

    // T3b — includes fatigue signals signed by the evaluator (peer-tended about the subject)
    #[test]
    fn compose_context_includes_fatigue_signals_signed_by_evaluator() {
        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");
        let ts = now_ts();

        insert_tending(
            &mut conn,
            "cid-fatigue-evaluator",
            &evaluator(),
            "safety",
            ts,
            i64::MAX,
            None,
        );

        let ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("compose_context");

        assert_eq!(
            ctx.fatigue_signals.len(),
            1,
            "should include evaluator-signed safety signal"
        );
        assert_eq!(ctx.fatigue_signals[0].classification, "safety");
    }

    // T4 — excludes ValuesForward and ScopeMismatch classifications
    #[test]
    fn compose_context_excludes_unrelated_classifications() {
        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");
        let ts = now_ts();

        insert_tending(
            &mut conn,
            "cid-values-forward",
            &subject(),
            "values-forward",
            ts,
            604_800,
            None,
        );
        insert_tending(
            &mut conn,
            "cid-scope-mismatch",
            &subject(),
            "scope-mismatch",
            ts,
            604_800,
            None,
        );

        let ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("compose_context");

        assert!(
            ctx.fatigue_signals.is_empty(),
            "values-forward and scope-mismatch must NOT appear in fatigue_signals"
        );
    }

    // T5 — excludes rows outside the 30-day window
    #[test]
    fn compose_context_excludes_rows_outside_30day_window() {
        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");
        let now = now_ts();
        let thirty_one_days_ago = now - (31 * 86_400);

        insert_tending(
            &mut conn,
            "cid-old-fatigue",
            &subject(),
            "fatigue",
            thirty_one_days_ago,
            604_800,
            None,
        );

        let ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("compose_context");

        assert!(
            ctx.fatigue_signals.is_empty(),
            "row older than 30 days must NOT appear in fatigue_signals"
        );
    }

    // T6 — always returns exactly 5 floor classes
    #[test]
    fn compose_context_returns_all_five_floor_classes_always() {
        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");

        // Cold-start
        let ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("cold-start");
        assert_eq!(
            ctx.floor_classes.len(),
            5,
            "cold-start must have 5 floor classes"
        );

        // After population
        insert_standing(&mut conn, &evaluator(), &subject(), "high");
        let ts = now_ts();
        insert_tending(
            &mut conn,
            "cid-fatigue-fc",
            &subject(),
            "fatigue",
            ts,
            604_800,
            None,
        );
        let ctx2 = compose_context(&mut conn, &evaluator(), &subject()).expect("populated");
        assert_eq!(
            ctx2.floor_classes.len(),
            5,
            "populated state must still have 5 floor classes"
        );
    }

    // T7 — excludes signals signed by an unrelated third party
    // (only subject-signed or evaluator-signed rows are included)
    #[test]
    fn compose_context_excludes_unrelated_signer() {
        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");
        let ts = now_ts();

        insert_tending(
            &mut conn,
            "cid-third-party-fatigue",
            &other(),
            "fatigue",
            ts,
            604_800,
            None,
        );

        let ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("compose_context");

        assert!(
            ctx.fatigue_signals.is_empty(),
            "fatigue row from unrelated signer must NOT appear"
        );
    }

    // T8 — performance: <200ms at 1000 standing_view + 100 tending rows
    // (gated to release mode — debug Rust is much slower and the 200ms ceiling
    // is not meaningful for unoptimized builds)
    #[cfg_attr(debug_assertions, ignore = "release-mode benchmark")]
    #[test]
    fn compose_context_completes_under_200ms_at_realistic_scale() {
        use crate::db::standing_view::{upsert, StandingViewRow};
        use std::time::Instant;

        let pool = migrated_pool();
        let mut conn = pool.get().expect("conn");

        // Insert 1000 standing_view rows for different subjects.
        for i in 0u32..1000 {
            let subj: Vec<u8> = i.to_le_bytes().iter().cycle().take(32).copied().collect();
            upsert(
                &mut conn,
                &StandingViewRow {
                    evaluator_pubkey: evaluator(),
                    subject_pubkey: subj,
                    score: "neutral".to_string(),
                    debit_weight_sum: 0,
                    last_signal_at: "2026-05-01T00:00:00Z".to_string(),
                    manifest_cid: "bafyreitest".to_string(),
                },
            )
            .expect("upsert standing");
        }

        // Insert 100 tending rows (mix of fatigue/safety/other).
        let ts = now_ts();
        let classifications = ["fatigue", "safety", "values-forward", "scope-mismatch"];
        for i in 0u32..100 {
            let cls = classifications[(i as usize) % classifications.len()];
            let cid = format!("cid-perf-{}", i);
            let signer = if i % 3 == 0 { subject() } else { evaluator() };
            insert_tending(&mut conn, &cid, &signer, cls, ts, 604_800, None);
        }

        let start = Instant::now();
        let _ctx = compose_context(&mut conn, &evaluator(), &subject()).expect("compose_context");
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 200,
            "compose_context took {}ms — must complete in <200ms (CI ceiling)",
            elapsed.as_millis()
        );
    }
}
