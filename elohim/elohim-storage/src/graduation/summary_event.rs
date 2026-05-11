//! Path 2 of graduation: observations → summary EconomicEvent.
//!
//! For high-frequency operational verbs (blob-served, consumed-compute, etc.),
//! the per-occurrence event would crush DHT write pressure. Instead, observations
//! accumulate on the substrate; periodic summaries graduate to DHT-notarised
//! EconomicEvents that reference the contributing observations via `observation_refs`.
//!
//! See spec §8.2.

use crate::db::diesel_schema::observations;
use crate::db::models::ObservationRow;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryEventSpec {
    pub observation_kind: String,
    pub action_verb: String,
    pub resource: String,
    pub window_seconds: i64,
}

/// Output of a graduation evaluation. Consumed by the coordinator that creates
/// the EconomicEvent on the DHT (downstream of this crate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraduatedSummary {
    pub action: String,
    pub provider_cid: String,
    pub resource: String,
    pub total_quantity: f64,
    pub period_start: i64,
    pub period_end: i64,
    pub observation_refs: Vec<String>,
}

impl SummaryEventSpec {
    /// Evaluate the window ending at `window_end`. Returns `Some(summary)` if
    /// at least one observation of this kind exists in the window; `None` otherwise.
    pub fn evaluate(
        &self,
        conn: &mut SqliteConnection,
        window_end: i64,
    ) -> Result<Option<GraduatedSummary>, diesel::result::Error> {
        let window_start = window_end - self.window_seconds;
        let rows: Vec<ObservationRow> = observations::table
            .filter(observations::observation_kind.eq(&self.observation_kind))
            .filter(observations::observed_at.ge(window_start))
            .filter(observations::observed_at.lt(window_end))
            .load(conn)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let provider_cid = rows[0].observer_cid.clone();
        let total_quantity: f64 = rows
            .iter()
            .filter_map(|r| {
                serde_json::from_str::<serde_json::Value>(&r.payload_json)
                    .ok()
                    .and_then(|v| v.get("bytes").and_then(|n| n.as_f64()))
            })
            .sum();
        let observation_refs = rows
            .iter()
            .map(|r| format!("iroh://{}@{}#{}", r.observer_cid, r.log_cid, r.log_offset))
            .collect();

        Ok(Some(GraduatedSummary {
            action: self.action_verb.clone(),
            provider_cid,
            resource: self.resource.clone(),
            total_quantity,
            period_start: window_start,
            period_end: window_end,
            observation_refs,
        }))
    }
}
