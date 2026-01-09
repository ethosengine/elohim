//! Relationships CRUD operations
//!
//! Content graph edge storage for content relationships.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::StorageError;

// =============================================================================
// Types
// =============================================================================

/// Relationship row from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipRow {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub inference_source: String,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

/// Input for creating a relationship
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRelationshipInput {
    pub id: Option<String>,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default = "default_inference_source")]
    pub inference_source: String,
    pub metadata_json: Option<String>,
}

fn default_confidence() -> f64 { 1.0 }
fn default_inference_source() -> String { "explicit".to_string() }

/// Query parameters for listing relationships
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RelationshipQuery {
    pub content_id: Option<String>,
    pub direction: Option<String>,  // outgoing, incoming, both
    pub relationship_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 100 }

/// Content graph node for tree traversal
#[derive(Debug, Clone, Serialize)]
pub struct ContentGraphNode {
    pub content_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub children: Vec<ContentGraphNode>,
}

/// Content graph output
#[derive(Debug, Clone, Serialize)]
pub struct ContentGraph {
    pub root_id: String,
    pub related: Vec<ContentGraphNode>,
    pub total_nodes: usize,
}

// =============================================================================
// CRUD Operations
// =============================================================================

/// Get a relationship by ID
pub fn get_relationship(conn: &Connection, id: &str) -> Result<Option<RelationshipRow>, StorageError> {
    let sql = "SELECT id, source_id, target_id, relationship_type, confidence,
               inference_source, metadata_json, created_at
               FROM relationships WHERE id = ?";

    conn.query_row(sql, params![id], |row| {
        Ok(RelationshipRow {
            id: row.get(0)?,
            source_id: row.get(1)?,
            target_id: row.get(2)?,
            relationship_type: row.get(3)?,
            confidence: row.get(4)?,
            inference_source: row.get(5)?,
            metadata_json: row.get(6)?,
            created_at: row.get(7)?,
        })
    })
    .optional()
    .map_err(|e| StorageError::Internal(format!("Failed to get relationship: {}", e)))
}

/// List relationships with filtering
pub fn list_relationships(conn: &Connection, query: &RelationshipQuery) -> Result<Vec<RelationshipRow>, StorageError> {
    let mut sql = String::from(
        "SELECT id, source_id, target_id, relationship_type, confidence,
         inference_source, metadata_json, created_at FROM relationships WHERE 1=1"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(content_id) = &query.content_id {
        match query.direction.as_deref() {
            Some("outgoing") => {
                sql.push_str(" AND source_id = ?");
                params_vec.push(Box::new(content_id.clone()));
            }
            Some("incoming") => {
                sql.push_str(" AND target_id = ?");
                params_vec.push(Box::new(content_id.clone()));
            }
            _ => {
                // Both directions (default)
                sql.push_str(" AND (source_id = ? OR target_id = ?)");
                params_vec.push(Box::new(content_id.clone()));
                params_vec.push(Box::new(content_id.clone()));
            }
        }
    }

    if let Some(rel_type) = &query.relationship_type {
        sql.push_str(" AND relationship_type = ?");
        params_vec.push(Box::new(rel_type.clone()));
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");
    params_vec.push(Box::new(query.limit));
    params_vec.push(Box::new(query.offset));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| StorageError::Internal(format!("Failed to prepare statement: {}", e)))?;

    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(RelationshipRow {
            id: row.get(0)?,
            source_id: row.get(1)?,
            target_id: row.get(2)?,
            relationship_type: row.get(3)?,
            confidence: row.get(4)?,
            inference_source: row.get(5)?,
            metadata_json: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).map_err(|e| StorageError::Internal(format!("Failed to query relationships: {}", e)))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| StorageError::Internal(format!("Failed to read row: {}", e)))?);
    }

    Ok(results)
}

/// Create a new relationship
pub fn create_relationship(conn: &mut Connection, input: CreateRelationshipInput) -> Result<RelationshipRow, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let sql = "INSERT INTO relationships (id, source_id, target_id, relationship_type, confidence, inference_source, metadata_json)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(source_id, target_id, relationship_type) DO UPDATE SET
               confidence = excluded.confidence,
               inference_source = excluded.inference_source,
               metadata_json = excluded.metadata_json";

    conn.execute(sql, params![
        id,
        input.source_id,
        input.target_id,
        input.relationship_type,
        input.confidence,
        input.inference_source,
        input.metadata_json,
    ]).map_err(|e| StorageError::Internal(format!("Failed to create relationship: {}", e)))?;

    get_relationship(conn, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created relationship".to_string()))
}

/// Delete a relationship by ID
pub fn delete_relationship(conn: &mut Connection, id: &str) -> Result<bool, StorageError> {
    let rows = conn.execute("DELETE FROM relationships WHERE id = ?", params![id])
        .map_err(|e| StorageError::Internal(format!("Failed to delete relationship: {}", e)))?;

    Ok(rows > 0)
}

/// Get content graph starting from a root node
/// Returns immediate relationships (depth=1) grouped by relationship type
pub fn get_content_graph(
    conn: &Connection,
    content_id: &str,
    relationship_types: Option<&[String]>,
) -> Result<ContentGraph, StorageError> {
    let mut sql = String::from(
        "SELECT id, source_id, target_id, relationship_type, confidence
         FROM relationships WHERE source_id = ?"
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(content_id.to_string())];

    if let Some(types) = relationship_types {
        if !types.is_empty() {
            let placeholders: Vec<&str> = types.iter().map(|_| "?").collect();
            sql.push_str(&format!(" AND relationship_type IN ({})", placeholders.join(",")));
            for t in types {
                params_vec.push(Box::new(t.clone()));
            }
        }
    }

    sql.push_str(" ORDER BY relationship_type, confidence DESC");

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| StorageError::Internal(format!("Failed to prepare statement: {}", e)))?;

    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok((
            row.get::<_, String>(2)?,  // target_id
            row.get::<_, String>(3)?,  // relationship_type
            row.get::<_, f64>(4)?,     // confidence
        ))
    }).map_err(|e| StorageError::Internal(format!("Failed to query graph: {}", e)))?;

    let mut related = Vec::new();
    for row in rows {
        let (target_id, rel_type, confidence) = row
            .map_err(|e| StorageError::Internal(format!("Failed to read row: {}", e)))?;

        related.push(ContentGraphNode {
            content_id: target_id,
            relationship_type: rel_type,
            confidence,
            children: vec![],  // Depth 1 only for now
        });
    }

    let total_nodes = related.len();

    Ok(ContentGraph {
        root_id: content_id.to_string(),
        related,
        total_nodes,
    })
}

/// Bulk create relationships
pub fn bulk_create_relationships(
    conn: &mut Connection,
    inputs: Vec<CreateRelationshipInput>,
) -> Result<BulkRelationshipResult, StorageError> {
    let tx = conn.transaction()
        .map_err(|e| StorageError::Internal(format!("Failed to start transaction: {}", e)))?;

    let mut created = 0;
    let mut errors = Vec::new();

    for input in inputs {
        let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

        let result = tx.execute(
            "INSERT INTO relationships (id, source_id, target_id, relationship_type, confidence, inference_source, metadata_json)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source_id, target_id, relationship_type) DO UPDATE SET
             confidence = excluded.confidence,
             inference_source = excluded.inference_source,
             metadata_json = excluded.metadata_json",
            params![
                id,
                input.source_id,
                input.target_id,
                input.relationship_type,
                input.confidence,
                input.inference_source,
                input.metadata_json,
            ],
        );

        match result {
            Ok(_) => created += 1,
            Err(e) => errors.push(format!("{}→{}: {}", input.source_id, input.target_id, e)),
        }
    }

    tx.commit()
        .map_err(|e| StorageError::Internal(format!("Failed to commit transaction: {}", e)))?;

    Ok(BulkRelationshipResult { created, errors })
}

/// Result of bulk relationship creation
#[derive(Debug, Clone, Serialize)]
pub struct BulkRelationshipResult {
    pub created: usize,
    pub errors: Vec<String>,
}

/// Delete all relationships where content is source or target
pub fn delete_relationships_for_content(conn: &mut Connection, content_id: &str) -> Result<usize, StorageError> {
    let rows = conn.execute(
        "DELETE FROM relationships WHERE source_id = ? OR target_id = ?",
        params![content_id, content_id],
    ).map_err(|e| StorageError::Internal(format!("Failed to delete relationships: {}", e)))?;

    Ok(rows)
}
