//! Stewarded node CRUD operations using Diesel
//!
//! Manages the physical node registry and node–human stewardship relationships.
//! Nodes are registered on the DHT and projected here for fast local queries.

use diesel::prelude::*;
use uuid::Uuid;

use super::diesel_schema::{node_stewardship, stewarded_nodes};
use super::models::{
    current_timestamp, NewNodeStewardship, NewStewardedNode, NodeStewardship, StewardedNode,
};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for registering a new stewarded node
#[derive(Debug, Clone)]
pub struct CreateStewardedNodeInput {
    pub id: String,
    pub display_name: String,
    pub claim_status: String,
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    pub bandwidth_mbps: i32,
    pub steward_tier: String,
    pub custodian_opt_in: i32,
    pub region: Option<String>,
    pub context_epr_id: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub h_app_id: String,
}

/// Input for creating a stewardship relationship between a node and a human
#[derive(Debug, Clone)]
pub struct CreateNodeStewardshipInput {
    pub node_id: String,
    pub human_id: String,
    pub affinity_score: f64,
    pub relationship: String,
    pub context_epr_id: Option<String>,
}

// ============================================================================
// CRUD Operations — StewardedNode
// ============================================================================

/// Outcome of `create_stewarded_node`.
///
/// A duplicate registration of an existing content-addressed/deterministic
/// row is not an internal error (concern canon C6b idempotent effect, C10
/// contract honesty): the caller gets back the existing row, discriminated
/// by whether the resubmitted payload matches what is already on record.
#[derive(Debug, Clone)]
pub enum StewardedNodeOutcome {
    /// No row existed for this id — a fresh row was inserted.
    Created(StewardedNode),
    /// A row already existed and the caller's payload matches its
    /// content-bearing fields byte-for-byte — safe to treat as success.
    ExistingMatch(StewardedNode),
    /// A row already existed but the caller's payload disagrees with it on
    /// at least one content-bearing field — a genuine conflict, not a retry.
    ExistingConflict(StewardedNode),
}

impl StewardedNodeOutcome {
    /// Unwrap to the row regardless of outcome — convenience for callers
    /// (tests, upstream callers) that only need the row.
    pub fn into_row(self) -> StewardedNode {
        match self {
            Self::Created(n) | Self::ExistingMatch(n) | Self::ExistingConflict(n) => n,
        }
    }
}

/// Register a stewarded node idempotently.
///
/// Returns `StewardedNodeOutcome::Created` on first registration. A
/// resubmission of the same `id` is NOT an internal error — SQLite's
/// `UNIQUE constraint failed` is caught at the `INSERT` via
/// `ON CONFLICT DO NOTHING`, then the existing row is compared against the
/// caller's payload: an identical resubmission comes back as
/// `ExistingMatch` (map to HTTP 200), a payload that disagrees on a
/// content-bearing field comes back as `ExistingConflict` (map to HTTP 409).
pub fn create_stewarded_node(
    conn: &mut SqliteConnection,
    input: CreateStewardedNodeInput,
) -> Result<StewardedNodeOutcome, StorageError> {
    let id = if input.id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        input.id.clone()
    };

    let now = current_timestamp();

    let new_node = NewStewardedNode {
        id: id.clone(),
        display_name: input.display_name.clone(),
        claim_status: input.claim_status.clone(),
        cpu_cores: input.cpu_cores,
        memory_gb: input.memory_gb,
        storage_tb: input.storage_tb,
        bandwidth_mbps: input.bandwidth_mbps,
        steward_tier: input.steward_tier.clone(),
        custodian_opt_in: input.custodian_opt_in,
        region: input.region.clone(),
        context_epr_id: input.context_epr_id.clone(),
        dht_anchor_hash: input.dht_anchor_hash.clone(),
        h_app_id: input.h_app_id.clone(),
        // Archetype/household fields populated by Task C5 upsert_from_shape;
        // legacy callers leave them null/default.
        device_archetype_id: None,
        household_id: None,
        hostname: None,
        node_role: None,
        capability_level: None,
        can_steward: 0,
        can_infer: 0,
        can_doorway: 0,
        signature: None,
        signed_at: None,
    };

    let inserted = diesel::insert_into(stewarded_nodes::table)
        .values(&new_node)
        .on_conflict(stewarded_nodes::id)
        .do_nothing()
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to insert stewarded node: {}", e)))?;

    if inserted == 1 {
        // Set created_at / updated_at via an immediate update so the returned row has timestamps.
        // (Diesel insert_into doesn't support DEFAULT expressions for SQLite TEXT columns.)
        diesel::update(stewarded_nodes::table.filter(stewarded_nodes::id.eq(&id)))
            .set((
                stewarded_nodes::created_at.eq(&now),
                stewarded_nodes::updated_at.eq(&now),
            ))
            .execute(conn)
            .map_err(|e| {
                StorageError::Internal(format!("Failed to set timestamps on stewarded node: {}", e))
            })?;

        let row = get_stewarded_node_by_id(conn, &id)?.ok_or_else(|| {
            StorageError::Internal("StewardedNode not found after insert".to_string())
        })?;
        return Ok(StewardedNodeOutcome::Created(row));
    }

    // ON CONFLICT DO NOTHING fired — a row with this id already exists.
    // Compare the caller's payload against the existing row's
    // content-bearing fields. dht_anchor_hash and h_app_id are excluded:
    // they are context-injected (zome-commit / app_ctx), never part of the
    // client's registration payload.
    let existing = get_stewarded_node_by_id(conn, &id)?.ok_or_else(|| {
        StorageError::Internal("StewardedNode disappeared after conflicting insert".to_string())
    })?;

    let matches = existing.display_name == input.display_name
        && existing.claim_status == input.claim_status
        && existing.cpu_cores == input.cpu_cores
        && existing.memory_gb == input.memory_gb
        && existing.storage_tb == input.storage_tb
        && existing.bandwidth_mbps == input.bandwidth_mbps
        && existing.steward_tier == input.steward_tier
        && existing.custodian_opt_in == input.custodian_opt_in
        && existing.region == input.region
        && existing.context_epr_id == input.context_epr_id;

    if matches {
        Ok(StewardedNodeOutcome::ExistingMatch(existing))
    } else {
        Ok(StewardedNodeOutcome::ExistingConflict(existing))
    }
}

/// Retrieve a stewarded node by its stable ID.
pub fn get_stewarded_node_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<StewardedNode>, StorageError> {
    stewarded_nodes::table
        .filter(stewarded_nodes::id.eq(id))
        .first::<StewardedNode>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch stewarded node by id: {}", e)))
}

/// List stewarded nodes, optionally filtered by `claim_status`.
///
/// Results are ordered by `created_at` descending (newest first).
pub fn list_stewarded_nodes(
    conn: &mut SqliteConnection,
    claim_status: Option<&str>,
) -> Result<Vec<StewardedNode>, StorageError> {
    let mut query = stewarded_nodes::table.into_boxed();

    if let Some(status) = claim_status {
        query = query.filter(stewarded_nodes::claim_status.eq(status));
    }

    query
        .order(stewarded_nodes::created_at.desc())
        .load::<StewardedNode>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list stewarded nodes: {}", e)))
}

/// Upsert a stewarded node from a boot-time NodeShapeView. Preserves
/// `dht_anchor_hash` if already present (signal projection fills it later).
///
/// Source of truth: the corresponding `NodeRegistration` DHT entry authored
/// by the node's agent key. This is the operational projection.
pub fn upsert_from_shape(
    conn: &mut SqliteConnection,
    view: &crate::views::NodeShapeView,
) -> Result<StewardedNode, StorageError> {
    use crate::db::diesel_schema::stewarded_nodes as t;

    let now = current_timestamp();

    if get_stewarded_node_by_id(conn, &view.node_id)?.is_some() {
        diesel::update(t::table.filter(t::id.eq(&view.node_id)))
            .set((
                t::display_name.eq(&view.hostname),
                t::hostname.eq(&view.hostname),
                t::device_archetype_id.eq(&view.device_archetype_id),
                t::household_id.eq(&view.household_id),
                t::node_role.eq(&view.role),
                t::capability_level.eq(view.capability_level),
                t::cpu_cores.eq(view.committed.cpu_cores),
                t::memory_gb.eq(view.committed.memory_gb),
                t::storage_tb.eq(view.committed.storage_tb),
                t::bandwidth_mbps.eq(view.committed.bandwidth_mbps.unwrap_or(0)),
                t::can_steward.eq(view.committed.can_steward as i32),
                t::can_infer.eq(view.committed.can_infer as i32),
                t::can_doorway.eq(view.committed.can_doorway as i32),
                t::steward_tier.eq(view
                    .steward_tier
                    .clone()
                    .unwrap_or_else(|| "caretaker".into())),
                t::custodian_opt_in.eq(view.custodian_opt_in as i32),
                t::region.eq(view.region.clone()),
                t::signature.eq(Some(view.signature.clone())),
                t::signed_at.eq(Some(view.signed_at.clone())),
                t::updated_at.eq(&now),
            ))
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("upsert_from_shape update: {}", e)))?;
    } else {
        let new_node = NewStewardedNode {
            id: view.node_id.clone(),
            display_name: view.hostname.clone(),
            claim_status: "claimed".into(),
            cpu_cores: view.committed.cpu_cores,
            memory_gb: view.committed.memory_gb,
            storage_tb: view.committed.storage_tb,
            bandwidth_mbps: view.committed.bandwidth_mbps.unwrap_or(0),
            steward_tier: view
                .steward_tier
                .clone()
                .unwrap_or_else(|| "caretaker".into()),
            custodian_opt_in: view.custodian_opt_in as i32,
            region: view.region.clone(),
            context_epr_id: None,
            dht_anchor_hash: view.dht_anchor_hash.clone(),
            h_app_id: "elohim".into(),
            device_archetype_id: Some(view.device_archetype_id.clone()),
            household_id: Some(view.household_id.clone()),
            hostname: Some(view.hostname.clone()),
            node_role: Some(view.role.clone()),
            capability_level: Some(view.capability_level),
            can_steward: view.committed.can_steward as i32,
            can_infer: view.committed.can_infer as i32,
            can_doorway: view.committed.can_doorway as i32,
            signature: Some(view.signature.clone()),
            signed_at: Some(view.signed_at.clone()),
        };
        diesel::insert_into(stewarded_nodes::table)
            .values(&new_node)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("upsert_from_shape insert: {}", e)))?;
        diesel::update(t::table.filter(t::id.eq(&view.node_id)))
            .set((t::created_at.eq(&now), t::updated_at.eq(&now)))
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("upsert_from_shape timestamps: {}", e)))?;
    }

    get_stewarded_node_by_id(conn, &view.node_id)?
        .ok_or_else(|| StorageError::Internal("Row missing after upsert".into()))
}

/// Persist the DHT anchor hash after the zome commit completes.
pub fn set_dht_anchor(
    conn: &mut SqliteConnection,
    node_id: &str,
    anchor_hash: &str,
) -> Result<(), StorageError> {
    use crate::db::diesel_schema::stewarded_nodes as t;
    diesel::update(t::table.filter(t::id.eq(node_id)))
        .set((
            t::dht_anchor_hash.eq(Some(anchor_hash.to_string())),
            t::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("set_dht_anchor: {}", e)))?;
    Ok(())
}

/// Count distinct households with at least one active (non-stale) peer.
/// Used by `/api/v1/network/posture` for the reciprocation metric.
pub fn distinct_active_households(
    conn: &mut SqliteConnection,
    stale_threshold_secs: i64,
    now_secs: i64,
) -> Result<i64, StorageError> {
    use crate::db::diesel_schema::peer_statuses as p;
    use crate::db::diesel_schema::stewarded_nodes as t;
    use diesel::dsl::sql;
    use diesel::sql_types::BigInt;

    let cutoff = now_secs - stale_threshold_secs;

    // COUNT(DISTINCT household_id) via a raw sql expression — diesel's
    // .distinct().count() would count rows, not distinct values.
    t::table
        .inner_join(p::table.on(p::peer_id.eq(t::id)))
        .filter(t::household_id.is_not_null())
        .filter(p::updated_at.gt(cutoff))
        .filter(p::status.eq_any(["online", "degraded"]))
        .select(sql::<BigInt>(
            "COUNT(DISTINCT stewarded_nodes.household_id)",
        ))
        .first::<i64>(conn)
        .map_err(|e| StorageError::Internal(format!("distinct_active_households: {}", e)))
}

/// List household devices joining `peer_statuses` for live vitals.
///
/// Returns rows as (stewarded_node, optional peer_status) tuples. Nodes with
/// no recent PeerStatus publication render as "offline" in the view.
pub fn list_by_household_with_peer_status(
    conn: &mut SqliteConnection,
    household_id: &str,
) -> Result<
    Vec<(
        StewardedNode,
        Option<crate::db::peer_statuses::PeerStatusRow>,
    )>,
    StorageError,
> {
    use crate::db::diesel_schema::peer_statuses as p;
    use crate::db::diesel_schema::stewarded_nodes as t;

    t::table
        .left_join(p::table.on(p::peer_id.eq(t::id)))
        .filter(t::household_id.eq(household_id))
        .select((StewardedNode::as_select(), p::all_columns.nullable()))
        .load::<(
            StewardedNode,
            Option<crate::db::peer_statuses::PeerStatusRow>,
        )>(conn)
        .map_err(|e| StorageError::Internal(format!("list_by_household_with_peer_status: {}", e)))
}

// ============================================================================
// CRUD Operations — NodeStewardship
// ============================================================================

/// Outcome of `create_node_stewardship`. Mirrors `StewardedNodeOutcome` — a
/// duplicate `(node_id, human_id)` submission is not an internal error
/// (concern canon C6b idempotent effect, C10 contract honesty).
#[derive(Debug, Clone)]
pub enum NodeStewardshipOutcome {
    /// No row existed for this `(node_id, human_id)` pair — freshly inserted.
    Created(NodeStewardship),
    /// A row already existed and the caller's payload matches its
    /// content-bearing fields — safe to treat as success.
    ExistingMatch(NodeStewardship),
    /// A row already existed but disagrees with the caller's payload on at
    /// least one content-bearing field — a genuine conflict.
    ExistingConflict(NodeStewardship),
}

impl NodeStewardshipOutcome {
    /// Unwrap to the row regardless of outcome.
    pub fn into_row(self) -> NodeStewardship {
        match self {
            Self::Created(s) | Self::ExistingMatch(s) | Self::ExistingConflict(s) => s,
        }
    }
}

/// Grant a node stewardship relationship idempotently.
///
/// The composite primary key is `(node_id, human_id)`. A resubmission of the
/// same pair is caught at the `INSERT` via `ON CONFLICT DO NOTHING` rather
/// than surfacing SQLite's `UNIQUE constraint failed` as an internal error;
/// the existing row is then compared against the caller's payload to
/// discriminate `ExistingMatch` (HTTP 200) from `ExistingConflict` (HTTP 409).
pub fn create_node_stewardship(
    conn: &mut SqliteConnection,
    input: CreateNodeStewardshipInput,
) -> Result<NodeStewardshipOutcome, StorageError> {
    let now = current_timestamp();

    let new_rel = NewNodeStewardship {
        node_id: input.node_id.clone(),
        human_id: input.human_id.clone(),
        affinity_score: input.affinity_score,
        relationship: input.relationship.clone(),
        context_epr_id: input.context_epr_id.clone(),
    };

    let inserted = diesel::insert_into(node_stewardship::table)
        .values(&new_rel)
        .on_conflict((node_stewardship::node_id, node_stewardship::human_id))
        .do_nothing()
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to insert node stewardship: {}", e)))?;

    if inserted == 1 {
        // Set granted_at timestamp
        diesel::update(
            node_stewardship::table
                .filter(node_stewardship::node_id.eq(&input.node_id))
                .filter(node_stewardship::human_id.eq(&input.human_id)),
        )
        .set(node_stewardship::granted_at.eq(&now))
        .execute(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to set granted_at on node stewardship: {}",
                e
            ))
        })?;

        let row =
            get_node_stewardship(conn, &input.node_id, &input.human_id)?.ok_or_else(|| {
                StorageError::Internal("NodeStewardship not found after insert".to_string())
            })?;
        return Ok(NodeStewardshipOutcome::Created(row));
    }

    // ON CONFLICT DO NOTHING fired — a row for this (node_id, human_id) pair
    // already exists. Compare content-bearing fields against the caller's
    // payload (granted_at is a server-stamped timestamp, not payload).
    let existing =
        get_node_stewardship(conn, &input.node_id, &input.human_id)?.ok_or_else(|| {
            StorageError::Internal(
                "NodeStewardship disappeared after conflicting insert".to_string(),
            )
        })?;

    let matches = existing.affinity_score == input.affinity_score
        && existing.relationship == input.relationship
        && existing.context_epr_id == input.context_epr_id;

    if matches {
        Ok(NodeStewardshipOutcome::ExistingMatch(existing))
    } else {
        Ok(NodeStewardshipOutcome::ExistingConflict(existing))
    }
}

/// Retrieve a specific stewardship record by `(node_id, human_id)`.
pub fn get_node_stewardship(
    conn: &mut SqliteConnection,
    node_id: &str,
    human_id: &str,
) -> Result<Option<NodeStewardship>, StorageError> {
    node_stewardship::table
        .filter(node_stewardship::node_id.eq(node_id))
        .filter(node_stewardship::human_id.eq(human_id))
        .first::<NodeStewardship>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch node stewardship: {}", e)))
}

/// List all stewards for a given node, ordered by affinity score descending.
pub fn list_stewards_for_node(
    conn: &mut SqliteConnection,
    node_id: &str,
) -> Result<Vec<NodeStewardship>, StorageError> {
    node_stewardship::table
        .filter(node_stewardship::node_id.eq(node_id))
        .order(node_stewardship::affinity_score.desc())
        .load::<NodeStewardship>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list stewards for node: {}", e)))
}

/// List all nodes stewarded by a given human, ordered by affinity score descending.
pub fn list_nodes_for_human(
    conn: &mut SqliteConnection,
    human_id: &str,
) -> Result<Vec<NodeStewardship>, StorageError> {
    node_stewardship::table
        .filter(node_stewardship::human_id.eq(human_id))
        .order(node_stewardship::affinity_score.desc())
        .load::<NodeStewardship>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list nodes for human: {}", e)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::peer_statuses::{upsert as upsert_peer, PeerStatusRow};
    use crate::test_util::test_pool;

    /// Insert a stewarded node with a given (optional) household_id.
    fn insert_node(conn: &mut SqliteConnection, id: &str, household: Option<&str>) {
        create_stewarded_node(
            conn,
            CreateStewardedNodeInput {
                id: id.to_string(),
                display_name: format!("node-{id}"),
                claim_status: "claimed".to_string(),
                cpu_cores: 4,
                memory_gb: 8,
                storage_tb: 1.0,
                bandwidth_mbps: 100,
                steward_tier: "household".to_string(),
                custodian_opt_in: 0,
                region: None,
                context_epr_id: None,
                dht_anchor_hash: Some(format!("uhCkk-anchor-{id}")),
                h_app_id: "lamad".to_string(),
            },
        )
        .expect("create node");
        if let Some(h) = household {
            diesel::update(stewarded_nodes::table.filter(stewarded_nodes::id.eq(id)))
                .set(stewarded_nodes::household_id.eq(h))
                .execute(conn)
                .expect("set household_id");
        }
    }

    /// Insert a human fixture row — `node_stewardship.human_id` carries a
    /// `REFERENCES humans(id)` FK, enforced under the test pool's SQLite
    /// connection, so stewardship tests need a real humans row to point at.
    fn insert_human(conn: &mut SqliteConnection, id: &str) {
        crate::db::humans::create_human(
            conn,
            crate::db::humans::CreateHumanInput {
                id: id.to_string(),
                agent_pub_key: None,
                display_name: format!("Human {id}"),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "public".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: "elohim".to_string(),
                household_id: None,
            },
        )
        .expect("create human fixture");
    }

    /// Insert a peer status row joined to a node id (peer_id == node id).
    fn insert_peer(conn: &mut SqliteConnection, peer_id: &str, status: &str, updated_at: i64) {
        upsert_peer(
            conn,
            &PeerStatusRow {
                peer_id: peer_id.to_string(),
                status: status.to_string(),
                general_pool_member: 0,
                accepting_stewardship_reserves: 0,
                archetype_class: None,
                timestamp: updated_at,
                dht_anchor_hash: format!("uhCkk-ps-{peer_id}"),
                updated_at,
            },
        )
        .expect("upsert peer status");
    }

    /// distinct_active_households counts DISTINCT non-null household_id over
    /// nodes that join an active (online/degraded, non-stale) peer status.
    #[test]
    fn distinct_active_households_counts_distinct_nonnull_over_active_nodes() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let now = 1_000_000_i64;
        let stale_threshold = 120_i64;
        let fresh = now - 10; // within window
        let stale = now - 500; // older than stale_threshold

        // Two nodes in household "eden", both active → counts as ONE distinct.
        insert_node(&mut conn, "n-eden-1", Some("eden"));
        insert_node(&mut conn, "n-eden-2", Some("eden"));
        insert_peer(&mut conn, "n-eden-1", "online", fresh);
        insert_peer(&mut conn, "n-eden-2", "degraded", fresh);

        // One node in household "ararat", active → second distinct household.
        insert_node(&mut conn, "n-ararat-1", Some("ararat"));
        insert_peer(&mut conn, "n-ararat-1", "online", fresh);

        // Node in household "sinai" but STALE peer status → excluded.
        insert_node(&mut conn, "n-sinai-1", Some("sinai"));
        insert_peer(&mut conn, "n-sinai-1", "online", stale);

        // Node in household "horeb" but OFFLINE status → excluded.
        insert_node(&mut conn, "n-horeb-1", Some("horeb"));
        insert_peer(&mut conn, "n-horeb-1", "offline", fresh);

        // Node with NULL household_id, active → excluded (non-null filter).
        insert_node(&mut conn, "n-null-1", None);
        insert_peer(&mut conn, "n-null-1", "online", fresh);

        let count = distinct_active_households(&mut conn, stale_threshold, now)
            .expect("distinct_active_households");

        assert_eq!(
            count, 2,
            "only 'eden' (deduped across 2 nodes) and 'ararat' are active+non-null; \
             stale/offline/null-household nodes excluded"
        );
    }

    /// With no active nodes carrying a household_id, the count is zero (honest
    /// absence, not a fake number).
    #[test]
    fn distinct_active_households_zero_when_none_qualify() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let now = 2_000_000_i64;
        // Node with household but no peer status row at all → no join → excluded.
        insert_node(&mut conn, "n-orphan", Some("eden"));

        let count =
            distinct_active_households(&mut conn, 120, now).expect("distinct_active_households");
        assert_eq!(count, 0, "a node with no joined peer status is not counted");
    }

    // ========================================================================
    // Idempotent registration — StewardedNode
    // ========================================================================

    /// Build a `CreateStewardedNodeInput` matching the exact seeder payload
    /// shape (see `genesis/seeder/src/seed-nodes.ts` `registerNode`), varied
    /// only by the fields the test cares about.
    fn seeder_node_input(id: &str, display_name: &str, cpu_cores: i32) -> CreateStewardedNodeInput {
        CreateStewardedNodeInput {
            id: id.to_string(),
            display_name: display_name.to_string(),
            claim_status: "unclaimed".to_string(),
            cpu_cores,
            memory_gb: 16,
            storage_tb: 2.0,
            bandwidth_mbps: 500,
            steward_tier: "household".to_string(),
            custodian_opt_in: 1,
            region: Some("us-east".to_string()),
            context_epr_id: Some("epr:node-context:1".to_string()),
            dht_anchor_hash: None,
            h_app_id: "elohim".to_string(),
        }
    }

    /// First registration of a node id creates the row.
    #[test]
    fn create_stewarded_node_first_call_creates() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let outcome = create_stewarded_node(&mut conn, seeder_node_input("n-alpha", "Alpha", 4))
            .expect("create_stewarded_node");

        match outcome {
            StewardedNodeOutcome::Created(row) => {
                assert_eq!(row.id, "n-alpha");
                assert_eq!(row.display_name, "Alpha");
            }
            other => panic!("expected Created, got {other:?}"),
        }
    }

    /// Regression: registering the exact same node payload twice (the seeder's
    /// re-run shape) must NOT surface a 500-class internal error — it must
    /// come back as an idempotent ExistingMatch carrying the same row, so a
    /// resumed/retried seed run does not fail.
    #[test]
    fn create_stewarded_node_duplicate_matching_payload_is_existing_match_not_error() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let first = create_stewarded_node(&mut conn, seeder_node_input("n-beta", "Beta", 8))
            .expect("first create_stewarded_node");
        let first_row = match first {
            StewardedNodeOutcome::Created(row) => row,
            other => panic!("expected Created on first call, got {other:?}"),
        };

        let second = create_stewarded_node(&mut conn, seeder_node_input("n-beta", "Beta", 8))
            .expect("duplicate create_stewarded_node must not error");

        match second {
            StewardedNodeOutcome::ExistingMatch(row) => {
                assert_eq!(row.id, first_row.id);
                assert_eq!(row.display_name, first_row.display_name);
                assert_eq!(row.cpu_cores, first_row.cpu_cores);
            }
            other => panic!("expected ExistingMatch, got {other:?}"),
        }
    }

    /// A resubmission under the same id but with a content-bearing field that
    /// disagrees with the row on record is a genuine conflict, not a silent
    /// no-op — the caller gets the existing row back so it can compare.
    #[test]
    fn create_stewarded_node_duplicate_conflicting_payload_is_existing_conflict() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        create_stewarded_node(&mut conn, seeder_node_input("n-gamma", "Gamma", 4))
            .expect("first create_stewarded_node");

        // Same id, different cpu_cores — a disagreeing resubmission.
        let second = create_stewarded_node(&mut conn, seeder_node_input("n-gamma", "Gamma", 16))
            .expect("conflicting create_stewarded_node must not error");

        match second {
            StewardedNodeOutcome::ExistingConflict(row) => {
                assert_eq!(row.id, "n-gamma");
                // The row on record still reflects the FIRST registration —
                // the conflicting resubmission never overwrote it.
                assert_eq!(row.cpu_cores, 4);
            }
            other => panic!("expected ExistingConflict, got {other:?}"),
        }
    }

    // ========================================================================
    // Idempotent registration — NodeStewardship
    // ========================================================================

    fn steward_input(
        node_id: &str,
        human_id: &str,
        affinity_score: f64,
        relationship: &str,
    ) -> CreateNodeStewardshipInput {
        CreateNodeStewardshipInput {
            node_id: node_id.to_string(),
            human_id: human_id.to_string(),
            affinity_score,
            relationship: relationship.to_string(),
            context_epr_id: Some("epr:stewardship-context:1".to_string()),
        }
    }

    #[test]
    fn create_node_stewardship_first_call_creates() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_node(&mut conn, "n-stew-1", None);
        insert_human(&mut conn, "h-matthew");

        let outcome = create_node_stewardship(
            &mut conn,
            steward_input("n-stew-1", "h-matthew", 0.9, "primary"),
        )
        .expect("create_node_stewardship");

        match outcome {
            NodeStewardshipOutcome::Created(row) => {
                assert_eq!(row.node_id, "n-stew-1");
                assert_eq!(row.human_id, "h-matthew");
            }
            other => panic!("expected Created, got {other:?}"),
        }
    }

    /// Regression: re-granting the exact same stewardship (node_id, human_id,
    /// payload) must not 500 — the seeder re-runs this call idempotently.
    #[test]
    fn create_node_stewardship_duplicate_matching_payload_is_existing_match_not_error() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_node(&mut conn, "n-stew-2", None);
        insert_human(&mut conn, "h-adam");

        create_node_stewardship(
            &mut conn,
            steward_input("n-stew-2", "h-adam", 0.75, "primary"),
        )
        .expect("first create_node_stewardship");

        let second = create_node_stewardship(
            &mut conn,
            steward_input("n-stew-2", "h-adam", 0.75, "primary"),
        )
        .expect("duplicate create_node_stewardship must not error");

        match second {
            NodeStewardshipOutcome::ExistingMatch(row) => {
                assert_eq!(row.node_id, "n-stew-2");
                assert_eq!(row.human_id, "h-adam");
                assert_eq!(row.affinity_score, 0.75);
            }
            other => panic!("expected ExistingMatch, got {other:?}"),
        }
    }

    /// A resubmission for the same (node_id, human_id) pair with a different
    /// affinity_score/relationship is a genuine conflict.
    #[test]
    fn create_node_stewardship_duplicate_conflicting_payload_is_existing_conflict() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_node(&mut conn, "n-stew-3", None);
        insert_human(&mut conn, "h-jane");

        create_node_stewardship(
            &mut conn,
            steward_input("n-stew-3", "h-jane", 0.5, "primary"),
        )
        .expect("first create_node_stewardship");

        let second = create_node_stewardship(
            &mut conn,
            steward_input("n-stew-3", "h-jane", 0.5, "secondary"),
        )
        .expect("conflicting create_node_stewardship must not error");

        match second {
            NodeStewardshipOutcome::ExistingConflict(row) => {
                assert_eq!(row.node_id, "n-stew-3");
                assert_eq!(row.human_id, "h-jane");
                // The row on record still reflects the FIRST registration.
                assert_eq!(row.relationship, "primary");
            }
            other => panic!("expected ExistingConflict, got {other:?}"),
        }
    }
}
