//! `ObservationManagerBackend` — owns one `ObservationLog` per local observer
//! and orchestrates writes through the `ObservationProjector` into SQL.
//!
//! Two write paths:
//! - `append_local`: self-authored observations. Keyed by the observation's
//!   `observer_cid`; stamps `log_cid` and `log_offset` from that observer's
//!   log, projects the row and persists the log head (`observation_logs`).
//! - `project_remote`: observations pulled from another observer's log.
//!   Projects to SQL only — the remote observer owns log metadata.
//!
//! A node hosts more than one observer (every person using this node through
//! the doorway or the desktop app), so logs are held per observer and each is
//! resumed lazily from its persisted head the first time it is written after a
//! start: offsets continue across restarts instead of re-minting 0. The server
//! builds one manager at boot (`HttpServer::observation_manager`).
//!
//! Read path: `query_by_subject` reads the `observations` projection by
//! `(subject_cid, observation_kind)`.
//!
//! See spec §5.1 + §4.4.

use crate::db::diesel_schema::{observation_logs, observations};
use crate::db::models::{NewObservationLogRow, ObservationLogRow, ObservationRow};
use crate::observation::log::{ObservationLog, ObservationLogError};
use crate::observation::projector::ObservationProjector;
use crate::observation::wire::Observation;
use crate::services::observation_kinds::ObservationKindRegistry;
use crate::views::ObservationView;
use diesel::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Retention class recorded for a log head when the kind is not declared (or
/// no registry is wired): the shortest-lived class, so an undeclared kind never
/// earns longer retention by default.
pub const UNDECLARED_RETENTION_CLASS: &str = "operational";

pub struct ObservationManagerBackend {
    logs: RwLock<HashMap<String, ObservationLog>>,
    projector: ObservationProjector,
    kind_registry: Option<Arc<ObservationKindRegistry>>,
}

impl Default for ObservationManagerBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservationManagerBackend {
    /// A manager with no logs loaded yet; each observer's log resumes from
    /// `observation_logs` on its first append.
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(HashMap::new()),
            projector: ObservationProjector::new(),
            kind_registry: None,
        }
    }

    /// Compatibility constructor for callers written against the single-log
    /// manager. Logs are keyed by each observation's `observer_cid`, so the
    /// argument no longer selects a log; it is equivalent to [`Self::new`].
    pub fn new_in_memory(_observer_cid: String) -> Self {
        Self::new()
    }

    /// Wire the manifest-declared observation kinds (retention class for the
    /// log head; payload validation for the write route).
    pub fn with_kind_registry(mut self, registry: Arc<ObservationKindRegistry>) -> Self {
        self.kind_registry = Some(registry);
        self
    }

    /// The manifest-declared observation kinds, when wired.
    pub fn kind_registry(&self) -> Option<&Arc<ObservationKindRegistry>> {
        self.kind_registry.as_ref()
    }

    /// Append a self-authored observation to its observer's log and return it
    /// stamped with `log_offset` and `log_cid`.
    ///
    /// Order matters:
    ///   1. Resume the observer's log from `observation_logs` if not loaded.
    ///   2. Read `latest_offset` — this observation's position in the log.
    ///   3. Append to log — advances the offset AND the BLAKE3 root.
    ///   4. Read `current_log_cid` — the new root after this observation.
    ///   5. In one transaction: project the row and upsert the log head.
    ///
    /// If step 5 fails, the in-memory log is dropped so the next append resumes
    /// from the persisted head rather than from a head SQL never recorded.
    pub async fn append_local(
        &self,
        conn: &mut SqliteConnection,
        mut obs: Observation,
    ) -> Result<Observation, ObservationLogError> {
        let observer = obs.observer_cid.clone();
        let mut logs = self.logs.write().await;

        if !logs.contains_key(&observer) {
            // The manager never reads a log's entries back (the SQL
            // projection is the read side), so its logs keep no tail: each is
            // only a hasher and an offset.
            let log = match load_head(conn, &observer)? {
                Some(head) => ObservationLog::resume(
                    observer.clone(),
                    u64::try_from(head.latest_offset).unwrap_or(0),
                    head.latest_log_cid,
                ),
                None => ObservationLog::new_in_memory(observer.clone()),
            }
            .with_tail_capacity(0);
            logs.insert(observer.clone(), log);
        }

        {
            let log = logs
                .get_mut(&observer)
                .expect("log inserted above for this observer");
            obs.log_offset = log.latest_offset();
            log.append(obs.clone()).await?;
            obs.log_cid = log.current_log_cid();
        }

        let retention_class = self
            .kind_registry
            .as_ref()
            .and_then(|r| r.get(&obs.observation_kind))
            .map(|d| d.retention_class.clone())
            .unwrap_or_else(|| UNDECLARED_RETENTION_CLASS.to_string());

        let persisted = conn.transaction::<_, diesel::result::Error, _>(|conn| {
            self.projector.project(conn, &obs)?;
            upsert_head(conn, &obs, &retention_class)
        });
        if let Err(e) = persisted {
            logs.remove(&observer);
            return Err(ObservationLogError::Persistence(format!(
                "observation projection / log head write failed: {e}"
            )));
        }
        Ok(obs)
    }

    /// Project a remote observation pulled from another observer's log.
    /// Does not touch the local logs — the remote observer owns log metadata.
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

fn load_head(
    conn: &mut SqliteConnection,
    observer: &str,
) -> Result<Option<ObservationLogRow>, ObservationLogError> {
    observation_logs::table
        .filter(observation_logs::observer_cid.eq(observer))
        .select(ObservationLogRow::as_select())
        .first(conn)
        .optional()
        .map_err(|e| ObservationLogError::Persistence(format!("log head read failed: {e}")))
}

/// Record the observer's log head after an append. `latest_offset` is the next
/// offset to be written; `retention_class` is the appended kind's declared class.
fn upsert_head(
    conn: &mut SqliteConnection,
    obs: &Observation,
    retention_class: &str,
) -> Result<(), diesel::result::Error> {
    let next_offset = obs.log_offset as i64 + 1;
    let row = NewObservationLogRow {
        observer_cid: &obs.observer_cid,
        latest_log_cid: &obs.log_cid,
        latest_offset: next_offset,
        retention_class,
    };
    diesel::insert_into(observation_logs::table)
        .values(&row)
        .on_conflict(observation_logs::observer_cid)
        .do_update()
        .set((
            observation_logs::latest_log_cid.eq(&obs.log_cid),
            observation_logs::latest_offset.eq(next_offset),
            observation_logs::retention_class.eq(retention_class),
        ))
        .execute(conn)?;
    Ok(())
}
