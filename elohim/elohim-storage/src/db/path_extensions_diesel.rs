//! Path Extensions CRUD operations using Diesel with app scoping
//!
//! Path extensions represent user customizations and forks of learning paths,
//! including insertions, annotations, reorderings, and exclusions.

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::path_extensions;
use super::models::{current_timestamp, NewPathExtension, PathExtension};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating a path extension
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePathExtensionInput {
    #[serde(default)]
    pub id: Option<String>,
    pub base_path_id: String,
    pub base_path_version: String,
    pub extended_by: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub insertions_json: Option<String>,
    #[serde(default)]
    pub annotations_json: Option<String>,
    #[serde(default)]
    pub reorderings_json: Option<String>,
    #[serde(default)]
    pub exclusions_json: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub shared_with_json: Option<String>,
    #[serde(default)]
    pub forked_from: Option<String>,
    #[serde(default)]
    pub forks_json: Option<String>,
    #[serde(default)]
    pub upstream_proposal_json: Option<String>,
    #[serde(default)]
    pub stats_json: Option<String>,
}

fn default_visibility() -> String {
    "private".to_string()
}

/// Query parameters for listing path extensions
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
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

fn default_limit() -> i64 {
    100
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get path extension by ID - scoped by app
pub fn get_path_extension(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<PathExtension>, StorageError> {
    path_extensions::table
        .filter(path_extensions::app_id.eq(&ctx.app_id))
        .filter(path_extensions::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List path extensions with filtering - scoped by app
pub fn list_path_extensions(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &PathExtensionQuery,
) -> Result<Vec<PathExtension>, StorageError> {
    let mut base_query = path_extensions::table
        .filter(path_extensions::app_id.eq(&ctx.app_id))
        .into_boxed();

    if let Some(ref base_path_id) = query.base_path_id {
        base_query = base_query.filter(path_extensions::base_path_id.eq(base_path_id));
    }

    if let Some(ref extended_by) = query.extended_by {
        base_query = base_query.filter(path_extensions::extended_by.eq(extended_by));
    }

    if let Some(ref visibility) = query.visibility {
        base_query = base_query.filter(path_extensions::visibility.eq(visibility));
    }

    if let Some(ref forked_from) = query.forked_from {
        base_query = base_query.filter(path_extensions::forked_from.eq(forked_from));
    }

    base_query
        .order(path_extensions::updated_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create a path extension - scoped by app
pub fn create_path_extension(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreatePathExtensionInput,
) -> Result<PathExtension, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let new_ext = NewPathExtension {
        id: &id,
        app_id: &ctx.app_id,
        base_path_id: &input.base_path_id,
        base_path_version: &input.base_path_version,
        extended_by: &input.extended_by,
        title: &input.title,
        description: input.description.as_deref(),
        insertions_json: input.insertions_json.as_deref(),
        annotations_json: input.annotations_json.as_deref(),
        reorderings_json: input.reorderings_json.as_deref(),
        exclusions_json: input.exclusions_json.as_deref(),
        visibility: &input.visibility,
        shared_with_json: input.shared_with_json.as_deref(),
        forked_from: input.forked_from.as_deref(),
        forks_json: input.forks_json.as_deref(),
        upstream_proposal_json: input.upstream_proposal_json.as_deref(),
        stats_json: input.stats_json.as_deref(),
    };

    diesel::insert_into(path_extensions::table)
        .values(&new_ext)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_path_extension(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created path extension".into()))
}

/// Update a path extension - scoped by app
pub fn update_path_extension(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    input: CreatePathExtensionInput,
) -> Result<PathExtension, StorageError> {
    let now = current_timestamp();

    diesel::update(
        path_extensions::table
            .filter(path_extensions::app_id.eq(&ctx.app_id))
            .filter(path_extensions::id.eq(id)),
    )
    .set((
        path_extensions::base_path_id.eq(&input.base_path_id),
        path_extensions::base_path_version.eq(&input.base_path_version),
        path_extensions::extended_by.eq(&input.extended_by),
        path_extensions::title.eq(&input.title),
        path_extensions::description.eq(input.description.as_deref()),
        path_extensions::insertions_json.eq(input.insertions_json.as_deref()),
        path_extensions::annotations_json.eq(input.annotations_json.as_deref()),
        path_extensions::reorderings_json.eq(input.reorderings_json.as_deref()),
        path_extensions::exclusions_json.eq(input.exclusions_json.as_deref()),
        path_extensions::visibility.eq(&input.visibility),
        path_extensions::shared_with_json.eq(input.shared_with_json.as_deref()),
        path_extensions::forked_from.eq(input.forked_from.as_deref()),
        path_extensions::forks_json.eq(input.forks_json.as_deref()),
        path_extensions::upstream_proposal_json.eq(input.upstream_proposal_json.as_deref()),
        path_extensions::stats_json.eq(input.stats_json.as_deref()),
        path_extensions::updated_at.eq(&now),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_path_extension(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Path extension not found after update".into()))
}

/// Delete a path extension by ID - scoped by app
pub fn delete_path_extension(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<bool, StorageError> {
    let deleted = diesel::delete(
        path_extensions::table
            .filter(path_extensions::app_id.eq(&ctx.app_id))
            .filter(path_extensions::id.eq(id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted > 0)
}

// ============================================================================
// Stats
// ============================================================================

/// Get path extension count for an app
pub fn path_extension_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    path_extensions::table
        .filter(path_extensions::app_id.eq(&ctx.app_id))
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

        // Create paths table first (FK target)
        diesel::sql_query(
            r#"
            CREATE TABLE paths (
                id TEXT PRIMARY KEY NOT NULL,
                app_id TEXT NOT NULL DEFAULT 'lamad',
                title TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&mut conn)
        .expect("Failed to create paths table");

        diesel::sql_query(
            r#"
            CREATE TABLE path_extensions (
                id TEXT PRIMARY KEY NOT NULL,
                app_id TEXT NOT NULL DEFAULT 'lamad',
                base_path_id TEXT NOT NULL,
                base_path_version TEXT NOT NULL,
                extended_by TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                insertions_json TEXT,
                annotations_json TEXT,
                reorderings_json TEXT,
                exclusions_json TEXT,
                visibility TEXT NOT NULL DEFAULT 'private',
                shared_with_json TEXT,
                forked_from TEXT,
                forks_json TEXT,
                upstream_proposal_json TEXT,
                stats_json TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (base_path_id) REFERENCES paths(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&mut conn)
        .expect("Failed to create path_extensions table");

        // Insert a path to satisfy FK
        diesel::sql_query(
            "INSERT INTO paths (id, app_id, title) VALUES ('path-1', 'lamad', 'Test Path')",
        )
        .execute(&mut conn)
        .expect("Failed to insert test path");

        conn
    }

    #[test]
    fn test_create_path_extension() {
        let mut conn = setup_test_db();
        let ctx = AppContext::default_lamad();

        let input = CreatePathExtensionInput {
            id: None,
            base_path_id: "path-1".to_string(),
            base_path_version: "1.0.0".to_string(),
            extended_by: "user-1".to_string(),
            title: "My custom path".to_string(),
            description: Some("Added extra exercises".to_string()),
            insertions_json: Some("[{\"after\":\"step-3\",\"content\":\"extra-1\"}]".to_string()),
            annotations_json: None,
            reorderings_json: None,
            exclusions_json: None,
            visibility: "private".to_string(),
            shared_with_json: None,
            forked_from: None,
            forks_json: None,
            upstream_proposal_json: None,
            stats_json: None,
        };

        let ext = create_path_extension(&mut conn, &ctx, input).unwrap();
        assert_eq!(ext.base_path_id, "path-1");
        assert_eq!(ext.extended_by, "user-1");
        assert_eq!(ext.title, "My custom path");
    }

    #[test]
    fn test_app_isolation() {
        let mut conn = setup_test_db();
        let lamad_ctx = AppContext::default_lamad();
        let elohim_ctx = AppContext::default_elohim();

        let input = CreatePathExtensionInput {
            id: None,
            base_path_id: "path-1".to_string(),
            base_path_version: "1.0.0".to_string(),
            extended_by: "user-1".to_string(),
            title: "Extension".to_string(),
            description: None,
            insertions_json: None,
            annotations_json: None,
            reorderings_json: None,
            exclusions_json: None,
            visibility: "private".to_string(),
            shared_with_json: None,
            forked_from: None,
            forks_json: None,
            upstream_proposal_json: None,
            stats_json: None,
        };

        create_path_extension(&mut conn, &lamad_ctx, input).unwrap();

        let lamad_count = path_extension_count(&mut conn, &lamad_ctx).unwrap();
        assert_eq!(lamad_count, 1);

        let elohim_count = path_extension_count(&mut conn, &elohim_ctx).unwrap();
        assert_eq!(elohim_count, 0);
    }
}
