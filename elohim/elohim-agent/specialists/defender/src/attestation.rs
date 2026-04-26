//! Attestation stub — produces canned "no anomaly" responses.
//! Schema: elohim/sdk/schemas/v1/agent/anomaly-attestation.schema.json

use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyAttestation {
    pub observed_at: String,
    pub anomaly_kind: String,
    pub evidence: Vec<serde_json::Value>,
    pub confidence: f64,
}

pub fn build_no_anomaly_attestation() -> AnomalyAttestation {
    AnomalyAttestation {
        observed_at: Utc::now().to_rfc3339(),
        anomaly_kind: "none".into(),
        evidence: vec![],
        confidence: 0.0,
    }
}
