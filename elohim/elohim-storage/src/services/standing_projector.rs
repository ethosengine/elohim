//! Standing projector — recomputes per-evaluator StandingScore on FeedbackSignal arrival.
//!
//! Category C (operational). Per brainstorm §4.2: standing is a derived view,
//! never a stored authoritative score. Different evaluators see different
//! views (pluralism property) because each peer projects through THEIR
//! manifest subscriptions.
//!
//! Wiring deferred to T19:
//!   TODO(T19): in src/api/epr.rs put_epr handler, when an arriving EPR has
//!   kind == FeedbackSignal, call project_signal(conn, &policy, &local_evaluator,
//!   &signal, manifest_cid). The local_evaluator is the local agent's pubkey;
//!   manifest_cid is the active standing-policy manifest CID from ManifestRegistry.
//!
//! See: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md §4

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use diesel::sqlite::SqliteConnection;

use crate::db::standing_view::{fetch, upsert, StandingViewRow};
use crate::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};
use crate::services::standing::StandingScore;

// ============================================================================
// Policy trait
// ============================================================================

pub trait DebitWeightPolicy {
    /// Debit weight for a (signal_kind, standing_impact) pair. Positive values
    /// reduce the subject's StandingScore. Zero means advisory-only (no impact).
    fn debit_weight(&self, kind: SignalKind, impact: StandingImpact) -> i32;
}

// ============================================================================
// Default bootstrap policy
// ============================================================================

pub struct DefaultDebitWeightPolicy;

impl DebitWeightPolicy for DefaultDebitWeightPolicy {
    fn debit_weight(&self, kind: SignalKind, impact: StandingImpact) -> i32 {
        // Bootstrap weights — T17 replaces this with manifest-driven values.
        // Advisory has zero weight: it is informational only. Soft and firm
        // graduate the impact. Squelch is the lightest signal kind even at
        // debit-firm impact (it is local-effect-first); Correction and
        // Quarantine carry full weight at debit-firm.
        match (kind, impact) {
            (_, StandingImpact::Advisory) => 0,
            (SignalKind::Squelch, StandingImpact::DebitSoft) => 1,
            (SignalKind::Squelch, StandingImpact::DebitFirm) => 3,
            (SignalKind::Correction, StandingImpact::DebitSoft) => 2,
            (SignalKind::Correction, StandingImpact::DebitFirm) => 8,
            (SignalKind::Retraction, StandingImpact::DebitSoft) => -1, // restitution
            (SignalKind::Retraction, StandingImpact::DebitFirm) => -3, // restitution
            (SignalKind::Quarantine, StandingImpact::DebitSoft) => 4,
            (SignalKind::Quarantine, StandingImpact::DebitFirm) => 12,
        }
    }
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ProjectorError {
    #[error("database error: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("invalid signed_by base64: {0}")]
    InvalidSignedBy(String),
}

// ============================================================================
// Core projection function
// ============================================================================

/// Project a FeedbackSignal: update the standing_view row for the (evaluator, subject)
/// pair, applying the debit weight from the policy. Subject is decoded from
/// `signal.signed_by` (base64 ed25519 public key).
pub fn project_signal(
    conn: &mut SqliteConnection,
    policy: &dyn DebitWeightPolicy,
    evaluator: &[u8],
    signal: &FeedbackSignal,
    manifest_cid: &str,
) -> Result<StandingScore, ProjectorError> {
    let subject = BASE64
        .decode(&signal.signed_by)
        .map_err(|e| ProjectorError::InvalidSignedBy(e.to_string()))?;

    let weight = policy.debit_weight(signal.signal_kind, signal.standing_impact);
    let now = Utc::now().to_rfc3339();

    let existing = fetch(conn, evaluator, &subject)?;
    let new_sum = existing.as_ref().map(|r| r.debit_weight_sum).unwrap_or(0) + weight;
    let new_score = score_for_debit_sum(new_sum);

    upsert(
        conn,
        &StandingViewRow {
            evaluator_pubkey: evaluator.to_vec(),
            subject_pubkey: subject,
            score: serialize_score(new_score),
            debit_weight_sum: new_sum,
            last_signal_at: now,
            manifest_cid: manifest_cid.to_string(),
        },
    )?;

    Ok(new_score)
}

// ============================================================================
// Score/sum helpers (public so standing.rs can use deserialize_score)
// ============================================================================

fn score_for_debit_sum(sum: i32) -> StandingScore {
    // Cumulative debit thresholds. Negative sum = restitution beyond baseline.
    // Bootstrap thresholds; T17 replaces with manifest-driven values.
    match sum {
        i32::MIN..=-3 => StandingScore::Trusted,
        -2..=-1 => StandingScore::High,
        0..=2 => StandingScore::Neutral,
        3..=7 => StandingScore::Low,
        _ => StandingScore::Floor,
    }
}

pub fn serialize_score(score: StandingScore) -> String {
    match score {
        StandingScore::Floor => "floor",
        StandingScore::Low => "low",
        StandingScore::Neutral => "neutral",
        StandingScore::High => "high",
        StandingScore::Trusted => "trusted",
    }
    .to_string()
}

pub fn deserialize_score(s: &str) -> Option<StandingScore> {
    match s {
        "floor" => Some(StandingScore::Floor),
        "low" => Some(StandingScore::Low),
        "neutral" => Some(StandingScore::Neutral),
        "high" => Some(StandingScore::High),
        "trusted" => Some(StandingScore::Trusted),
        _ => None,
    }
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
            "file:projector_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn evaluator() -> Vec<u8> {
        vec![0xE0u8; 32]
    }

    fn evaluator2() -> Vec<u8> {
        vec![0xE1u8; 32]
    }

    /// Base64-encoded 32-byte zero pubkey (valid base64, decodes to 32 bytes).
    fn subject_b64() -> String {
        BASE64.encode(vec![0xABu8; 32])
    }

    fn make_signal(kind: SignalKind, impact: StandingImpact) -> FeedbackSignal {
        FeedbackSignal {
            target_cid: "bafyreitarget".to_string(),
            signal_kind: kind,
            evidence_cid: if kind == SignalKind::Correction {
                Some("bafyreievi".to_string())
            } else {
                None
            },
            standing_impact: impact,
            signed_by: subject_b64(),
            signature: BASE64.encode(vec![0xFFu8; 64]),
        }
    }

    // Test 1: first squelch (debit-firm = 3) lands at Low (sum 3).
    #[test]
    fn empty_view_then_squelch_writes_low() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let policy = DefaultDebitWeightPolicy;

        let signal = make_signal(SignalKind::Squelch, StandingImpact::DebitFirm);
        let score = project_signal(&mut conn, &policy, &evaluator(), &signal, "bafyreimanifest")
            .expect("project");

        assert_eq!(score, StandingScore::Low, "sum=3 should be Low");

        // Verify row was written.
        let subject = BASE64.decode(subject_b64()).unwrap();
        let row = crate::db::standing_view::fetch(&mut conn, &evaluator(), &subject)
            .expect("fetch")
            .expect("row should exist");
        assert_eq!(row.debit_weight_sum, 3);
        assert_eq!(row.score, "low");
    }

    // Test 2: advisory impact yields no debit; sum stays 0; score Neutral.
    #[test]
    fn advisory_signal_writes_zero_weight() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let policy = DefaultDebitWeightPolicy;

        let signal = make_signal(SignalKind::Squelch, StandingImpact::Advisory);
        let score = project_signal(&mut conn, &policy, &evaluator(), &signal, "bafyreimanifest")
            .expect("project");

        assert_eq!(score, StandingScore::Neutral, "advisory should be Neutral");

        let subject = BASE64.decode(subject_b64()).unwrap();
        let row = crate::db::standing_view::fetch(&mut conn, &evaluator(), &subject)
            .expect("fetch")
            .expect("row should exist");
        assert_eq!(row.debit_weight_sum, 0);
        assert_eq!(row.score, "neutral");
    }

    // Test 3: Correction(DebitFirm)=+8 then Retraction(DebitFirm)=-3; final sum=5 (Low).
    #[test]
    fn retraction_after_correction_raises_score() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let policy = DefaultDebitWeightPolicy;

        let correction = make_signal(SignalKind::Correction, StandingImpact::DebitFirm);
        project_signal(
            &mut conn,
            &policy,
            &evaluator(),
            &correction,
            "bafyreimanifest",
        )
        .expect("apply correction");

        let retraction = make_signal(SignalKind::Retraction, StandingImpact::DebitFirm);
        let score = project_signal(
            &mut conn,
            &policy,
            &evaluator(),
            &retraction,
            "bafyreimanifest",
        )
        .expect("apply retraction");

        // sum = 8 + (-3) = 5 → Low
        assert_eq!(score, StandingScore::Low, "sum=5 should still be Low");

        let subject = BASE64.decode(subject_b64()).unwrap();
        let row = crate::db::standing_view::fetch(&mut conn, &evaluator(), &subject)
            .expect("fetch")
            .expect("row exists");
        assert_eq!(row.debit_weight_sum, 5);
    }

    // Test 4: 3 × Quarantine(DebitFirm) = 3 × 12 = 36 → Floor.
    #[test]
    fn cumulative_signals_drive_to_floor() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let policy = DefaultDebitWeightPolicy;

        for _ in 0..3 {
            let signal = make_signal(SignalKind::Quarantine, StandingImpact::DebitFirm);
            project_signal(&mut conn, &policy, &evaluator(), &signal, "bafyreimanifest")
                .expect("project quarantine");
        }

        let subject = BASE64.decode(subject_b64()).unwrap();
        let row = crate::db::standing_view::fetch(&mut conn, &evaluator(), &subject)
            .expect("fetch")
            .expect("row exists");
        assert_eq!(row.debit_weight_sum, 36);
        assert_eq!(
            deserialize_score(&row.score),
            Some(StandingScore::Floor),
            "sum=36 should be Floor"
        );
    }

    // Test 5: two evaluators project independently for same subject+signal.
    #[test]
    fn two_evaluators_independent_state() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let policy = DefaultDebitWeightPolicy;

        let signal = make_signal(SignalKind::Correction, StandingImpact::DebitFirm);

        // Evaluator 1 applies the signal.
        project_signal(
            &mut conn,
            &policy,
            &evaluator(),
            &signal,
            "bafyreimanifest_a",
        )
        .expect("eval1 project");

        // Evaluator 2 applies a different (advisory) signal for the same subject.
        let advisory = make_signal(SignalKind::Squelch, StandingImpact::Advisory);
        project_signal(
            &mut conn,
            &policy,
            &evaluator2(),
            &advisory,
            "bafyreimanifest_b",
        )
        .expect("eval2 project");

        let subject = BASE64.decode(subject_b64()).unwrap();

        let row1 = crate::db::standing_view::fetch(&mut conn, &evaluator(), &subject)
            .expect("fetch eval1")
            .expect("eval1 row exists");

        let row2 = crate::db::standing_view::fetch(&mut conn, &evaluator2(), &subject)
            .expect("fetch eval2")
            .expect("eval2 row exists");

        // Evaluator 1 sees sum=8 (Correction DebitFirm), evaluator 2 sees sum=0 (Advisory).
        assert_eq!(row1.debit_weight_sum, 8);
        assert_eq!(row2.debit_weight_sum, 0);
        assert_ne!(
            row1.score, row2.score,
            "evaluators must have independent standing views"
        );
        assert_eq!(row1.manifest_cid, "bafyreimanifest_a");
        assert_eq!(row2.manifest_cid, "bafyreimanifest_b");
    }
}
