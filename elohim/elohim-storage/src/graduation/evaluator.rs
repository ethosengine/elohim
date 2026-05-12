//! `GraduationEvaluator` — wraps both graduation paths (attestation + summary
//! EconomicEvent) in a single tickable entrypoint. Subsequent runtime work
//! will wire this into a tokio interval task that polls on a cadence.
//!
//! See spec §8.

use crate::db::diesel_schema::observations;
use crate::graduation::attestation::{AttestationGraduationSpec, AttestationPlan};
use crate::graduation::diversity::DiversityThreshold;
use crate::graduation::summary_event::{GraduatedSummary, SummaryEventSpec};
use diesel::prelude::*;

/// Discriminated config for one graduation rule — either an attestation
/// threshold gate or a time-windowed summary EconomicEvent.
#[derive(Debug, Clone)]
pub enum GraduationConfig {
    Attestation(AttestationGraduationSpec),
    Summary(SummaryEventSpec),
}

impl GraduationConfig {
    /// Build an attestation config — one plan per qualifying subject_cid.
    pub fn attestation(
        observation_kind: &str,
        subtype: &str,
        threshold: DiversityThreshold,
    ) -> Self {
        Self::Attestation(AttestationGraduationSpec {
            observation_kind: observation_kind.into(),
            attestation_subtype: subtype.into(),
            threshold,
        })
    }

    /// Build a summary EconomicEvent config — one summary per non-empty window.
    pub fn summary(
        observation_kind: &str,
        action_verb: &str,
        resource: &str,
        window_seconds: i64,
    ) -> Self {
        Self::Summary(SummaryEventSpec {
            observation_kind: observation_kind.into(),
            action_verb: action_verb.into(),
            resource: resource.into(),
            window_seconds,
        })
    }
}

/// Aggregated output of a single `tick`. A future runtime layer converts
/// these into DHT writes (attestations) and EconomicEvent submissions
/// (summaries).
#[derive(Debug, Default)]
pub struct GraduationPlans {
    pub attestations: Vec<AttestationPlan>,
    pub summaries: Vec<GraduatedSummary>,
}

/// Evaluates a list of `GraduationConfig`s against the current observation
/// table in a single pass. Designed to be called on a periodic cadence by a
/// tokio interval task (runtime wiring is downstream of this crate).
pub struct GraduationEvaluator {
    configs: Vec<GraduationConfig>,
}

impl GraduationEvaluator {
    pub fn new(configs: Vec<GraduationConfig>) -> Self {
        Self { configs }
    }

    /// Evaluate every config against the current observation table.
    ///
    /// - **Attestation configs**: walk every distinct `subject_cid` for the
    ///   observation kind and emit one `AttestationPlan` per subject whose
    ///   diversity threshold is met.
    /// - **Summary configs**: use `now` as the window end; emit one
    ///   `GraduatedSummary` when at least one observation falls in the window.
    pub fn tick(
        &mut self,
        conn: &mut SqliteConnection,
        now: i64,
    ) -> Result<GraduationPlans, diesel::result::Error> {
        let mut plans = GraduationPlans::default();

        for cfg in &self.configs {
            match cfg {
                GraduationConfig::Attestation(spec) => {
                    // Collect every distinct non-null subject_cid for this kind.
                    let subjects: Vec<Option<String>> = observations::table
                        .filter(observations::observation_kind.eq(&spec.observation_kind))
                        .filter(observations::subject_cid.is_not_null())
                        .select(observations::subject_cid)
                        .distinct()
                        .load(conn)?;

                    for subject in subjects.into_iter().flatten() {
                        if let Some(plan) = spec.evaluate(conn, &subject)? {
                            plans.attestations.push(plan);
                        }
                    }
                }
                GraduationConfig::Summary(spec) => {
                    if let Some(summary) = spec.evaluate(conn, now)? {
                        plans.summaries.push(summary);
                    }
                }
            }
        }

        Ok(plans)
    }
}
