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
            // Vouch is a positive signal — negative weight reduces the debit sum.
            // Bootstrap values mirror bootstrap-standing-policy.json debitWeights.
            (SignalKind::Vouch, StandingImpact::DebitSoft) => -3,
            (SignalKind::Vouch, StandingImpact::DebitFirm) => -8,
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

pub(crate) fn score_for_debit_sum(sum: i32) -> StandingScore {
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
// ManifestDebitWeightPolicy
// ============================================================================

use crate::services::manifest_registry::ManifestRegistry;
use std::collections::HashMap;

/// Manifest-driven debit-weight lookup. Falls back to [`DefaultDebitWeightPolicy`]
/// when a key is absent from the registered standing-policy manifest, or when no
/// such manifest is registered at all.
pub struct ManifestDebitWeightPolicy {
    weights: HashMap<(SignalKind, StandingImpact), i32>,
    fallback: DefaultDebitWeightPolicy,
}

impl ManifestDebitWeightPolicy {
    pub fn from_registry(registry: &ManifestRegistry) -> Self {
        let mut weights = HashMap::new();
        if let Some(map) = registry.debit_weights() {
            for ((kind_str, impact_str), w) in map {
                if let (Some(kind), Some(impact)) = (
                    parse_signal_kind(&kind_str),
                    parse_standing_impact(&impact_str),
                ) {
                    weights.insert((kind, impact), w);
                }
            }
        }
        Self {
            weights,
            fallback: DefaultDebitWeightPolicy,
        }
    }
}

fn parse_signal_kind(s: &str) -> Option<SignalKind> {
    match s {
        "squelch" => Some(SignalKind::Squelch),
        "correction" => Some(SignalKind::Correction),
        "retraction" => Some(SignalKind::Retraction),
        "quarantine" => Some(SignalKind::Quarantine),
        "vouch" => Some(SignalKind::Vouch),
        _ => None,
    }
}

fn parse_standing_impact(s: &str) -> Option<StandingImpact> {
    match s {
        "advisory" => Some(StandingImpact::Advisory),
        "debit-soft" => Some(StandingImpact::DebitSoft),
        "debit-firm" => Some(StandingImpact::DebitFirm),
        _ => None,
    }
}

impl DebitWeightPolicy for ManifestDebitWeightPolicy {
    fn debit_weight(&self, kind: SignalKind, impact: StandingImpact) -> i32 {
        self.weights
            .get(&(kind, impact))
            .copied()
            .unwrap_or_else(|| self.fallback.debit_weight(kind, impact))
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
        use crate::p2p::feedback_signal::VouchKind;
        FeedbackSignal {
            target_cid: "bafyreitarget".to_string(),
            signal_kind: kind,
            vouch_kind: if kind == SignalKind::Vouch {
                Some(VouchKind::AcceptCorrection)
            } else {
                None
            },
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

    // Test 5: pluralism isolation — two evaluators receive the IDENTICAL signal through
    // the SAME default policy and each project into their OWN row. Row identity is
    // (evaluator_pubkey, subject_pubkey), so a wrong WHERE clause that bled state from
    // eval1's row into eval2's would produce sum=16 on eval2 instead of sum=8 and fail.
    #[test]
    fn two_evaluators_independent_state() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let policy = DefaultDebitWeightPolicy;

        // SAME signal sent to both evaluators.
        let signal = make_signal(SignalKind::Correction, StandingImpact::DebitFirm);

        // Evaluator 1 projects the signal (manifest_cid distinguishes ownership row).
        project_signal(
            &mut conn,
            &policy,
            &evaluator(),
            &signal,
            "bafyreimanifest_a",
        )
        .expect("eval1 project");

        // Evaluator 2 projects the SAME signal (different manifest_cid = different steward).
        project_signal(
            &mut conn,
            &policy,
            &evaluator2(),
            &signal,
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

        // Pluralism property: identical input + identical policy → identical output
        // value, but each evaluator owns its OWN row. Both sums must be 8, not 16
        // (state bleed) and not 0 (missing projection).
        // Correction(DebitFirm) = 8 debit weight → sum=8 → Floor (threshold: 3..=7 → Low, ≥8 → Floor).
        assert_eq!(row1.debit_weight_sum, 8, "eval1 sum must be 8");
        assert_eq!(
            row2.debit_weight_sum, 8,
            "eval2 sum must be 8 (not 16 from bleed)"
        );
        assert_eq!(
            row1.score, "floor",
            "eval1 score must be floor (sum=8 crosses Low threshold)"
        );
        assert_eq!(
            row2.score, "floor",
            "eval2 score must be floor (sum=8 crosses Low threshold)"
        );
        // Each row carries its own manifest_cid — independent stewardship identity.
        assert_eq!(row1.manifest_cid, "bafyreimanifest_a");
        assert_eq!(row2.manifest_cid, "bafyreimanifest_b");
    }

    // Test 6: threshold boundary walk — calls score_for_debit_sum at every breakpoint.
    // An off-by-one in any match arm boundary would be caught here.
    #[test]
    fn score_thresholds_walk_boundaries() {
        // sum ≤ -3  → Trusted
        assert_eq!(
            score_for_debit_sum(-3),
            StandingScore::Trusted,
            "sum=-3 → Trusted"
        );
        assert_eq!(
            score_for_debit_sum(-100),
            StandingScore::Trusted,
            "sum=-100 → Trusted"
        );

        // -2 ..= -1 → High
        assert_eq!(
            score_for_debit_sum(-2),
            StandingScore::High,
            "sum=-2 → High"
        );
        assert_eq!(
            score_for_debit_sum(-1),
            StandingScore::High,
            "sum=-1 → High"
        );

        // 0 ..= 2   → Neutral
        assert_eq!(
            score_for_debit_sum(0),
            StandingScore::Neutral,
            "sum=0 → Neutral"
        );
        assert_eq!(
            score_for_debit_sum(2),
            StandingScore::Neutral,
            "sum=2 → Neutral"
        );

        // 3 ..= 7   → Low
        assert_eq!(score_for_debit_sum(3), StandingScore::Low, "sum=3 → Low");
        assert_eq!(score_for_debit_sum(7), StandingScore::Low, "sum=7 → Low");

        // 8+        → Floor
        assert_eq!(
            score_for_debit_sum(8),
            StandingScore::Floor,
            "sum=8 → Floor"
        );
        assert_eq!(
            score_for_debit_sum(100),
            StandingScore::Floor,
            "sum=100 → Floor"
        );
    }

    // -------------------------------------------------------------------------
    // T9 — ManifestDebitWeightPolicy tests
    // -------------------------------------------------------------------------

    #[test]
    fn manifest_policy_returns_manifest_weights_when_registered() {
        let json = r#"{
            "manifestKind": "standing-policy", "revision": 1,
            "floor": {"classes":[]},
            "newVoiceBaseline": {"score":"floor","vulnerableClassLift":"low"},
            "debitWeights": {
                "squelch":    {"advisory":0,"debit-soft":1,"debit-firm":3},
                "correction": {"advisory":0,"debit-soft":10,"debit-firm":20},
                "retraction": {"advisory":0,"debit-soft":-5,"debit-firm":-10},
                "quarantine": {"advisory":0,"debit-soft":12,"debit-firm":30},
                "vouch":      {"advisory":0,"debit-soft":-3,"debit-firm":-8}
            }
        }"#;
        let registry =
            crate::services::manifest_registry::ManifestRegistry::from_payload_json(json)
                .expect("parse");
        let policy = ManifestDebitWeightPolicy::from_registry(&registry);
        assert_eq!(
            policy.debit_weight(SignalKind::Vouch, StandingImpact::DebitSoft),
            -3
        );
        assert_eq!(
            policy.debit_weight(SignalKind::Correction, StandingImpact::DebitFirm),
            20
        );
    }

    #[test]
    fn manifest_policy_falls_back_to_default_when_empty() {
        let registry = crate::services::manifest_registry::ManifestRegistry::default();
        let policy = ManifestDebitWeightPolicy::from_registry(&registry);
        // DefaultDebitWeightPolicy returns 1 for squelch/debit-soft.
        assert_eq!(
            policy.debit_weight(SignalKind::Squelch, StandingImpact::DebitSoft),
            1
        );
    }
}
