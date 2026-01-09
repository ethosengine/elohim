//! Knowledge Maps CRUD operations
//!
//! User's personalized domain maps for tracking learning progress and affinities.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::StorageError;

// =============================================================================
// Types
// =============================================================================

/// Knowledge map row from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMapRow {
    pub id: String,
    pub map_type: String,
    pub owner_id: String,
    pub title: String,
    pub description: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_name: String,
    pub visibility: String,
    pub shared_with_json: Option<String>,
    pub nodes_json: String,
    pub path_ids_json: Option<String>,
    pub overall_affinity: f64,
    pub content_graph_id: Option<String>,
    pub mastery_levels_json: Option<String>,
    pub goals_json: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a knowledge map
#[derive(Debug, Clone, Deserialize)]
pub struct CreateKnowledgeMapInput {
    pub id: Option<String>,
    pub map_type: String,
    pub owner_id: String,
    pub title: String,
    pub description: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_name: String,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    pub shared_with_json: Option<String>,
    pub nodes_json: String,
    pub path_ids_json: Option<String>,
    #[serde(default)]
    pub overall_affinity: f64,
    pub content_graph_id: Option<String>,
    pub mastery_levels_json: Option<String>,
    pub goals_json: Option<String>,
    pub metadata_json: Option<String>,
}

fn default_visibility() -> String { "private".to_string() }

/// Query parameters for listing knowledge maps
#[derive(Debug, Clone, Deserialize, Default)]
pub struct KnowledgeMapQuery {
    pub owner_id: Option<String>,
    pub map_type: Option<String>,
    pub subject_id: Option<String>,
    pub visibility: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 100 }

// =============================================================================
// CRUD Operations
// =============================================================================

/// Get a knowledge map by ID
pub fn get_knowledge_map(conn: &Connection, id: &str) -> Result<Option<KnowledgeMapRow>, StorageError> {
    let sql = "SELECT id, map_type, owner_id, title, description, subject_type, subject_id,
               subject_name, visibility, shared_with_json, nodes_json, path_ids_json,
               overall_affinity, content_graph_id, mastery_levels_json, goals_json,
               metadata_json, created_at, updated_at
               FROM knowledge_maps WHERE id = ?";

    conn.query_row(sql, params![id], |row| {
        Ok(KnowledgeMapRow {
            id: row.get(0)?,
            map_type: row.get(1)?,
            owner_id: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            subject_type: row.get(5)?,
            subject_id: row.get(6)?,
            subject_name: row.get(7)?,
            visibility: row.get(8)?,
            shared_with_json: row.get(9)?,
            nodes_json: row.get(10)?,
            path_ids_json: row.get(11)?,
            overall_affinity: row.get(12)?,
            content_graph_id: row.get(13)?,
            mastery_levels_json: row.get(14)?,
            goals_json: row.get(15)?,
            metadata_json: row.get(16)?,
            created_at: row.get(17)?,
            updated_at: row.get(18)?,
        })
    })
    .optional()
    .map_err(|e| StorageError::Internal(format!("Failed to get knowledge map: {}", e)))
}

/// List knowledge maps with filtering
pub fn list_knowledge_maps(conn: &Connection, query: &KnowledgeMapQuery) -> Result<Vec<KnowledgeMapRow>, StorageError> {
    let mut sql = String::from(
        "SELECT id, map_type, owner_id, title, description, subject_type, subject_id,
         subject_name, visibility, shared_with_json, nodes_json, path_ids_json,
         overall_affinity, content_graph_id, mastery_levels_json, goals_json,
         metadata_json, created_at, updated_at
         FROM knowledge_maps WHERE 1=1"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(owner_id) = &query.owner_id {
        sql.push_str(" AND owner_id = ?");
        params_vec.push(Box::new(owner_id.clone()));
    }

    if let Some(map_type) = &query.map_type {
        sql.push_str(" AND map_type = ?");
        params_vec.push(Box::new(map_type.clone()));
    }

    if let Some(subject_id) = &query.subject_id {
        sql.push_str(" AND subject_id = ?");
        params_vec.push(Box::new(subject_id.clone()));
    }

    if let Some(visibility) = &query.visibility {
        sql.push_str(" AND visibility = ?");
        params_vec.push(Box::new(visibility.clone()));
    }

    sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");
    params_vec.push(Box::new(query.limit));
    params_vec.push(Box::new(query.offset));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| StorageError::Internal(format!("Failed to prepare statement: {}", e)))?;

    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(KnowledgeMapRow {
            id: row.get(0)?,
            map_type: row.get(1)?,
            owner_id: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            subject_type: row.get(5)?,
            subject_id: row.get(6)?,
            subject_name: row.get(7)?,
            visibility: row.get(8)?,
            shared_with_json: row.get(9)?,
            nodes_json: row.get(10)?,
            path_ids_json: row.get(11)?,
            overall_affinity: row.get(12)?,
            content_graph_id: row.get(13)?,
            mastery_levels_json: row.get(14)?,
            goals_json: row.get(15)?,
            metadata_json: row.get(16)?,
            created_at: row.get(17)?,
            updated_at: row.get(18)?,
        })
    }).map_err(|e| StorageError::Internal(format!("Failed to query knowledge maps: {}", e)))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| StorageError::Internal(format!("Failed to read row: {}", e)))?);
    }

    Ok(results)
}

/// Create a new knowledge map
pub fn create_knowledge_map(conn: &mut Connection, input: CreateKnowledgeMapInput) -> Result<KnowledgeMapRow, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let sql = "INSERT INTO knowledge_maps (
               id, map_type, owner_id, title, description, subject_type, subject_id,
               subject_name, visibility, shared_with_json, nodes_json, path_ids_json,
               overall_affinity, content_graph_id, mastery_levels_json, goals_json, metadata_json
               ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

    conn.execute(sql, params![
        id,
        input.map_type,
        input.owner_id,
        input.title,
        input.description,
        input.subject_type,
        input.subject_id,
        input.subject_name,
        input.visibility,
        input.shared_with_json,
        input.nodes_json,
        input.path_ids_json,
        input.overall_affinity,
        input.content_graph_id,
        input.mastery_levels_json,
        input.goals_json,
        input.metadata_json,
    ]).map_err(|e| StorageError::Internal(format!("Failed to create knowledge map: {}", e)))?;

    get_knowledge_map(conn, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created knowledge map".to_string()))
}

/// Update a knowledge map
pub fn update_knowledge_map(conn: &mut Connection, id: &str, input: CreateKnowledgeMapInput) -> Result<KnowledgeMapRow, StorageError> {
    let sql = "UPDATE knowledge_maps SET
               map_type = ?, title = ?, description = ?, subject_type = ?, subject_id = ?,
               subject_name = ?, visibility = ?, shared_with_json = ?, nodes_json = ?,
               path_ids_json = ?, overall_affinity = ?, content_graph_id = ?,
               mastery_levels_json = ?, goals_json = ?, metadata_json = ?,
               updated_at = datetime('now')
               WHERE id = ?";

    let rows = conn.execute(sql, params![
        input.map_type,
        input.title,
        input.description,
        input.subject_type,
        input.subject_id,
        input.subject_name,
        input.visibility,
        input.shared_with_json,
        input.nodes_json,
        input.path_ids_json,
        input.overall_affinity,
        input.content_graph_id,
        input.mastery_levels_json,
        input.goals_json,
        input.metadata_json,
        id,
    ]).map_err(|e| StorageError::Internal(format!("Failed to update knowledge map: {}", e)))?;

    if rows == 0 {
        return Err(StorageError::NotFound(format!("Knowledge map not found: {}", id)));
    }

    get_knowledge_map(conn, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated knowledge map".to_string()))
}

/// Delete a knowledge map by ID
pub fn delete_knowledge_map(conn: &mut Connection, id: &str) -> Result<bool, StorageError> {
    let rows = conn.execute("DELETE FROM knowledge_maps WHERE id = ?", params![id])
        .map_err(|e| StorageError::Internal(format!("Failed to delete knowledge map: {}", e)))?;

    Ok(rows > 0)
}
