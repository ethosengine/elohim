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
    pub app_id: String,
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

/// Insert a new stewarded node record.
///
/// Returns the created `StewardedNode` row. Errors if the `id` already exists.
pub fn create_stewarded_node(
    conn: &mut SqliteConnection,
    input: CreateStewardedNodeInput,
) -> Result<StewardedNode, StorageError> {
    let id = if input.id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        input.id
    };

    let now = current_timestamp();

    let new_node = NewStewardedNode {
        id: id.clone(),
        display_name: input.display_name,
        claim_status: input.claim_status,
        cpu_cores: input.cpu_cores,
        memory_gb: input.memory_gb,
        storage_tb: input.storage_tb,
        bandwidth_mbps: input.bandwidth_mbps,
        steward_tier: input.steward_tier,
        custodian_opt_in: input.custodian_opt_in,
        region: input.region,
        context_epr_id: input.context_epr_id,
        dht_anchor_hash: input.dht_anchor_hash,
        app_id: input.app_id,
    };

    diesel::insert_into(stewarded_nodes::table)
        .values(&new_node)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to insert stewarded node: {}", e)))?;

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

    get_stewarded_node_by_id(conn, &id)?
        .ok_or_else(|| StorageError::Internal("StewardedNode not found after insert".to_string()))
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

// ============================================================================
// CRUD Operations — NodeStewardship
// ============================================================================

/// Insert a new node stewardship relationship.
///
/// The composite primary key is `(node_id, human_id)`. Returns the created row.
pub fn create_node_stewardship(
    conn: &mut SqliteConnection,
    input: CreateNodeStewardshipInput,
) -> Result<NodeStewardship, StorageError> {
    let now = current_timestamp();

    let new_rel = NewNodeStewardship {
        node_id: input.node_id.clone(),
        human_id: input.human_id.clone(),
        affinity_score: input.affinity_score,
        relationship: input.relationship,
        context_epr_id: input.context_epr_id,
    };

    diesel::insert_into(node_stewardship::table)
        .values(&new_rel)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to insert node stewardship: {}", e)))?;

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

    get_node_stewardship(conn, &input.node_id, &input.human_id)?
        .ok_or_else(|| StorageError::Internal("NodeStewardship not found after insert".to_string()))
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
