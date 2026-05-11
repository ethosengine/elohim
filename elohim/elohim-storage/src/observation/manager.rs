//! `ObservationManagerBackend` — owns the local observer's `ObservationLog`
//! and orchestrates writes through the `ObservationProjector` into SQL.
//!
//! Two write paths:
//! - `append_local`: self-authored observations. Stamps `log_cid` and
//!   `log_offset` from the log's current state before projecting.
//! - `project_remote`: observations pulled from another observer's log.
//!   Projects to SQL only — the remote observer owns log metadata.
//!
//! Read path: `query_by_subject` reads the `observations` projection by
//! `(subject_cid, observation_kind)`.
//!
//! See spec §5.1 + §4.4.

use crate::db::diesel_schema::observations;
use crate::db::models::ObservationRow;
use crate::observation::log::{ObservationLog, ObservationLogError};
use crate::observation::projector::ObservationProjector;
use crate::observation::wire::Observation;
use crate::views::ObservationView;
use diesel::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ObservationManagerBackend {
    local_log: Arc<RwLock<ObservationLog>>,
    projector: ObservationProjector,
}

impl ObservationManagerBackend {
    pub fn new_in_memory(observer_cid: String) -> Self {
        Self {
            local_log: Arc::new(RwLock::new(ObservationLog::new_in_memory(observer_cid))),
            projector: ObservationProjector::new(),
        }
    }

    /// Append a self-authored observation. Stamps `log_offset` from the log's
    /// current length (before append), appends to the log (advancing the
    /// BLAKE3 root), then stamps `log_cid` from the new root and projects to
    /// SQL.
    ///
    /// Order matters:
    ///   1. Read `latest_offset` — this observation's position in the log.
    ///   2. Append to log — advances internal counter AND BLAKE3 root.
    ///   3. Read `current_log_cid` — the new root after this observation.
    ///   4. Set both fields on `obs`.
    ///   5. Project to SQL.
    pub async fn append_local(
        &self,
        conn: &mut SqliteConnection,
        mut obs: Observation,
    ) -> Result<(), ObservationLogError> {
        let mut log = self.local_log.write().await;
        obs.log_offset = log.latest_offset();
        log.append(obs.clone()).await?;
        obs.log_cid = log.current_log_cid();
        self.projector
            .project(conn, &obs)
            .map_err(|e| ObservationLogError::Encoding(format!("SQL projection failed: {e}")))?;
        Ok(())
    }

    /// Project a remote observation pulled from another observer's log.
    /// Does not touch the local log — the remote observer owns log metadata.
    pub async fn project_remote(
        &self,
        conn: &mut SqliteConnection,
        obs: &Observation,
    ) -> Result<(), ObservationLogError> {
        self.projector
            .project(conn, obs)
            .map_err(|e| ObservationLogError::Encoding(format!("SQL projection failed: {e}")))?;
        Ok(())
    }

    /// Query observations by `(subject_cid, observation_kind)`, newest first.
    /// Queries the `ObservationRow` projection and converts to `ObservationView`.
    pub fn query_by_subject(
        &self,
        conn: &mut SqliteConnection,
        subject: &str,
        kind: &str,
    ) -> Result<Vec<ObservationView>, diesel::result::Error> {
        let rows: Vec<ObservationRow> = observations::table
            .filter(observations::subject_cid.eq(subject))
            .filter(observations::observation_kind.eq(kind))
            .order(observations::observed_at.desc())
            .load(conn)?;
        Ok(rows.into_iter().map(ObservationView::from).collect())
    }
}
