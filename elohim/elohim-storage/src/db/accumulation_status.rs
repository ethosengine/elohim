//! AccumulationStatus projection — M-POLICY-1.
//!
//! Source of truth: derived projection of FeedbackSignal × Manifest(kind:
//! standing-policy). Category C operational — reconstructable at any time.
//!
//! The projection is recomputed on two triggers:
//! 1. `Signal::FeedbackSignalCreated` — new signal for an (entityType, entityId)
//! 2. `Signal::ManifestUpdated{kind: "standing-policy"}` — policy changed;
//!    all rows are recomputed against the new thresholds.
//!
//! Protocol-default thresholds (when no standing-policy Manifest exists):
//! - readyForSensemaking: minTotalSignals=20, maxConsensusStrength=0.7
//! - controversyDetected: minTotalSignals=10, maxConsensusStrength=0.3
//! - settled: minTotalSignals=30, maxConsensusStrength(=minConsensus)=0.85

use diesel::prelude::*;
use serde_json::Value;

use crate::db::diesel_schema::accumulation_status;
use crate::db::diesel_schema::governance_signals;
use crate::db::diesel_schema::manifests;
use crate::error::StorageError;
use elohim_views::AccumulationStatusView;

// ---------------------------------------------------------------------------
// Default thresholds (mirrors the client-side constants from
// signal-accumulation.service.ts that are being retired by this ticket)
// ---------------------------------------------------------------------------

const DEFAULT_READY_MIN_SIGNALS: i64 = 20;
const DEFAULT_READY_MAX_CONSENSUS: f64 = 0.7;
const DEFAULT_CONTROVERSY_MIN_SIGNALS: i64 = 10;
const DEFAULT_CONTROVERSY_MAX_CONSENSUS: f64 = 0.3;
const DEFAULT_SETTLED_MIN_SIGNALS: i64 = 30;
const DEFAULT_SETTLED_MIN_CONSENSUS: f64 = 0.85; // called "maxConsensusStrength" in schema but read as min for settled

// ---------------------------------------------------------------------------
// Diesel row types
// ---------------------------------------------------------------------------

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = accumulation_status)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AccumulationStatusRow {
    pub entity_type: String,
    pub entity_id: String,
    pub total_signals: i32,
    pub unique_participants: i32,
    pub consensus_strength: f64,
    pub status: String,
    pub ready_for_sensemaking: i32,
    pub controversy_detected: i32,
    pub settled: i32,
    pub policy_manifest_cid: Option<String>,
    pub computed_at: String,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = accumulation_status)]
pub struct UpsertAccumulationStatus<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub total_signals: i32,
    pub unique_participants: i32,
    pub consensus_strength: f64,
    pub status: &'a str,
    pub ready_for_sensemaking: i32,
    pub controversy_detected: i32,
    pub settled: i32,
    pub policy_manifest_cid: Option<&'a str>,
    pub computed_at: &'a str,
}

// ---------------------------------------------------------------------------
// Threshold helpers
// ---------------------------------------------------------------------------

struct AccumulationThresholds {
    ready_min_signals: i64,
    ready_max_consensus: f64,
    controversy_min_signals: i64,
    controversy_max_consensus: f64,
    settled_min_signals: i64,
    settled_min_consensus: f64,
}

impl Default for AccumulationThresholds {
    fn default() -> Self {
        Self {
            ready_min_signals: DEFAULT_READY_MIN_SIGNALS,
            ready_max_consensus: DEFAULT_READY_MAX_CONSENSUS,
            controversy_min_signals: DEFAULT_CONTROVERSY_MIN_SIGNALS,
            controversy_max_consensus: DEFAULT_CONTROVERSY_MAX_CONSENSUS,
            settled_min_signals: DEFAULT_SETTLED_MIN_SIGNALS,
            settled_min_consensus: DEFAULT_SETTLED_MIN_CONSENSUS,
        }
    }
}

/// Parse thresholds from a standing-policy Manifest payload_json.
/// Returns protocol defaults when the payload is missing the
/// `accumulation_thresholds` key, or when individual threshold fields are
/// absent.
fn parse_thresholds(payload_json: &str) -> AccumulationThresholds {
    let defaults = AccumulationThresholds::default();
    let Ok(payload) = serde_json::from_str::<Value>(payload_json) else {
        return defaults;
    };
    let Some(thresholds) = payload.get("accumulation_thresholds") else {
        return defaults;
    };

    let ready_min = thresholds
        .get("readyForSensemaking")
        .and_then(|t| t.get("minTotalSignals"))
        .and_then(|v| v.as_i64())
        .unwrap_or(defaults.ready_min_signals);
    let ready_max = thresholds
        .get("readyForSensemaking")
        .and_then(|t| t.get("maxConsensusStrength"))
        .and_then(|v| v.as_f64())
        .unwrap_or(defaults.ready_max_consensus);
    let controversy_min = thresholds
        .get("controversyDetected")
        .and_then(|t| t.get("minTotalSignals"))
        .and_then(|v| v.as_i64())
        .unwrap_or(defaults.controversy_min_signals);
    let controversy_max = thresholds
        .get("controversyDetected")
        .and_then(|t| t.get("maxConsensusStrength"))
        .and_then(|v| v.as_f64())
        .unwrap_or(defaults.controversy_max_consensus);
    let settled_min = thresholds
        .get("settled")
        .and_then(|t| t.get("minTotalSignals"))
        .and_then(|v| v.as_i64())
        .unwrap_or(defaults.settled_min_signals);
    let settled_min_consensus = thresholds
        .get("settled")
        .and_then(|t| t.get("maxConsensusStrength"))
        .and_then(|v| v.as_f64())
        .unwrap_or(defaults.settled_min_consensus);

    AccumulationThresholds {
        ready_min_signals: ready_min,
        ready_max_consensus: ready_max,
        controversy_min_signals: controversy_min,
        controversy_max_consensus: controversy_max,
        settled_min_signals: settled_min,
        settled_min_consensus: settled_min_consensus,
    }
}

// ---------------------------------------------------------------------------
// Core projection logic
// ---------------------------------------------------------------------------

/// Compute consensus_strength from governance_signals for (entity_type, entity_id).
///
/// Uses normalized Shannon entropy over signal_value distribution (mirrors the
/// existing aggregate_signals function in governance.rs).
fn compute_aggregate(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<(i64, i64, f64), StorageError> {
    // Load signals for the entity using Diesel query builder
    let signals: Vec<(String, String)> = governance_signals::table
        .filter(governance_signals::entity_type.eq(entity_type))
        .filter(governance_signals::entity_id.eq(entity_id))
        .select((governance_signals::human_id, governance_signals::signal_value))
        .load::<(String, String)>(conn)
        .map_err(|e| StorageError::Database(format!("accumulation_status aggregate: {e}")))?;

    let total_signals = signals.len() as i64;
    if total_signals == 0 {
        return Ok((0, 0, 0.0));
    }

    let mut by_value: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut participants: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (human_id, signal_value) in &signals {
        *by_value.entry(signal_value.clone()).or_insert(0) += 1;
        participants.insert(human_id.clone());
    }
    let unique_participants = participants.len() as i64;

    let consensus_strength = {
        let num_distinct = by_value.len();
        if num_distinct <= 1 {
            1.0
        } else {
            let total = total_signals as f64;
            let entropy: f64 = by_value
                .values()
                .map(|&count| {
                    let p = count as f64 / total;
                    -p * p.ln()
                })
                .sum();
            let max_entropy = (num_distinct as f64).ln();
            1.0 - (entropy / max_entropy)
        }
    };

    Ok((total_signals, unique_participants, consensus_strength))
}

/// Compute status string from flags.
fn compute_status(ready: bool, controversy: bool, settled: bool) -> &'static str {
    if settled {
        "settled"
    } else if controversy {
        "controversy_detected"
    } else if ready {
        "ready_for_sensemaking"
    } else {
        "pending"
    }
}

/// Upsert a single (entity_type, entity_id) accumulation status row.
///
/// Reads the governance_signals aggregate, reads the most recent
/// standing-policy Manifest (if any), computes status flags, and upserts.
pub fn project_accumulation_status(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<AccumulationStatusView, StorageError> {
    let now = chrono::Utc::now().to_rfc3339();

    // 1. Compute signal aggregate.
    let (total_signals, unique_participants, consensus_strength) =
        compute_aggregate(conn, entity_type, entity_id)?;

    // 2. Load the most recent standing-policy Manifest (highest revision).
    let policy_row = manifests::table
        .filter(manifests::manifest_kind.eq("standing-policy"))
        .order(manifests::revision.desc())
        .first::<crate::db::manifests::ManifestRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("accumulation_status manifest load: {e}")))?;

    let (thresholds, policy_cid) = match &policy_row {
        Some(row) => (parse_thresholds(&row.payload_json), Some(row.cid.as_str())),
        None => (AccumulationThresholds::default(), None),
    };

    // 3. Apply thresholds.
    let ready = total_signals >= thresholds.ready_min_signals
        && consensus_strength < thresholds.ready_max_consensus;
    let controversy = total_signals >= thresholds.controversy_min_signals
        && consensus_strength < thresholds.controversy_max_consensus;
    let settled = total_signals >= thresholds.settled_min_signals
        && consensus_strength >= thresholds.settled_min_consensus;
    let status = compute_status(ready, controversy, settled);

    // 4. Upsert the projection row.
    let row = UpsertAccumulationStatus {
        entity_type,
        entity_id,
        total_signals: total_signals as i32,
        unique_participants: unique_participants as i32,
        consensus_strength,
        status,
        ready_for_sensemaking: ready as i32,
        controversy_detected: controversy as i32,
        settled: settled as i32,
        policy_manifest_cid: policy_cid,
        computed_at: &now,
    };

    diesel::insert_into(accumulation_status::table)
        .values(&row)
        .on_conflict((accumulation_status::entity_type, accumulation_status::entity_id))
        .do_update()
        .set(&row)
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("accumulation_status upsert: {e}")))?;

    Ok(AccumulationStatusView {
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        total_signals,
        unique_participants,
        consensus_strength,
        status: status.to_string(),
        ready_for_sensemaking: ready,
        controversy_detected: controversy,
        settled,
        policy_manifest_cid: policy_cid.map(String::from),
        computed_at: now,
    })
}

/// Fetch an already-projected accumulation status row by (entity_type, entity_id).
///
/// Returns None if the entity has no projection yet (no signals ever recorded).
pub fn fetch_accumulation_status(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Option<AccumulationStatusView>, StorageError> {
    let row = accumulation_status::table
        .filter(accumulation_status::entity_type.eq(entity_type))
        .filter(accumulation_status::entity_id.eq(entity_id))
        .first::<AccumulationStatusRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("accumulation_status fetch: {e}")))?;

    Ok(row.map(|r| AccumulationStatusView {
        entity_type: r.entity_type,
        entity_id: r.entity_id,
        total_signals: r.total_signals as i64,
        unique_participants: r.unique_participants as i64,
        consensus_strength: r.consensus_strength,
        status: r.status,
        ready_for_sensemaking: r.ready_for_sensemaking != 0,
        controversy_detected: r.controversy_detected != 0,
        settled: r.settled != 0,
        policy_manifest_cid: r.policy_manifest_cid,
        computed_at: r.computed_at,
    }))
}

/// Recompute accumulation status for ALL known (entity_type, entity_id) pairs
/// that currently have governance signals.
///
/// Called when a standing-policy Manifest is created or updated — the new
/// thresholds apply to all existing entities.
pub fn reproject_all_accumulation_statuses(
    conn: &mut SqliteConnection,
) -> Result<usize, StorageError> {
    // Collect distinct (entity_type, entity_id) pairs from governance_signals.
    let pairs: Vec<(String, String)> = governance_signals::table
        .select((governance_signals::entity_type, governance_signals::entity_id))
        .distinct()
        .load::<(String, String)>(conn)
        .map_err(|e| StorageError::Database(format!("accumulation_status pairs load: {e}")))?;

    let count = pairs.len();
    for (et, ei) in pairs {
        project_accumulation_status(conn, &et, &ei)?;
    }
    Ok(count)
}

// ---------------------------------------------------------------------------
// From impl (DB row → View) for callers that already have a row
// ---------------------------------------------------------------------------

impl From<AccumulationStatusRow> for AccumulationStatusView {
    fn from(r: AccumulationStatusRow) -> Self {
        AccumulationStatusView {
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            total_signals: r.total_signals as i64,
            unique_participants: r.unique_participants as i64,
            consensus_strength: r.consensus_strength,
            status: r.status,
            ready_for_sensemaking: r.ready_for_sensemaking != 0,
            controversy_detected: r.controversy_detected != 0,
            settled: r.settled != 0,
            policy_manifest_cid: r.policy_manifest_cid,
            computed_at: r.computed_at,
        }
    }
}
