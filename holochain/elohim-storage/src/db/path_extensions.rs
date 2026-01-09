//! Path Extensions CRUD operations
//!
//! User customizations and forks of learning paths.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::StorageError;

// =============================================================================
// Types
// =============================================================================

/// Path extension row from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathExtensionRow {
    pub id: String,
    pub base_path_id: String,
    pub base_path_version: String,
    pub extended_by: String,
    pub title: String,
    pub description: Option<String>,
    pub insertions_json: Option<String>,
    pub annotations_json: Option<String>,
    pub reorderings_json: Option<String>,
    pub exclusions_json: Option<String>,
    pub visibility: String,
    pub shared_with_json: Option<String>,
    pub forked_from: Option<String>,
    pub forks_json: Option<String>,
    pub upstream_proposal_json: Option<String>,
    pub stats_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a path extension
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePathExtensionInput {
    pub id: Option<String>,
    pub base_path_id: String,
    pub base_path_version: String,
    pub extended_by: String,
    pub title: String,
    pub description: Option<String>,
    pub insertions_json: Option<String>,
    pub annotations_json: Option<String>,
    pub reorderings_json: Option<String>,
    pub exclusions_json: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    pub shared_with_json: Option<String>,
    pub forked_from: Option<String>,
    pub forks_json: Option<String>,
    pub upstream_proposal_json: Option<String>,
    pub stats_json: Option<String>,
}

fn default_visibility() -> String { "private".to_string() }

/// Query parameters for listing path extensions
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PathExtensionQuery {
    pub base_path_id: Option<String>,
    pub extended_by: Option<String>,
    pub visibility: Option<String>,
    pub forked_from: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 100 }

// =============================================================================
// CRUD Operations
// =============================================================================

/// Get a path extension by ID
pub fn get_path_extension(conn: &Connection, id: &str) -> Result<Option<PathExtensionRow>, StorageError> {
    let sql = "SELECT id, base_path_id, base_path_version, extended_by, title, description,
               insertions_json, annotations_json, reorderings_json, exclusions_json,
               visibility, shared_with_json, forked_from, forks_json, upstream_proposal_json,
               stats_json, created_at, updated_at
               FROM path_extensions WHERE id = ?";

    conn.query_row(sql, params![id], |row| {
        Ok(PathExtensionRow {
            id: row.get(0)?,
            base_path_id: row.get(1)?,
            base_path_version: row.get(2)?,
            extended_by: row.get(3)?,
            title: row.get(4)?,
            description: row.get(5)?,
            insertions_json: row.get(6)?,
            annotations_json: row.get(7)?,
            reorderings_json: row.get(8)?,
            exclusions_json: row.get(9)?,
            visibility: row.get(10)?,
            shared_with_json: row.get(11)?,
            forked_from: row.get(12)?,
            forks_json: row.get(13)?,
            upstream_proposal_json: row.get(14)?,
            stats_json: row.get(15)?,
            created_at: row.get(16)?,
            updated_at: row.get(17)?,
        })
    })
    .optional()
    .map_err(|e| StorageError::Internal(format!("Failed to get path extension: {}", e)))
}

/// List path extensions with filtering
pub fn list_path_extensions(conn: &Connection, query: &PathExtensionQuery) -> Result<Vec<PathExtensionRow>, StorageError> {
    let mut sql = String::from(
        "SELECT id, base_path_id, base_path_version, extended_by, title, description,
         insertions_json, annotations_json, reorderings_json, exclusions_json,
         visibility, shared_with_json, forked_from, forks_json, upstream_proposal_json,
         stats_json, created_at, updated_at
         FROM path_extensions WHERE 1=1"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(base_path_id) = &query.base_path_id {
        sql.push_str(" AND base_path_id = ?");
        params_vec.push(Box::new(base_path_id.clone()));
    }

    if let Some(extended_by) = &query.extended_by {
        sql.push_str(" AND extended_by = ?");
        params_vec.push(Box::new(extended_by.clone()));
    }

    if let Some(visibility) = &query.visibility {
        sql.push_str(" AND visibility = ?");
        params_vec.push(Box::new(visibility.clone()));
    }

    if let Some(forked_from) = &query.forked_from {
        sql.push_str(" AND forked_from = ?");
        params_vec.push(Box::new(forked_from.clone()));
    }

    sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");
    params_vec.push(Box::new(query.limit));
    params_vec.push(Box::new(query.offset));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| StorageError::Internal(format!("Failed to prepare statement: {}", e)))?;

    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(PathExtensionRow {
            id: row.get(0)?,
            base_path_id: row.get(1)?,
            base_path_version: row.get(2)?,
            extended_by: row.get(3)?,
            title: row.get(4)?,
            description: row.get(5)?,
            insertions_json: row.get(6)?,
            annotations_json: row.get(7)?,
            reorderings_json: row.get(8)?,
            exclusions_json: row.get(9)?,
            visibility: row.get(10)?,
            shared_with_json: row.get(11)?,
            forked_from: row.get(12)?,
            forks_json: row.get(13)?,
            upstream_proposal_json: row.get(14)?,
            stats_json: row.get(15)?,
            created_at: row.get(16)?,
            updated_at: row.get(17)?,
        })
    }).map_err(|e| StorageError::Internal(format!("Failed to query path extensions: {}", e)))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| StorageError::Internal(format!("Failed to read row: {}", e)))?);
    }

    Ok(results)
}

/// Create a new path extension
pub fn create_path_extension(conn: &mut Connection, input: CreatePathExtensionInput) -> Result<PathExtensionRow, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let sql = "INSERT INTO path_extensions (
               id, base_path_id, base_path_version, extended_by, title, description,
               insertions_json, annotations_json, reorderings_json, exclusions_json,
               visibility, shared_with_json, forked_from, forks_json, upstream_proposal_json, stats_json
               ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

    conn.execute(sql, params![
        id,
        input.base_path_id,
        input.base_path_version,
        input.extended_by,
        input.title,
        input.description,
        input.insertions_json,
        input.annotations_json,
        input.reorderings_json,
        input.exclusions_json,
        input.visibility,
        input.shared_with_json,
        input.forked_from,
        input.forks_json,
        input.upstream_proposal_json,
        input.stats_json,
    ]).map_err(|e| StorageError::Internal(format!("Failed to create path extension: {}", e)))?;

    get_path_extension(conn, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created path extension".to_string()))
}

/// Update a path extension
pub fn update_path_extension(conn: &mut Connection, id: &str, input: CreatePathExtensionInput) -> Result<PathExtensionRow, StorageError> {
    let sql = "UPDATE path_extensions SET
               base_path_id = ?, base_path_version = ?, title = ?, description = ?,
               insertions_json = ?, annotations_json = ?, reorderings_json = ?,
               exclusions_json = ?, visibility = ?, shared_with_json = ?,
               forked_from = ?, forks_json = ?, upstream_proposal_json = ?, stats_json = ?,
               updated_at = datetime('now')
               WHERE id = ?";

    let rows = conn.execute(sql, params![
        input.base_path_id,
        input.base_path_version,
        input.title,
        input.description,
        input.insertions_json,
        input.annotations_json,
        input.reorderings_json,
        input.exclusions_json,
        input.visibility,
        input.shared_with_json,
        input.forked_from,
        input.forks_json,
        input.upstream_proposal_json,
        input.stats_json,
        id,
    ]).map_err(|e| StorageError::Internal(format!("Failed to update path extension: {}", e)))?;

    if rows == 0 {
        return Err(StorageError::NotFound(format!("Path extension not found: {}", id)));
    }

    get_path_extension(conn, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated path extension".to_string()))
}

/// Delete a path extension by ID
pub fn delete_path_extension(conn: &mut Connection, id: &str) -> Result<bool, StorageError> {
    let rows = conn.execute("DELETE FROM path_extensions WHERE id = ?", params![id])
        .map_err(|e| StorageError::Internal(format!("Failed to delete path extension: {}", e)))?;

    Ok(rows > 0)
}
