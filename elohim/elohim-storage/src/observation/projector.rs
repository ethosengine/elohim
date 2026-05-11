//! ObservationProjector — writes Observation wire records into the SQL
//! projection table. Idempotent on primary key
//! `(observer_cid, log_cid, log_offset)` so log replay is safe.
//!
//! Source of truth for an observation is the per-observer iroh-blob log;
//! this projector is the read-optimised SQL projection.
//!
//! See spec §4.4.

use crate::db::diesel_schema::observations;
use crate::observation::wire::Observation;
use base64::Engine;
use diesel::prelude::*;

pub struct ObservationProjector;

impl ObservationProjector {
    pub fn new() -> Self {
        Self
    }

    /// Insert this observation into SQL. Idempotent: a row with the same
    /// `(observer_cid, log_cid, log_offset)` is silently ignored.
    pub fn project(
        &self,
        conn: &mut SqliteConnection,
        obs: &Observation,
    ) -> Result<(), diesel::result::Error> {
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(&obs.signature);

        diesel::insert_or_ignore_into(observations::table)
            .values((
                observations::observer_cid.eq(obs.observer_cid.as_str()),
                observations::log_cid.eq(obs.log_cid.as_str()),
                observations::log_offset.eq(obs.log_offset as i64),
                observations::observed_at.eq(obs.observed_at),
                observations::seq.eq(obs.seq as i64),
                observations::observation_kind.eq(obs.observation_kind.as_str()),
                observations::subject_cid.eq(obs.subject_cid.as_deref()),
                observations::subject_kind.eq(obs.subject_kind.as_deref()),
                observations::payload_json.eq(obs.payload_json.as_str()),
                observations::observer_household_cid.eq(obs.observer_household_cid.as_deref()),
                observations::observer_collective_cid.eq(obs.observer_collective_cid.as_deref()),
                observations::observer_region.eq(obs.observer_region.as_deref()),
                observations::observer_archetype.eq(obs.observer_archetype.as_deref()),
                observations::observer_compute_class.eq(obs.observer_compute_class.as_deref()),
                observations::signature_b64.eq(sig_b64.as_str()),
            ))
            .execute(conn)?;
        Ok(())
    }
}

impl Default for ObservationProjector {
    fn default() -> Self {
        Self::new()
    }
}
