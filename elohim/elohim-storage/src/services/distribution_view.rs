//! Distribution view service — compose `DistributionSummary` from substrate state.
//!
//! ## Source of Truth
//!
//! Operational (Category C) per the p2p-design-gate output in
//! `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! Composes a read-projection from notarized DHT state. The DHT remains canonical.
//! No SQLite table here is authoritative.

use std::collections::HashSet;

use chrono::DateTime;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{Float, Nullable};
use thiserror::Error;

use crate::db::models::PeerIdentityBindingRow;
use crate::db::DbPool;
use crate::views::{
    DistributionSummary, DiversityHint, FetchSource, MyRole, ReachClass, ReplicaHealth,
};

// ============================================================================
// Public types
// ============================================================================

/// Distribution-view authentication context. Visitor sees the public surface;
/// Steward sees their own role + reciprocity hint.
pub enum DistributionContext<'a> {
    Visitor,
    Steward {
        agent_cid: &'a str,
        bindings: &'a [PeerIdentityBindingRow],
    },
}

#[derive(Debug, Error)]
pub enum DistributionViewError {
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("pool error: {0}")]
    Pool(String),
}

// ============================================================================
// Public helpers
// ============================================================================

/// Map a reach class to a sensible target replica count.
///
/// Defaults are operator-tunable in a later sprint; T23 ships these as
/// hard-coded structural floors. The relative ordering (Public > District >
/// ... > Private) reflects "broader reach => more replicas needed for
/// resilience under wider distribution."
pub fn replica_target_for(reach: &ReachClass) -> u32 {
    match reach {
        ReachClass::Private => 2,
        ReachClass::Intimate => 4,
        ReachClass::Household => 6,
        ReachClass::Neighborhood => 8,
        ReachClass::Collective => 10,
        ReachClass::Community => 12,
        ReachClass::District => 14,
        ReachClass::Public => 16,
    }
}

/// Replica-health classification: count vs target.
///   ratio >= 0.85 -> Healthy
///   ratio >= 0.50 -> AtRisk
///   ratio  < 0.50 -> Critical
///   target == 0   -> Healthy (vacuous)
pub fn replica_health_for(count: u32, target: u32) -> ReplicaHealth {
    if target == 0 {
        return ReplicaHealth::Healthy;
    }
    let ratio = count as f64 / target as f64;
    if ratio >= 0.85 {
        ReplicaHealth::Healthy
    } else if ratio >= 0.50 {
        ReplicaHealth::AtRisk
    } else {
        ReplicaHealth::Critical
    }
}

// ============================================================================
// Main compose function
// ============================================================================

/// Compose a `DistributionSummary` from substrate state for a given blob hash.
///
/// - `replica_count`: from `peer_blob_inventory` (observed reality, not commitment)
/// - `reach_class`: from `content.reach` (default `Private` on miss)
/// - `reciprocity_hint`: REA outflow-minus-inflow for steward context (None for Visitor)
/// - `projector_count`: stubbed 0 (TODO Phase 4 follow-up — no projector table yet)
/// - `diversity_hint`: stubbed None (TODO Phase 4 follow-up — no geo/archetype index yet)
pub async fn compose_distribution_summary(
    pool: &DbPool,
    blob_hash: &str,
    ctx: DistributionContext<'_>,
) -> Result<DistributionSummary, DistributionViewError> {
    let mut conn = pool
        .get()
        .map_err(|e| DistributionViewError::Pool(e.to_string()))?;

    // --- replica_count and last_verified_seconds from peer_blob_inventory ---
    let inventory_rows: Vec<(String, String)> = {
        use crate::db::diesel_schema::peer_blob_inventory::dsl as inv;
        inv::peer_blob_inventory
            .filter(inv::blob_hash.eq(blob_hash))
            .select((inv::peer_id, inv::last_seen_at))
            .load::<(String, String)>(&mut conn)?
    };

    // Deduplicate by peer_id (composite PK is (peer_id, blob_hash) but load
    // may return multiple rows if the schema ever evolves — be defensive).
    let mut seen_peers: HashSet<String> = HashSet::new();
    let mut max_last_seen: Option<String> = None;
    for (peer_id, last_seen) in &inventory_rows {
        seen_peers.insert(peer_id.clone());
        match &max_last_seen {
            None => max_last_seen = Some(last_seen.clone()),
            Some(current) if last_seen > current => {
                max_last_seen = Some(last_seen.clone());
            }
            _ => {}
        }
    }
    let replica_count = seen_peers.len() as u32;

    // Parse ISO8601 → seconds since Unix epoch. On any parse failure default to 0.
    let last_verified_seconds: u64 = max_last_seen
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp().max(0) as u64)
        .unwrap_or(0);

    // --- reach_class from content table ---
    let reach_str: Option<String> = {
        use crate::db::diesel_schema::content::dsl as c;
        c::content
            .filter(c::blob_hash.eq(blob_hash))
            .select(c::reach)
            .first::<String>(&mut conn)
            .optional()?
    };
    let reach_class = reach_str
        .as_deref()
        .and_then(parse_reach_class)
        .unwrap_or(ReachClass::Private);

    // --- replica_target and health ---
    let replica_target = replica_target_for(&reach_class);
    let replica_health = replica_health_for(replica_count, replica_target);

    // --- projector_count: stubbed ---
    // TODO(Phase 4 follow-up): wire projector identity once doorway_projector_acks lands
    let projector_count: u32 = 0;

    // --- diversity_hint: stubbed ---
    // TODO(Phase 4 follow-up): compose from peer geo/archetype index
    let diversity_hint = DiversityHint::None;

    // --- this_fetch_source: constant for T23 ---
    let this_fetch_source = FetchSource::ProjectedViaDoorway;

    // --- my_role and reciprocity_hint: context-dependent ---
    let (my_role, reciprocity_hint) = match &ctx {
        DistributionContext::Visitor => (None, None),
        DistributionContext::Steward {
            agent_cid: _,
            bindings,
        } => {
            // Build set of this steward's peer_ids from their bindings.
            let my_peers: HashSet<&str> = bindings.iter().map(|b| b.peer_id.as_str()).collect();

            // Determine my_role by intersecting with inventory replica set.
            let any_replica = seen_peers.iter().any(|p| my_peers.contains(p.as_str()));
            // TODO(Phase 4 follow-up): any_projector when doorway_projector_acks lands
            let any_projector = false;

            let role = if any_replica && any_projector {
                MyRole::ReplicaAndProjector
            } else if any_replica && replica_count == 1 {
                MyRole::SoleReplica
            } else if any_replica {
                MyRole::Replica
            } else {
                MyRole::NotHosting
            };

            // --- reciprocity_hint from rea_commitments ---
            let hint = compute_reciprocity(&mut conn, &my_peers).unwrap_or(0);

            (Some(role), Some(hint))
        }
    };

    Ok(DistributionSummary {
        replica_count,
        replica_target,
        replica_health,
        projector_count,
        reach_class,
        diversity_hint,
        this_fetch_source,
        last_verified_seconds,
        my_role,
        reciprocity_hint,
    })
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Parse a reach string from the `content.reach` column into `ReachClass`.
fn parse_reach_class(s: &str) -> Option<ReachClass> {
    match s {
        "private" => Some(ReachClass::Private),
        "intimate" => Some(ReachClass::Intimate),
        "household" => Some(ReachClass::Household),
        "neighborhood" => Some(ReachClass::Neighborhood),
        "collective" => Some(ReachClass::Collective),
        "community" => Some(ReachClass::Community),
        "district" => Some(ReachClass::District),
        "public" => Some(ReachClass::Public),
        _ => None,
    }
}

/// Compute the steward's reciprocity hint:
///   outflow = SUM(resource_quantity_value) WHERE provider IN (my_peer_ids)
///   inflow  = SUM(resource_quantity_value) WHERE receiver IN (my_peer_ids)
///   result  = (outflow - inflow).floor() as i64
///
/// NULL resource_quantity_value treated as 0.
/// On any error, returns 0.
fn compute_reciprocity(
    conn: &mut diesel::SqliteConnection,
    my_peers: &HashSet<&str>,
) -> Result<i64, diesel::result::Error> {
    if my_peers.is_empty() {
        return Ok(0);
    }

    let peer_list: Vec<&str> = my_peers.iter().copied().collect();

    // SUM of outflow (provider IN my_peers, action='custody-blob')
    let outflow: f64 = {
        use crate::db::diesel_schema::rea_commitments::dsl as rc;
        rc::rea_commitments
            .filter(rc::action.eq("custody-blob"))
            .filter(rc::provider.eq_any(&peer_list))
            .select(sql::<Nullable<Float>>("SUM(resource_quantity_value)"))
            .first::<Option<f32>>(conn)?
            .map(|v| v as f64)
            .unwrap_or(0.0)
    };

    // SUM of inflow (receiver IN my_peers, action='custody-blob')
    let inflow: f64 = {
        use crate::db::diesel_schema::rea_commitments::dsl as rc;
        rc::rea_commitments
            .filter(rc::action.eq("custody-blob"))
            .filter(rc::receiver.eq_any(&peer_list))
            .select(sql::<Nullable<Float>>("SUM(resource_quantity_value)"))
            .first::<Option<f32>>(conn)?
            .map(|v| v as f64)
            .unwrap_or(0.0)
    };

    Ok((outflow - inflow).floor() as i64)
}
