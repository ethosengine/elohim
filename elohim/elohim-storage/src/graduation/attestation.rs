//! Path 1 of graduation: observations → Attestation plan.
//!
//! Gates issuance on a diversity threshold (spec §6 + §8.1). When the threshold
//! is met, returns an `AttestationPlan` that the downstream coordinator converts
//! into a Content+content_type="attestation:<subtype>" DHT entry.

use crate::db::diesel_schema::observations;
use crate::db::models::ObservationRow;
use crate::graduation::diversity::{threshold_met, DiversityThreshold};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationGraduationSpec {
    pub observation_kind: String,
    pub attestation_subtype: String,
    pub threshold: DiversityThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationPlan {
    pub attestation_content_type: String,
    pub subject_cid: String,
    pub observation_refs: Vec<String>,
    pub proof_class: String,
}

impl AttestationGraduationSpec {
    /// Evaluate whether `subject_cid` has accumulated enough observation diversity
    /// to issue an attestation of this subtype. Returns `Some(plan)` when the
    /// threshold is met; `None` otherwise.
    pub fn evaluate(
        &self,
        conn: &mut SqliteConnection,
        subject_cid: &str,
    ) -> Result<Option<AttestationPlan>, diesel::result::Error> {
        if !threshold_met(conn, subject_cid, &self.observation_kind, &self.threshold)? {
            return Ok(None);
        }
        let rows: Vec<ObservationRow> = observations::table
            .filter(observations::subject_cid.eq(subject_cid))
            .filter(observations::observation_kind.eq(&self.observation_kind))
            .order(observations::observed_at.asc())
            .load(conn)?;

        let observation_refs = rows
            .iter()
            .map(|r| format!("iroh://{}@{}#{}", r.observer_cid, r.log_cid, r.log_offset))
            .collect();

        Ok(Some(AttestationPlan {
            attestation_content_type: format!("attestation:{}", self.attestation_subtype),
            subject_cid: subject_cid.to_string(),
            observation_refs,
            proof_class: "witness".to_string(),
        }))
    }
}
