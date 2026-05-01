//! K-anonymous tending aggregator — Phase 3.5 trust-compute gradient substrate.
//!
//! Category C (operational) per P2P design gate. Reads local `attention_tending`
//! cache rows (T15) within a context window, groups by classification, and emits
//! [`CollectiveFilterPatternCandidate`] records when participation meets the
//! k-threshold.
//!
//! ## Window semantics
//!
//! `list_recent_tending` uses `last_tended_at >= window_start` (inclusive lower
//! bound). A row whose `last_tended_at` equals exactly `window_start` IS included
//! in the aggregation. Future authors: do not change this to `>` without updating
//! all callers and their tests.
//!
//! ## Privacy invariant (constitutional)
//!
//! The emitted [`CollectiveFilterPatternCandidate`] MUST NOT contain any peer
//! identity. The struct mirrors the T6 `CollectiveFilterPattern` HDI integrity
//! entry shape exactly: no `signer_pubkey`, no `subject`, no agent-identifying
//! field. Enforced by `struct_no_peer_identity` test below.
//!
//! ## Phase 3.5 simplifications (documented, not hidden)
//!
//! - `trend` is always `"stable"` — window-over-window comparison deferred to Phase 4+.
//! - `representative_filter_subject` picks the most-recent row (set-cover / union
//!   sketch deferred to Phase 4+).
//! - Laplace noise uses a deterministic pseudo-random stub — real CSPRNG-backed DP
//!   deferred. `AddLaplaceNoise` mode is manifest-declared; `Suppress` is the default.
//! - Cross-peer privacy-preserving union sketch deferred to Phase 4+.
//!
//! ## DHT publication
//!
//! This aggregator returns candidates only. Publication to the DHT via the
//! Holochain coordinator (`create_collective_filter_pattern`) is downstream (T19+).
//!
//! Source: genesis/docs/plans/epr-phase-3-5 §P3.5.6

use std::collections::BTreeMap;

use diesel::sqlite::SqliteConnection;

use crate::db::tending::{list_recent_tending, TendingRow};

// ============================================================================
// Public types
// ============================================================================

/// Candidate `CollectiveFilterPattern` emitted by the local-peer aggregator.
///
/// Mirrors the T6 HDI integrity entry shape exactly. K-anonymity invariant:
/// NO peer identity fields (no `signer_pubkey`, no `subject`, no agent key).
/// Publication to the DHT is downstream (T19+).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectiveFilterPatternCandidate {
    /// Non-empty collective identifier.
    pub collective: String,
    /// JSON string describing the tended filter subject.
    /// Syntax-checked at aggregation time.
    pub filter_subject_json: String,
    /// One of `values-forward` / `fatigue` / `scope-mismatch` / `safety`.
    pub classification: String,
    /// Percentage of tending rows (within context window) that share this
    /// classification. Clamped to [0, 100].
    pub participating_pct: u8,
    /// Trend direction. Phase 3.5: always `"stable"` (window-over-window
    /// comparison deferred to Phase 4+).
    pub trend: String,
    /// Context window in seconds used for this aggregation. ≥ 3600.
    pub context_window_seconds: u64,
}

/// Configuration for [`aggregate_and_emit`].
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    /// K-anonymity threshold: minimum number of tending records (within context
    /// window) required in a classification before a candidate is emitted.
    /// Default 5 per brainstorm §6.4.
    pub k_threshold: u8,
    /// Context window in seconds. Records older than `(now - window)` are
    /// excluded. Minimum 3600 (matches T6 floor).
    pub context_window_seconds: u64,
    /// What to do when participation < k_threshold.
    pub below_threshold_mode: BelowThresholdMode,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            k_threshold: 5,
            context_window_seconds: 86_400, // 1 day
            below_threshold_mode: BelowThresholdMode::Suppress,
        }
    }
}

/// Policy for classifications that fall below the k-threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BelowThresholdMode {
    /// Drop the candidate entirely (default — no signal beats noise signal).
    Suppress,
    /// Emit candidate with Laplace-noised `participating_pct`.
    ///
    /// Phase 3.5 stub — real DP requires CSPRNG and proper sensitivity
    /// analysis. The noise is deterministic given `seed` (the classification
    /// group count), which is sufficient for test coverage but NOT
    /// cryptographically sound. Manifests declare this mode when opted in.
    AddLaplaceNoise {
        /// Maximum noise magnitude in percentage points (clamped to scale_pct).
        scale_pct: u8,
    },
}

/// Errors returned by [`aggregate_and_emit`].
#[derive(Debug, thiserror::Error)]
pub enum AggregatorError {
    #[error("database: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("invalid: {0}")]
    Invalid(String),
}

// ============================================================================
// Public API
// ============================================================================

/// Aggregate local tending rows within `config.context_window_seconds` and
/// emit [`CollectiveFilterPatternCandidate`] records for classifications that
/// meet (or exceed) `config.k_threshold`.
///
/// # Parameters
///
/// - `conn` — mutable SQLite connection.
/// - `collective` — non-empty collective identifier written into each candidate.
/// - `config` — aggregation configuration.
/// - `now` — current unix timestamp in seconds (passed in for testability).
///
/// # Errors
///
/// - [`AggregatorError::Invalid`] if `collective` is empty or
///   `config.context_window_seconds < 3600`.
/// - [`AggregatorError::Database`] on any Diesel query failure.
pub fn aggregate_and_emit(
    conn: &mut SqliteConnection,
    collective: &str,
    config: &AggregatorConfig,
    now: i64,
) -> Result<Vec<CollectiveFilterPatternCandidate>, AggregatorError> {
    // 1. Validate inputs.
    if collective.is_empty() {
        return Err(AggregatorError::Invalid(
            "collective must be non-empty".into(),
        ));
    }
    if config.context_window_seconds < 3600 {
        return Err(AggregatorError::Invalid(
            "context_window_seconds must be >= 3600 (T6 floor)".into(),
        ));
    }

    // 2. Fetch all local tending rows within the context window.
    let window_start = now - config.context_window_seconds as i64;
    let rows = list_recent_tending(conn, window_start)?;

    let total_count = rows.len();

    // 3. Group by classification string (kebab-case, matching SQLite TEXT).
    // BTreeMap gives stable iteration order for deterministic output.
    let mut by_classification: BTreeMap<String, Vec<&TendingRow>> = BTreeMap::new();
    for row in &rows {
        by_classification
            .entry(row.classification.clone())
            .or_default()
            .push(row);
    }

    // 4. For each group, compute pct, apply k-threshold, build candidate.
    let mut candidates = Vec::new();

    for (classification, group) in &by_classification {
        let count = group.len();
        let pct = (count * 100).checked_div(total_count).unwrap_or(0).min(100) as u8;

        // Phase 3.5: trend is always "stable" — real trend needs
        // window-over-window comparison (deferred to Phase 4+).
        let candidate = CollectiveFilterPatternCandidate {
            collective: collective.to_string(),
            filter_subject_json: representative_filter_subject(group),
            classification: classification.clone(),
            participating_pct: pct,
            trend: "stable".to_string(),
            context_window_seconds: config.context_window_seconds,
        };

        let above_threshold = count >= config.k_threshold as usize;

        if above_threshold {
            candidates.push(candidate);
        } else {
            match config.below_threshold_mode {
                BelowThresholdMode::Suppress => {
                    // Drop — no signal beats a noise signal.
                }
                BelowThresholdMode::AddLaplaceNoise { scale_pct } => {
                    let noised = noise_pct(pct, scale_pct, count as u64);
                    let mut c = candidate;
                    c.participating_pct = noised;
                    candidates.push(c);
                }
            }
        }
    }

    Ok(candidates)
}

// ============================================================================
// Helpers
// ============================================================================

/// Return a representative `filter_subject_json` for the group.
///
/// Phase 3.5: returns the most-recent row's `filter_subject_json`. Real
/// aggregation (set-cover or union sketch) is Phase 4+ work.
fn representative_filter_subject(group: &[&TendingRow]) -> String {
    group
        .iter()
        .max_by_key(|r| r.last_tended_at)
        .map(|r| r.filter_subject_json.clone())
        .unwrap_or_else(|| "{}".to_string())
}

/// Deterministic pseudo-Laplace noise for `participating_pct`.
///
/// Phase 3.5 stub — real DP requires CSPRNG and proper sensitivity analysis.
/// Inputs `pct`, `scale_pct`, and `seed` (typically the group count or a
/// hash) produce a noised pct clamped to [0, 100]. Sign alternates by seed
/// parity. Magnitude is bounded by `scale_pct`.
fn noise_pct(pct: u8, scale_pct: u8, seed: u64) -> u8 {
    // Multiply by a Knuth hash constant to spread the seed, then mod by
    // (scale_pct + 1) so magnitude is in [0, scale_pct].
    let magnitude = (seed.wrapping_mul(2_654_435_761) % (scale_pct as u64 + 1)) as i32;
    let signed = if seed & 1 == 0 { magnitude } else { -magnitude };
    let result = pct as i32 + signed;
    result.clamp(0, 100) as u8
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tending::{insert as db_insert, NewTendingRow};
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sqlite::SqliteConnection;

    // -----------------------------------------------------------------------
    // Test infrastructure
    // -----------------------------------------------------------------------

    fn test_pool() -> DbPool {
        let url = format!(
            "file:aggregator_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// Insert `count` tending rows all sharing `classification` and
    /// `last_tended_at`. CIDs are made unique via classification + index +
    /// last_tended_at so that the same classification can be inserted at two
    /// different timestamps without CID collisions.
    fn insert_n(
        conn: &mut SqliteConnection,
        classification: &str,
        count: usize,
        last_tended_at: i64,
    ) {
        for i in 0..count {
            let cid = format!("cid-{}-{}-{}", classification, i, last_tended_at);
            db_insert(
                conn,
                &NewTendingRow {
                    tending_cid: cid,
                    signer_pubkey: vec![0xAAu8; 32],
                    classification: classification.to_string(),
                    filter_subject_json: r#"{"contentKind":"concept"}"#.to_string(),
                    context_json: r#"{"collective":"household"}"#.to_string(),
                    reason: None,
                    ttl_seconds: 604_800,
                    created_at: last_tended_at,
                    last_tended_at,
                    tended_at_history_json: serde_json::to_string(&vec![last_tended_at]).unwrap(),
                },
            )
            .expect("insert");
        }
    }

    // -----------------------------------------------------------------------
    // T1: default config uses brainstorm k=5
    // -----------------------------------------------------------------------

    #[test]
    fn default_config_uses_brainstorm_k_5() {
        let config = AggregatorConfig::default();
        assert_eq!(
            config.k_threshold, 5,
            "default k_threshold must be 5 per brainstorm §6.4"
        );
    }

    // -----------------------------------------------------------------------
    // T2: empty DB returns empty candidates
    // -----------------------------------------------------------------------

    #[test]
    fn empty_db_returns_empty_candidates() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let result = aggregate_and_emit(&mut conn, "household", &AggregatorConfig::default(), 0)
            .expect("aggregate");
        assert!(result.is_empty(), "no rows → no candidates");
    }

    // -----------------------------------------------------------------------
    // T3: below threshold with Suppress mode drops candidate
    // -----------------------------------------------------------------------

    #[test]
    fn below_threshold_suppress_drops_candidate() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let now = 1_000_000i64;
        // Insert 4 ValuesForward rows, all within the default 86_400s window.
        insert_n(&mut conn, "values-forward", 4, now);

        let config = AggregatorConfig {
            k_threshold: 5,
            context_window_seconds: 86_400,
            below_threshold_mode: BelowThresholdMode::Suppress,
        };

        let candidates =
            aggregate_and_emit(&mut conn, "household", &config, now).expect("aggregate");
        assert!(
            candidates.is_empty(),
            "count=4 < k=5 with Suppress must yield no candidates"
        );
    }

    // -----------------------------------------------------------------------
    // T4: at threshold — two classifications, each with pct = 50
    // -----------------------------------------------------------------------

    #[test]
    fn at_threshold_emits_candidate_with_correct_pct() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let now = 1_000_000i64;
        // 5 ValuesForward + 5 Fatigue = 10 total. Each pct = 50.
        insert_n(&mut conn, "values-forward", 5, now);
        insert_n(&mut conn, "fatigue", 5, now);

        let config = AggregatorConfig {
            k_threshold: 5,
            context_window_seconds: 86_400,
            below_threshold_mode: BelowThresholdMode::Suppress,
        };

        let candidates =
            aggregate_and_emit(&mut conn, "household", &config, now).expect("aggregate");

        assert_eq!(candidates.len(), 2, "both classifications meet k=5");

        // Both must have participating_pct = 50.
        for c in &candidates {
            assert_eq!(
                c.participating_pct, 50,
                "each of 2 equal groups must be 50%, got {:?}",
                c
            );
        }

        // Both must carry correct metadata.
        for c in &candidates {
            assert_eq!(c.collective, "household");
            assert_eq!(c.trend, "stable");
            assert_eq!(c.context_window_seconds, 86_400);
        }

        // Classification strings must be the two we inserted.
        let classifications: Vec<&str> = candidates
            .iter()
            .map(|c| c.classification.as_str())
            .collect();
        assert!(classifications.contains(&"values-forward"));
        assert!(classifications.contains(&"fatigue"));
    }

    // -----------------------------------------------------------------------
    // T5: above threshold for one, below for another
    // -----------------------------------------------------------------------

    #[test]
    fn above_threshold_emits_with_higher_pct() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let now = 1_000_000i64;
        // 8 Fatigue + 2 ValuesForward = 10 total.
        // Fatigue: 8 >= k=5 → emit pct=80.
        // ValuesForward: 2 < k=5 → suppress.
        insert_n(&mut conn, "fatigue", 8, now);
        insert_n(&mut conn, "values-forward", 2, now);

        let config = AggregatorConfig {
            k_threshold: 5,
            context_window_seconds: 86_400,
            below_threshold_mode: BelowThresholdMode::Suppress,
        };

        let candidates =
            aggregate_and_emit(&mut conn, "household", &config, now).expect("aggregate");

        assert_eq!(candidates.len(), 1, "only Fatigue (count=8) meets k=5");
        assert_eq!(candidates[0].classification, "fatigue");
        assert_eq!(candidates[0].participating_pct, 80);
    }

    // -----------------------------------------------------------------------
    // T6: below threshold with AddLaplaceNoise emits noised candidate
    // -----------------------------------------------------------------------

    #[test]
    fn below_threshold_laplace_mode_emits_with_noised_pct() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let now = 1_000_000i64;
        // 3 Safety rows (3 < k=5), scale_pct=10.
        // Raw pct = 3/3 * 100 = 100 (only classification in pool).
        // Wait — 3 Safety + nothing else → pct = 100. But the test asks for pct in [20,40].
        // Per spec: "3 Safety records" with expected pct in [max(0,30-10), min(100,30+10)].
        // 30% implies 3 out of 10. We need to add 7 more rows of a different class that
        // are BELOW threshold too (so they don't emit as a separate candidate).
        // Insert 3 Safety + 7 ValuesForward. scale_pct=10, k=5.
        // Safety count=3 < k=5, pct=30. AddLaplaceNoise{scale_pct:10} → pct in [20,40].
        // ValuesForward count=7 >= k=5 → emitted normally with pct=70 (ignored in assertion).
        insert_n(&mut conn, "safety", 3, now);
        insert_n(&mut conn, "values-forward", 7, now);

        let config = AggregatorConfig {
            k_threshold: 5,
            context_window_seconds: 86_400,
            below_threshold_mode: BelowThresholdMode::AddLaplaceNoise { scale_pct: 10 },
        };

        let candidates =
            aggregate_and_emit(&mut conn, "household", &config, now).expect("aggregate");

        // Safety candidate must be emitted (noised), ValuesForward emitted (above threshold).
        let safety_candidate = candidates
            .iter()
            .find(|c| c.classification == "safety")
            .expect("Safety candidate must be emitted with AddLaplaceNoise");

        let noised_pct = safety_candidate.participating_pct;
        assert!(
            (20u8..=40).contains(&noised_pct),
            "noised_pct must be in [20, 40] for scale_pct=10, raw_pct=30, got {noised_pct}"
        );
    }

    // -----------------------------------------------------------------------
    // T7: struct has no peer identity fields (constitutional invariant)
    // -----------------------------------------------------------------------

    #[test]
    fn candidate_struct_has_no_peer_identity() {
        // Exhaustive construction — if any peer-identity field is added to the
        // struct, this test will fail to compile (missing field).
        let candidate = CollectiveFilterPatternCandidate {
            collective: "household".to_string(),
            filter_subject_json: r#"{"contentKind":"concept"}"#.to_string(),
            classification: "safety".to_string(),
            participating_pct: 75,
            trend: "stable".to_string(),
            context_window_seconds: 86_400,
        };

        // Verify by serializing to JSON — none of these strings should appear.
        let json = serde_json::to_string(&candidate).expect("serialize");
        assert!(
            !json.contains("signerPubkey"),
            "candidate must not contain signerPubkey (peer identity leak)"
        );
        assert!(
            !json.contains("signer_pubkey"),
            "candidate must not contain signer_pubkey (peer identity leak)"
        );
        assert!(
            !json.contains("subject"),
            "candidate must not contain subject (peer identity leak): {json}"
        );
        assert!(
            !json.contains("agentKey"),
            "candidate must not contain agentKey (peer identity leak)"
        );
        assert!(
            !json.contains("pubkey"),
            "candidate must not contain pubkey (peer identity leak): {json}"
        );

        // Assert the struct has exactly the fields we expect and nothing more.
        assert_eq!(candidate.collective, "household");
        assert_eq!(
            candidate.filter_subject_json,
            r#"{"contentKind":"concept"}"#
        );
        assert_eq!(candidate.classification, "safety");
        assert_eq!(candidate.participating_pct, 75);
        assert_eq!(candidate.trend, "stable");
        assert_eq!(candidate.context_window_seconds, 86_400);
    }

    // -----------------------------------------------------------------------
    // T8: reject empty collective
    // -----------------------------------------------------------------------

    #[test]
    fn aggregate_rejects_empty_collective() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let result = aggregate_and_emit(&mut conn, "", &AggregatorConfig::default(), 0);
        assert!(
            matches!(result, Err(AggregatorError::Invalid(_))),
            "empty collective must return Err(Invalid), got: {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // T9: reject context_window_seconds < 3600
    // -----------------------------------------------------------------------

    #[test]
    fn aggregate_rejects_short_context_window() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let config = AggregatorConfig {
            k_threshold: 5,
            context_window_seconds: 3599,
            below_threshold_mode: BelowThresholdMode::Suppress,
        };

        let result = aggregate_and_emit(&mut conn, "household", &config, 0);
        assert!(
            matches!(result, Err(AggregatorError::Invalid(_))),
            "context_window_seconds=3599 must return Err(Invalid), got: {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // T10: rows outside the context window are excluded
    // -----------------------------------------------------------------------

    #[test]
    fn outside_window_rows_excluded() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let now = 1_000_000i64;
        let window_seconds = 86_400u64;

        // 3 rows within window.
        insert_n(&mut conn, "fatigue", 3, now);

        // 3 rows outside window: last_tended_at = 100_000 << 913_600.
        insert_n(&mut conn, "fatigue", 3, 100_000);

        let config = AggregatorConfig {
            k_threshold: 3,
            context_window_seconds: window_seconds,
            below_threshold_mode: BelowThresholdMode::Suppress,
        };

        let candidates =
            aggregate_and_emit(&mut conn, "household", &config, now).expect("aggregate");

        // Only the 3 recent rows are counted; 3 >= k=3, so candidate is emitted.
        assert_eq!(
            candidates.len(),
            1,
            "exactly one candidate (only recent rows)"
        );
        assert_eq!(candidates[0].classification, "fatigue");
        // 3 rows in window, all fatigue → pct = 100.
        assert_eq!(
            candidates[0].participating_pct, 100,
            "3 of 3 in-window rows are fatigue → 100%"
        );
    }
}
