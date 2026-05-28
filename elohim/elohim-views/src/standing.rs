//! standing view types — StandingScoreView for GET /api/v1/standing/{agent_cid}.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StandingScoreView {
    pub evaluator_cid: String,
    pub subject_cid: String,
    pub score: StandingScoreTier,
    pub debit_weight_sum: i32,
    pub recent_breaches: Vec<FeedbackSignalSummary>,
    pub computed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum StandingScoreTier {
    Unknown,
    Floor,
    Low,
    Neutral,
    High,
    Trusted,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FeedbackSignalSummary {
    pub signal_kind: String,
    pub emitted_at: String,
    pub weight: i32,
    pub evidence_summary: Option<String>,
}
