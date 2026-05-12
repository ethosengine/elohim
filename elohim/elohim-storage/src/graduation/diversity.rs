//! Diversity threshold check over `observation_diversity_summary`.
//! See spec §6.

use crate::db::diesel_schema::observation_diversity_summary as ods;
use crate::db::models::ObservationDiversitySummaryRow;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiversityThreshold {
    pub distinct_households: Option<i64>,
    pub distinct_collectives: Option<i64>,
    pub distinct_regions: Option<i64>,
    pub distinct_archetypes: Option<i64>,
    pub min_count: Option<i64>,
}

/// Returns true iff the (subject_cid, observation_kind) summary meets every
/// non-None component of the threshold. Missing summary row → false.
pub fn threshold_met(
    conn: &mut SqliteConnection,
    subject_cid: &str,
    observation_kind: &str,
    threshold: &DiversityThreshold,
) -> Result<bool, diesel::result::Error> {
    let row: Option<ObservationDiversitySummaryRow> = ods::table
        .filter(ods::subject_cid.eq(subject_cid))
        .filter(ods::observation_kind.eq(observation_kind))
        .first(conn)
        .optional()?;
    let Some(row) = row else {
        return Ok(false);
    };

    if let Some(t) = threshold.distinct_households {
        if row.distinct_households < t {
            return Ok(false);
        }
    }
    if let Some(t) = threshold.distinct_collectives {
        if row.distinct_collectives < t {
            return Ok(false);
        }
    }
    if let Some(t) = threshold.distinct_regions {
        if row.distinct_regions < t {
            return Ok(false);
        }
    }
    if let Some(t) = threshold.distinct_archetypes {
        if row.distinct_archetypes < t {
            return Ok(false);
        }
    }
    if let Some(t) = threshold.min_count {
        if row.total_count < t {
            return Ok(false);
        }
    }
    Ok(true)
}
