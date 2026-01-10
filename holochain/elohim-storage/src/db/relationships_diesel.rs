//! Relationships CRUD operations using Diesel with app scoping
//!
//! Content graph edge storage with bidirectionality, provenance tracking,
//! and governance layer support.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::relationships;
use super::models::{Relationship, NewRelationship, current_timestamp};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating a relationship
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRelationshipInput {
    #[serde(default)]
    pub id: Option<String>,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default = "default_inference_source")]
    pub inference_source: String,
    #[serde(default)]
    pub is_bidirectional: bool,
    #[serde(default)]
    pub provenance_chain_json: Option<String>,
    #[serde(default)]
    pub governance_layer: Option<String>,
    #[serde(default = "default_reach")]
    pub reach: String,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

fn default_confidence() -> f32 { 1.0 }
fn default_inference_source() -> String { "explicit".to_string() }
fn default_reach() -> String { "commons".to_string() }

/// Query parameters for listing relationships
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RelationshipQuery {
    pub content_id: Option<String>,
    /// Direction filter: "outgoing", "incoming", or "both" (default)
    pub direction: Option<String>,
    pub relationship_type: Option<String>,
    pub inference_source: Option<String>,
    pub governance_layer: Option<String>,
    pub min_confidence: Option<f32>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 100 }

/// Result of bulk operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkRelationshipResult {
    pub created: u64,
    pub updated: u64,
    pub errors: Vec<String>,
}

/// Bidirectional relationship creation result
#[derive(Debug, Clone, Serialize)]
pub struct BidirectionalResult {
    pub forward: Relationship,
    pub inverse: Option<Relationship>,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get relationship by ID - scoped by app
pub fn get_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<Relationship>, StorageError> {
    relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .filter(relationships::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List relationships with filtering - scoped by app
pub fn list_relationships(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &RelationshipQuery,
) -> Result<Vec<Relationship>, StorageError> {
    let mut base_query = relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .into_boxed();

    // Filter by content involvement
    if let Some(ref content_id) = query.content_id {
        match query.direction.as_deref() {
            Some("outgoing") => {
                base_query = base_query.filter(relationships::source_id.eq(content_id));
            }
            Some("incoming") => {
                base_query = base_query.filter(relationships::target_id.eq(content_id));
            }
            _ => {
                // Both directions (default)
                base_query = base_query.filter(
                    relationships::source_id.eq(content_id)
                        .or(relationships::target_id.eq(content_id))
                );
            }
        }
    }

    // Apply optional filters
    if let Some(ref rel_type) = query.relationship_type {
        base_query = base_query.filter(relationships::relationship_type.eq(rel_type));
    }

    if let Some(ref source) = query.inference_source {
        base_query = base_query.filter(relationships::inference_source.eq(source));
    }

    if let Some(ref layer) = query.governance_layer {
        base_query = base_query.filter(relationships::governance_layer.eq(layer));
    }

    if let Some(min_conf) = query.min_confidence {
        base_query = base_query.filter(relationships::confidence.ge(min_conf));
    }

    base_query
        .order(relationships::created_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get all relationships for a content node (both directions)
pub fn get_relationships_for_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<Relationship>, StorageError> {
    list_relationships(conn, ctx, &RelationshipQuery {
        content_id: Some(content_id.to_string()),
        ..Default::default()
    })
}

/// Get outgoing relationships from a content node
pub fn get_outgoing_relationships(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
    relationship_types: Option<&[String]>,
) -> Result<Vec<Relationship>, StorageError> {
    let mut base_query = relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .filter(relationships::source_id.eq(content_id))
        .into_boxed();

    if let Some(types) = relationship_types {
        if !types.is_empty() {
            base_query = base_query.filter(relationships::relationship_type.eq_any(types));
        }
    }

    base_query
        .order(relationships::confidence.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get incoming relationships to a content node
pub fn get_incoming_relationships(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
    relationship_types: Option<&[String]>,
) -> Result<Vec<Relationship>, StorageError> {
    let mut base_query = relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .filter(relationships::target_id.eq(content_id))
        .into_boxed();

    if let Some(types) = relationship_types {
        if !types.is_empty() {
            base_query = base_query.filter(relationships::relationship_type.eq_any(types));
        }
    }

    base_query
        .order(relationships::confidence.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create a single relationship - scoped by app
pub fn create_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateRelationshipInput,
) -> Result<Relationship, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let new_rel = NewRelationship {
        id: &id,
        app_id: &ctx.app_id,
        source_id: &input.source_id,
        target_id: &input.target_id,
        relationship_type: &input.relationship_type,
        confidence: input.confidence,
        inference_source: &input.inference_source,
        is_bidirectional: if input.is_bidirectional { 1 } else { 0 },
        inverse_relationship_id: None,
        provenance_chain_json: input.provenance_chain_json.as_deref(),
        governance_layer: input.governance_layer.as_deref(),
        reach: &input.reach,
        metadata_json: input.metadata_json.as_deref(),
    };

    // Use INSERT OR REPLACE for idempotent creation
    diesel::insert_into(relationships::table)
        .values(&new_rel)
        .on_conflict((relationships::app_id, relationships::source_id, relationships::target_id, relationships::relationship_type))
        .do_update()
        .set((
            relationships::confidence.eq(input.confidence),
            relationships::inference_source.eq(&input.inference_source),
            relationships::is_bidirectional.eq(if input.is_bidirectional { 1 } else { 0 }),
            relationships::metadata_json.eq(input.metadata_json.as_deref()),
            relationships::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_relationship(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created relationship".into()))
}

/// Create a bidirectional relationship pair atomically
///
/// Creates both the forward and inverse relationships, linking them together.
/// Uses the relationship type's inverse mapping (e.g., CONTAINS → CONTAINED_BY).
pub fn create_bidirectional(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateRelationshipInput,
) -> Result<BidirectionalResult, StorageError> {
    use super::models::relationship_types;

    conn.transaction(|conn| {
        let forward_id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let inverse_id = Uuid::new_v4().to_string();

        // Get inverse relationship type
        let inverse_type = relationship_types::inverse(&input.relationship_type)
            .unwrap_or(&input.relationship_type);

        // Create forward relationship
        let forward_rel = NewRelationship {
            id: &forward_id,
            app_id: &ctx.app_id,
            source_id: &input.source_id,
            target_id: &input.target_id,
            relationship_type: &input.relationship_type,
            confidence: input.confidence,
            inference_source: &input.inference_source,
            is_bidirectional: 1,
            inverse_relationship_id: Some(&inverse_id),
            provenance_chain_json: input.provenance_chain_json.as_deref(),
            governance_layer: input.governance_layer.as_deref(),
            reach: &input.reach,
            metadata_json: input.metadata_json.as_deref(),
        };

        diesel::insert_into(relationships::table)
            .values(&forward_rel)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Forward insert failed: {}", e)))?;

        // Create inverse relationship
        let inverse_rel = NewRelationship {
            id: &inverse_id,
            app_id: &ctx.app_id,
            source_id: &input.target_id,  // Swapped
            target_id: &input.source_id,  // Swapped
            relationship_type: inverse_type,
            confidence: input.confidence,
            inference_source: &input.inference_source,
            is_bidirectional: 1,
            inverse_relationship_id: Some(&forward_id),
            provenance_chain_json: input.provenance_chain_json.as_deref(),
            governance_layer: input.governance_layer.as_deref(),
            reach: &input.reach,
            metadata_json: input.metadata_json.as_deref(),
        };

        diesel::insert_into(relationships::table)
            .values(&inverse_rel)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Inverse insert failed: {}", e)))?;

        // Fetch both
        let forward = get_relationship(conn, ctx, &forward_id)?
            .ok_or_else(|| StorageError::Internal("Failed to fetch forward".into()))?;
        let inverse = get_relationship(conn, ctx, &inverse_id)?;

        Ok(BidirectionalResult { forward, inverse })
    })
}

/// Bulk create relationships (for seeding/import) - scoped by app
pub fn bulk_create_relationships(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    inputs: Vec<CreateRelationshipInput>,
) -> Result<BulkRelationshipResult, StorageError> {
    let mut created = 0u64;
    let mut updated = 0u64;
    let mut errors = vec![];

    conn.transaction(|conn| {
        for input in inputs {
            let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

            // Check if exists
            let exists: bool = relationships::table
                .filter(relationships::app_id.eq(&ctx.app_id))
                .filter(relationships::source_id.eq(&input.source_id))
                .filter(relationships::target_id.eq(&input.target_id))
                .filter(relationships::relationship_type.eq(&input.relationship_type))
                .select(diesel::dsl::count_star())
                .first::<i64>(conn)
                .map(|c| c > 0)
                .unwrap_or(false);

            let new_rel = NewRelationship {
                id: &id,
                app_id: &ctx.app_id,
                source_id: &input.source_id,
                target_id: &input.target_id,
                relationship_type: &input.relationship_type,
                confidence: input.confidence,
                inference_source: &input.inference_source,
                is_bidirectional: if input.is_bidirectional { 1 } else { 0 },
                inverse_relationship_id: None,
                provenance_chain_json: input.provenance_chain_json.as_deref(),
                governance_layer: input.governance_layer.as_deref(),
                reach: &input.reach,
                metadata_json: input.metadata_json.as_deref(),
            };

            match diesel::insert_into(relationships::table)
                .values(&new_rel)
                .on_conflict((relationships::app_id, relationships::source_id, relationships::target_id, relationships::relationship_type))
                .do_update()
                .set((
                    relationships::confidence.eq(input.confidence),
                    relationships::inference_source.eq(&input.inference_source),
                    relationships::is_bidirectional.eq(if input.is_bidirectional { 1 } else { 0 }),
                    relationships::metadata_json.eq(input.metadata_json.as_deref()),
                ))
                .execute(conn)
            {
                Ok(_) => {
                    if exists {
                        updated += 1;
                    } else {
                        created += 1;
                    }
                }
                Err(e) => {
                    errors.push(format!("{}→{}: {}", input.source_id, input.target_id, e));
                }
            }
        }

        Ok(BulkRelationshipResult { created, updated, errors })
    })
}

/// Delete a relationship by ID - scoped by app
pub fn delete_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<bool, StorageError> {
    // First check if it has an inverse to delete too
    let rel = get_relationship(conn, ctx, id)?;

    let deleted = diesel::delete(
        relationships::table
            .filter(relationships::app_id.eq(&ctx.app_id))
            .filter(relationships::id.eq(id))
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    // Also delete the inverse if it exists
    if let Some(rel) = rel {
        if let Some(inverse_id) = rel.inverse_relationship_id {
            let _ = diesel::delete(
                relationships::table
                    .filter(relationships::app_id.eq(&ctx.app_id))
                    .filter(relationships::id.eq(&inverse_id))
            )
            .execute(conn);
        }
    }

    Ok(deleted > 0)
}

/// Delete all relationships where content is source or target - scoped by app
pub fn delete_relationships_for_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<usize, StorageError> {
    let deleted = diesel::delete(
        relationships::table
            .filter(relationships::app_id.eq(&ctx.app_id))
            .filter(
                relationships::source_id.eq(content_id)
                    .or(relationships::target_id.eq(content_id))
            )
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted)
}

// ============================================================================
// Stats & Analysis
// ============================================================================

/// Get relationship count for an app
pub fn relationship_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get relationship statistics by type
pub fn relationship_stats_by_type(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<(String, i64)>, StorageError> {
    relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .group_by(relationships::relationship_type)
        .select((relationships::relationship_type, diesel::dsl::count_star()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Stats query failed: {}", e)))
}

/// Get relationship statistics by inference source
pub fn relationship_stats_by_source(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<(String, i64)>, StorageError> {
    relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .group_by(relationships::inference_source)
        .select((relationships::inference_source, diesel::dsl::count_star()))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Stats query failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sqlite::SqliteConnection;
    use diesel::Connection;

    fn setup_test_db() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:")
            .expect("Failed to create in-memory database");

        diesel::sql_query(
            r#"
            CREATE TABLE relationships (
                id TEXT PRIMARY KEY NOT NULL,
                app_id TEXT NOT NULL DEFAULT 'lamad',
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 1.0,
                inference_source TEXT NOT NULL DEFAULT 'explicit',
                is_bidirectional INTEGER NOT NULL DEFAULT 0,
                inverse_relationship_id TEXT,
                provenance_chain_json TEXT,
                governance_layer TEXT,
                reach TEXT NOT NULL DEFAULT 'commons',
                metadata_json TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&mut conn)
        .expect("Failed to create relationships table");

        diesel::sql_query(
            "CREATE UNIQUE INDEX idx_rel_unique ON relationships(app_id, source_id, target_id, relationship_type)"
        )
        .execute(&mut conn)
        .expect("Failed to create unique index");

        conn
    }

    #[test]
    fn test_create_relationship() {
        let mut conn = setup_test_db();
        let ctx = AppContext::default_lamad();

        let input = CreateRelationshipInput {
            id: None,
            source_id: "manifesto".to_string(),
            target_id: "governance-epic".to_string(),
            relationship_type: "CONTAINS".to_string(),
            confidence: 1.0,
            inference_source: "explicit".to_string(),
            is_bidirectional: false,
            provenance_chain_json: None,
            governance_layer: None,
            reach: "commons".to_string(),
            metadata_json: None,
        };

        let rel = create_relationship(&mut conn, &ctx, input).unwrap();
        assert_eq!(rel.source_id, "manifesto");
        assert_eq!(rel.target_id, "governance-epic");
        assert_eq!(rel.relationship_type, "CONTAINS");
    }

    #[test]
    fn test_app_isolation() {
        let mut conn = setup_test_db();
        let lamad_ctx = AppContext::default_lamad();
        let elohim_ctx = AppContext::default_elohim();

        // Create in lamad
        let input = CreateRelationshipInput {
            id: None,
            source_id: "content-a".to_string(),
            target_id: "content-b".to_string(),
            relationship_type: "RELATES_TO".to_string(),
            confidence: 0.9,
            inference_source: "explicit".to_string(),
            is_bidirectional: false,
            provenance_chain_json: None,
            governance_layer: None,
            reach: "commons".to_string(),
            metadata_json: None,
        };
        create_relationship(&mut conn, &lamad_ctx, input).unwrap();

        // Should see it in lamad
        let lamad_count = relationship_count(&mut conn, &lamad_ctx).unwrap();
        assert_eq!(lamad_count, 1);

        // Should NOT see it in elohim
        let elohim_count = relationship_count(&mut conn, &elohim_ctx).unwrap();
        assert_eq!(elohim_count, 0);
    }
}
