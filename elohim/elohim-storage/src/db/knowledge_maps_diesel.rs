//! Knowledge Maps CRUD operations using Diesel with app scoping
//!
//! Knowledge maps represent domain, self, person, and collective knowledge
//! views with mastery tracking and affinity scoring.

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::knowledge_maps;
use super::models::{current_timestamp, KnowledgeMap, NewKnowledgeMap};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating a knowledge map
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeMapInput {
    #[serde(default)]
    pub id: Option<String>,
    pub map_type: String,
    pub owner_id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_name: String,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub shared_with_json: Option<String>,
    pub nodes_json: String,
    #[serde(default)]
    pub path_ids_json: Option<String>,
    #[serde(default)]
    pub overall_affinity: f32,
    #[serde(default)]
    pub content_graph_id: Option<String>,
    #[serde(default)]
    pub mastery_levels_json: Option<String>,
    #[serde(default)]
    pub goals_json: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

fn default_visibility() -> String {
    "private".to_string()
}

/// Query parameters for listing knowledge maps
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
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

fn default_limit() -> i64 {
    100
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get knowledge map by ID - scoped by app
pub fn get_knowledge_map(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<KnowledgeMap>, StorageError> {
    knowledge_maps::table
        .filter(knowledge_maps::h_app_id.eq(&ctx.h_app_id))
        .filter(knowledge_maps::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List knowledge maps with filtering - scoped by app
pub fn list_knowledge_maps(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &KnowledgeMapQuery,
) -> Result<Vec<KnowledgeMap>, StorageError> {
    let mut base_query = knowledge_maps::table
        .filter(knowledge_maps::h_app_id.eq(&ctx.h_app_id))
        .into_boxed();

    if let Some(ref owner_id) = query.owner_id {
        base_query = base_query.filter(knowledge_maps::owner_id.eq(owner_id));
    }

    if let Some(ref map_type) = query.map_type {
        base_query = base_query.filter(knowledge_maps::map_type.eq(map_type));
    }

    if let Some(ref subject_id) = query.subject_id {
        base_query = base_query.filter(knowledge_maps::subject_id.eq(subject_id));
    }

    if let Some(ref visibility) = query.visibility {
        base_query = base_query.filter(knowledge_maps::visibility.eq(visibility));
    }

    base_query
        .order(knowledge_maps::updated_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create a knowledge map - scoped by app
pub fn create_knowledge_map(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateKnowledgeMapInput,
) -> Result<KnowledgeMap, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let new_map = NewKnowledgeMap {
        id: &id,
        h_app_id: &ctx.h_app_id,
        map_type: &input.map_type,
        owner_id: &input.owner_id,
        title: &input.title,
        description: input.description.as_deref(),
        subject_type: &input.subject_type,
        subject_id: &input.subject_id,
        subject_name: &input.subject_name,
        visibility: &input.visibility,
        shared_with_json: input.shared_with_json.as_deref(),
        nodes_json: &input.nodes_json,
        path_ids_json: input.path_ids_json.as_deref(),
        overall_affinity: input.overall_affinity,
        content_graph_id: input.content_graph_id.as_deref(),
        mastery_levels_json: input.mastery_levels_json.as_deref(),
        goals_json: input.goals_json.as_deref(),
        metadata_json: input.metadata_json.as_deref(),
    };

    diesel::insert_into(knowledge_maps::table)
        .values(&new_map)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_knowledge_map(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created knowledge map".into()))
}

/// Update a knowledge map - scoped by app
pub fn update_knowledge_map(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    input: CreateKnowledgeMapInput,
) -> Result<KnowledgeMap, StorageError> {
    let now = current_timestamp();

    diesel::update(
        knowledge_maps::table
            .filter(knowledge_maps::h_app_id.eq(&ctx.h_app_id))
            .filter(knowledge_maps::id.eq(id)),
    )
    .set((
        knowledge_maps::map_type.eq(&input.map_type),
        knowledge_maps::owner_id.eq(&input.owner_id),
        knowledge_maps::title.eq(&input.title),
        knowledge_maps::description.eq(input.description.as_deref()),
        knowledge_maps::subject_type.eq(&input.subject_type),
        knowledge_maps::subject_id.eq(&input.subject_id),
        knowledge_maps::subject_name.eq(&input.subject_name),
        knowledge_maps::visibility.eq(&input.visibility),
        knowledge_maps::shared_with_json.eq(input.shared_with_json.as_deref()),
        knowledge_maps::nodes_json.eq(&input.nodes_json),
        knowledge_maps::path_ids_json.eq(input.path_ids_json.as_deref()),
        knowledge_maps::overall_affinity.eq(input.overall_affinity),
        knowledge_maps::content_graph_id.eq(input.content_graph_id.as_deref()),
        knowledge_maps::mastery_levels_json.eq(input.mastery_levels_json.as_deref()),
        knowledge_maps::goals_json.eq(input.goals_json.as_deref()),
        knowledge_maps::metadata_json.eq(input.metadata_json.as_deref()),
        knowledge_maps::updated_at.eq(&now),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_knowledge_map(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Knowledge map not found after update".into()))
}

/// Delete a knowledge map by ID - scoped by app
pub fn delete_knowledge_map(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<bool, StorageError> {
    let deleted = diesel::delete(
        knowledge_maps::table
            .filter(knowledge_maps::h_app_id.eq(&ctx.h_app_id))
            .filter(knowledge_maps::id.eq(id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted > 0)
}

// ============================================================================
// Stats
// ============================================================================

/// Get knowledge map count for an app
pub fn knowledge_map_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    knowledge_maps::table
        .filter(knowledge_maps::h_app_id.eq(&ctx.h_app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sqlite::SqliteConnection;
    use diesel::Connection;

    fn setup_test_db() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");

        diesel::sql_query(
            r#"
            CREATE TABLE knowledge_maps (
                id TEXT PRIMARY KEY NOT NULL,
                h_app_id TEXT NOT NULL DEFAULT 'lamad',
                map_type TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                subject_type TEXT NOT NULL,
                subject_id TEXT NOT NULL,
                subject_name TEXT NOT NULL,
                visibility TEXT NOT NULL DEFAULT 'private',
                shared_with_json TEXT,
                nodes_json TEXT NOT NULL,
                path_ids_json TEXT,
                overall_affinity REAL NOT NULL DEFAULT 0.0,
                content_graph_id TEXT,
                mastery_levels_json TEXT,
                goals_json TEXT,
                metadata_json TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&mut conn)
        .expect("Failed to create knowledge_maps table");

        conn
    }

    #[test]
    fn test_create_knowledge_map() {
        let mut conn = setup_test_db();
        let ctx = AppContext::default_lamad();

        let input = CreateKnowledgeMapInput {
            id: None,
            map_type: "domain".to_string(),
            owner_id: "user-1".to_string(),
            title: "Mathematics".to_string(),
            description: Some("Domain knowledge map for math".to_string()),
            subject_type: "domain".to_string(),
            subject_id: "math".to_string(),
            subject_name: "Mathematics".to_string(),
            visibility: "private".to_string(),
            shared_with_json: None,
            nodes_json: "[]".to_string(),
            path_ids_json: None,
            overall_affinity: 0.5,
            content_graph_id: None,
            mastery_levels_json: None,
            goals_json: None,
            metadata_json: None,
        };

        let map = create_knowledge_map(&mut conn, &ctx, input).unwrap();
        assert_eq!(map.map_type, "domain");
        assert_eq!(map.owner_id, "user-1");
        assert_eq!(map.title, "Mathematics");
    }

    #[test]
    fn test_app_isolation() {
        let mut conn = setup_test_db();
        let lamad_ctx = AppContext::default_lamad();
        let elohim_ctx = AppContext::default_elohim();

        let input = CreateKnowledgeMapInput {
            id: None,
            map_type: "self".to_string(),
            owner_id: "user-1".to_string(),
            title: "My Map".to_string(),
            description: None,
            subject_type: "self".to_string(),
            subject_id: "user-1".to_string(),
            subject_name: "User 1".to_string(),
            visibility: "private".to_string(),
            shared_with_json: None,
            nodes_json: "[]".to_string(),
            path_ids_json: None,
            overall_affinity: 0.0,
            content_graph_id: None,
            mastery_levels_json: None,
            goals_json: None,
            metadata_json: None,
        };

        create_knowledge_map(&mut conn, &lamad_ctx, input).unwrap();

        let lamad_count = knowledge_map_count(&mut conn, &lamad_ctx).unwrap();
        assert_eq!(lamad_count, 1);

        let elohim_count = knowledge_map_count(&mut conn, &elohim_ctx).unwrap();
        assert_eq!(elohim_count, 0);
    }
}
