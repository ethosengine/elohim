//! Distribution view service — compose `DistributionSummary` from substrate state.
//!
//! ## Source of Truth
//!
//! Operational (Category C) per the p2p-design-gate output in
//! `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! Composes a read-projection from notarized DHT state. The DHT remains canonical.
//! No SQLite table here is authoritative.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{Float, Nullable};
use thiserror::Error;

use crate::db::models::PeerIdentityBindingRow;
use crate::db::{AppContext, DbPool};
use crate::views::{
    DeviceArchetype, DistributionDetails, DistributionSummary, FetchSource, JsonVal, MyRole,
    PlacementGapKind, PlacementGapRow, PlacementGapShortfall, ProjectorIdentity, ReachClass,
    ReplicaHealth, ReplicaPeer,
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
/// - `projector_count`: count of distinct doorway projectors that have acked
///   this blob, computed via `projection_events::distinct_projectors_for_blob`
///   (see body at `:163–167`).
/// - `diversity_hint`: derived from `peer_identity_bindings.device_archetype`
///   via `peer_diversity::diversity_hint_from_archetype_strs` (see body at
///   `:169–183`). Returns `None` when no archetype-tagged peers are known.
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

    // --- projector_count: from projection_events distinct projectors ---
    let projector_count =
        crate::db::projection_events::distinct_projectors_for_blob(&mut conn, blob_hash)
            .map(|v| v.len() as u32)
            .unwrap_or(0);

    // --- diversity_hint: archetype-mix over peer_identity_bindings for seen peers ---
    let archetypes_in_replicas: Vec<String> = {
        use crate::db::diesel_schema::peer_identity_bindings::dsl as bind;
        let seen_vec: Vec<&str> = seen_peers.iter().map(String::as_str).collect();
        bind::peer_identity_bindings
            .filter(bind::peer_id.eq_any(&seen_vec))
            .filter(bind::superseded_by.is_null())
            .select(bind::device_archetype)
            .distinct()
            .load::<String>(&mut conn)
            .unwrap_or_default()
    };
    let diversity_hint = crate::services::peer_diversity::diversity_hint_from_archetype_strs(
        &archetypes_in_replicas,
    );

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
            // any_projector: check if any of my agent_cids appear in projection_events
            let my_agent_cids: HashSet<&str> =
                bindings.iter().map(|b| b.agent_cid.as_str()).collect();
            let any_projector =
                crate::db::projection_events::distinct_projectors_for_blob(&mut conn, blob_hash)
                    .map(|projectors| {
                        projectors
                            .iter()
                            .any(|p| my_agent_cids.contains(p.as_str()))
                    })
                    .unwrap_or(false);

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

// ============================================================================
// compose_distribution_details (T24)
// ============================================================================

/// Compose a `DistributionDetails` — a lazy-fetched superset of the summary
/// for developer/operator drill-down. Stubs `projector_identities` and
/// `recent_projection_events` until those substrate sources land.
pub async fn compose_distribution_details(
    pool: &DbPool,
    blob_hash: &str,
    ctx: DistributionContext<'_>,
) -> Result<DistributionDetails, DistributionViewError> {
    // Fan-in: build the summary first (reuses T23's logic).
    let summary_ctx = ctx_for_summary(&ctx);
    let summary = compose_distribution_summary(pool, blob_hash, summary_ctx).await?;

    let mut conn = pool
        .get()
        .map_err(|e| DistributionViewError::Pool(e.to_string()))?;

    let replica_peers = load_replica_peers_full(&mut conn, blob_hash)?;

    // projector_identities: from distinct_projectors_for_blob; use agent_cid as
    // doorway_hostname (best available until a real hostname registry lands),
    // last_ack_seconds derived from most-recent emitted_at for each projector.
    let projector_identities: Vec<ProjectorIdentity> = {
        let projector_cids =
            crate::db::projection_events::distinct_projectors_for_blob(&mut conn, blob_hash)
                .unwrap_or_default();
        let now_seconds = chrono::Utc::now().timestamp().max(0) as u64;
        let mut out = Vec::with_capacity(projector_cids.len());
        for cid in projector_cids {
            // Find the most-recent ack for this projector.
            let recent_for_cid: Vec<crate::db::projection_events::ProjectionEventRow> = {
                use crate::db::diesel_schema::projection_events::dsl as pe;
                pe::projection_events
                    .filter(pe::blob_hash.eq(blob_hash))
                    .filter(pe::projector_agent_cid.eq(&cid))
                    .order(pe::emitted_at.desc())
                    .limit(1)
                    .load(&mut conn)
                    .unwrap_or_default()
            };
            let last_ack_seconds = recent_for_cid
                .first()
                .and_then(|r| DateTime::parse_from_rfc3339(&r.emitted_at).ok())
                .map(|dt| {
                    let secs = dt.timestamp().max(0) as u64;
                    now_seconds.saturating_sub(secs)
                })
                .unwrap_or(0);
            out.push(ProjectorIdentity {
                doorway_hostname: cid,
                last_ack_seconds,
                region_tier: None,
            });
        }
        out
    };

    let placement_gaps = load_placement_gaps_for(pool, &mut conn, blob_hash)?;

    // recent_projection_events: last 20 events as open-shape JSON.
    let recent_projection_events: Vec<JsonVal> = {
        let recent = crate::db::projection_events::list_recent_for_blob(&mut conn, blob_hash, 20)
            .unwrap_or_default();
        recent
            .into_iter()
            .map(|r| {
                JsonVal(serde_json::json!({
                    "blobHash": r.blob_hash,
                    "projectorAgentCid": r.projector_agent_cid,
                    "emittedAt": r.emitted_at,
                    "sourceActionHash": r.source_action_hash,
                }))
            })
            .collect()
    };

    let commitment_references = match &ctx {
        DistributionContext::Visitor => None,
        DistributionContext::Steward { bindings, .. } => {
            let my_peers: Vec<&str> = bindings.iter().map(|b| b.peer_id.as_str()).collect();
            Some(load_commitment_refs(&mut conn, blob_hash, &my_peers)?)
        }
    };

    Ok(DistributionDetails {
        summary,
        replica_peers,
        projector_identities,
        placement_gaps,
        recent_projection_events,
        commitment_references,
    })
}

/// Re-construct a `DistributionContext` for re-use without consuming `ctx`.
fn ctx_for_summary<'a>(ctx: &'a DistributionContext<'a>) -> DistributionContext<'a> {
    match ctx {
        DistributionContext::Visitor => DistributionContext::Visitor,
        DistributionContext::Steward {
            agent_cid,
            bindings,
        } => DistributionContext::Steward {
            agent_cid,
            bindings,
        },
    }
}

/// Load full `ReplicaPeer` rows for the given blob from `peer_blob_inventory`,
/// left-joined with `peer_identity_bindings` to pick up `device_archetype`.
///
/// Multiple binding rows for the same `peer_id` (different agent_cids sharing
/// a node) are deduplicated: the first archetype returned by the LEFT JOIN is
/// used. If no binding row exists for a peer, `DeviceArchetype::Node` is used.
fn load_replica_peers_full(
    conn: &mut diesel::SqliteConnection,
    blob_hash: &str,
) -> Result<Vec<ReplicaPeer>, DistributionViewError> {
    use crate::db::diesel_schema::peer_blob_inventory::dsl as inv;
    use crate::db::diesel_schema::peer_identity_bindings::dsl as bind;

    let rows: Vec<(String, String, Option<String>)> = inv::peer_blob_inventory
        .filter(inv::blob_hash.eq(blob_hash))
        .left_join(bind::peer_identity_bindings.on(bind::peer_id.eq(inv::peer_id)))
        .select((
            inv::peer_id,
            inv::last_seen_at,
            bind::device_archetype.nullable(),
        ))
        .load::<(String, String, Option<String>)>(conn)?;

    // Deduplicate by peer_id (the LEFT JOIN can multiply rows when a peer has
    // bindings under multiple agent_cids).
    let mut seen: HashSet<String> = HashSet::new();
    let now_seconds = Utc::now().timestamp().max(0) as u64;
    let mut out: Vec<ReplicaPeer> = Vec::new();
    for (peer_id, last_seen_at, archetype) in rows {
        if !seen.insert(peer_id.clone()) {
            continue;
        }
        let last_seen_seconds = DateTime::parse_from_rfc3339(&last_seen_at)
            .map(|dt| {
                let secs = dt.timestamp().max(0) as u64;
                now_seconds.saturating_sub(secs)
            })
            .unwrap_or(0);
        out.push(ReplicaPeer {
            peer_id,
            device_archetype: parse_archetype(archetype.as_deref().unwrap_or("node")),
            last_seen_seconds,
            hop_hint: None,
            household_id: None,
            region_tier: None,
        });
    }
    Ok(out)
}

/// Parse a `device_archetype` string from the `peer_identity_bindings` row
/// into the `DeviceArchetype` enum. Defaults to `Node` for unrecognized
/// values (matches the schema's default-on-backfill behaviour).
fn parse_archetype(s: &str) -> DeviceArchetype {
    match s {
        "desktop" => DeviceArchetype::Desktop,
        "mobile" => DeviceArchetype::Mobile,
        "steward" => DeviceArchetype::Steward,
        _ => DeviceArchetype::Node,
    }
}

/// Load hub-abstract `PlacementGapRow` view rows for the given blob_hash.
///
/// Calls `services::resilience::hub_summary` (C2) to classify peers into hubs,
/// then emits a `HubDiversity` gap row when the observed hub count falls below
/// the target. The target is currently a fixed floor of 2 — substrate-honest
/// for any content that should survive the loss of a single hub.
///
/// TODO(reach-aware-targeting): replace the fixed `TARGET_HUBS = 2` with a
/// value derived from the content's declared reach class on the EPR head.
/// Wider reach classes should require proportionally more hubs (analogous to
/// how `replica_target_for` scales with `ReachClass`). C4's job is to make
/// hub-diversity gap emission real; reach-aware tuning is a follow-up task.
///
/// `ReplicaCount` and `ReachClass` gap kinds are separate tasks and are not
/// emitted here.
fn load_placement_gaps_for(
    pool: &DbPool,
    _conn: &mut diesel::SqliteConnection,
    blob_hash: &str,
) -> Result<Vec<PlacementGapRow>, DistributionViewError> {
    // Minimum distinct hubs required for content to survive a single-hub failure.
    // Fixed floor for C4; reach-class-aware tuning is a follow-up (see TODO above).
    const TARGET_HUBS: i32 = 2;

    // Use the default lamad AppContext — distribution_view does not carry its own
    // AppContext today. hub_summary uses it only for future multi-tenant scoping;
    // the hub classification queries are not yet app-id-filtered.
    let ctx = AppContext::default_lamad();

    let hubs = crate::services::resilience::hub_summary(pool, &ctx, blob_hash).unwrap_or_default(); // permissive: lookup failure → treat as zero hubs

    let observed = hubs.len() as i32;
    let mut gaps = Vec::new();

    if observed < TARGET_HUBS {
        gaps.push(PlacementGapRow {
            kind: PlacementGapKind::HubDiversity,
            content_id: blob_hash.to_string(),
            shortfall: PlacementGapShortfall {
                target: TARGET_HUBS,
                observed,
            },
            remediation: Some("Recruit a replica in another hub".to_string()),
        });
    }

    Ok(gaps)
}

/// Load rea_commitment IDs where (`action='custody-blob'` AND
/// `resource_classified_as=blob_hash`) AND (provider OR receiver is one of my_peers).
fn load_commitment_refs(
    conn: &mut diesel::SqliteConnection,
    blob_hash: &str,
    my_peers: &[&str],
) -> Result<Vec<String>, DistributionViewError> {
    if my_peers.is_empty() {
        return Ok(vec![]);
    }
    use crate::db::diesel_schema::rea_commitments::dsl as rc;
    let rows: Vec<String> = rc::rea_commitments
        .filter(rc::action.eq("custody-blob"))
        .filter(rc::resource_classified_as.eq(blob_hash))
        .filter(
            rc::provider
                .eq_any(my_peers)
                .or(rc::receiver.eq_any(my_peers)),
        )
        .select(rc::id)
        .load::<String>(conn)?;
    Ok(rows)
}

// ============================================================================
// Internal helpers (T23)
// ============================================================================

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
