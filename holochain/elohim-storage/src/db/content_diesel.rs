//! Content CRUD operations using Diesel with app scoping
//!
//! All operations require an AppContext for multi-tenant isolation.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use serde::{Deserialize, Serialize};

use super::context::AppContext;
use super::diesel_schema::{content, content_tags};
use super::models::{Content, ContentTag, ContentWithTags, NewContent, NewContentTag};
use crate::error::StorageError;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type DbConn = PooledConnection<ConnectionManager<SqliteConnection>>;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating content
#[derive(Debug, Clone, Deserialize)]
pub struct CreateContentInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default = "default_content_format")]
    pub content_format: String,
    #[serde(default)]
    pub blob_hash: Option<String>,
    #[serde(default)]
    pub blob_cid: Option<String>,
    #[serde(default)]
    pub content_size_bytes: Option<i32>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default = "default_reach")]
    pub reach: String,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_content_type() -> String {
    "concept".to_string()
}
fn default_content_format() -> String {
    "markdown".to_string()
}
fn default_reach() -> String {
    "public".to_string()
}

/// Query parameters for listing content
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContentQuery {
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub content_format: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Result of bulk operation
#[derive(Debug, Clone, Serialize)]
pub struct BulkResult {
    pub inserted: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get content by ID - scoped by app
pub fn get_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Option<Content>, StorageError> {
    content::table
        .filter(content::app_id.eq(&ctx.app_id))
        .filter(content::id.eq(content_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get content with tags - scoped by app
pub fn get_content_with_tags(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Option<ContentWithTags>, StorageError> {
    let content_opt: Option<Content> = content::table
        .filter(content::app_id.eq(&ctx.app_id))
        .filter(content::id.eq(content_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    match content_opt {
        Some(c) => {
            let tags: Vec<String> = content_tags::table
                .filter(content_tags::app_id.eq(&ctx.app_id))
                .filter(content_tags::content_id.eq(content_id))
                .select(content_tags::tag)
                .load(conn)
                .map_err(|e| StorageError::Internal(format!("Tags query failed: {}", e)))?;

            Ok(Some(ContentWithTags { content: c, tags }))
        }
        None => Ok(None),
    }
}

/// Get tags for a content item - scoped by app
pub fn get_content_tags(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<String>, StorageError> {
    content_tags::table
        .filter(content_tags::app_id.eq(&ctx.app_id))
        .filter(content_tags::content_id.eq(content_id))
        .select(content_tags::tag)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Tags query failed: {}", e)))
}

/// List content with filters - scoped by app
pub fn list_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ContentQuery,
) -> Result<Vec<ContentWithTags>, StorageError> {
    // Prepare search pattern if needed (must outlive the query)
    let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

    // Build base query with app scoping
    let mut base_query = content::table
        .filter(content::app_id.eq(&ctx.app_id))
        .into_boxed();

    // Apply filters
    if let Some(ref ct) = query.content_type {
        base_query = base_query.filter(content::content_type.eq(ct));
    }

    if let Some(ref cf) = query.content_format {
        base_query = base_query.filter(content::content_format.eq(cf));
    }

    if let Some(ref pattern) = search_pattern {
        base_query = base_query.filter(
            content::title
                .like(pattern)
                .or(content::description.like(pattern)),
        );
    }

    // Execute query
    let contents: Vec<Content> = base_query
        .order(content::created_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    // Load tags for each content item
    // Note: This could be optimized with a single query and grouping
    let mut results = Vec::with_capacity(contents.len());
    for c in contents {
        let tags = get_content_tags(conn, ctx, &c.id)?;
        results.push(ContentWithTags { content: c, tags });
    }

    // If filtering by tags, filter results
    if !query.tags.is_empty() {
        results.retain(|c| query.tags.iter().any(|t| c.tags.contains(t)));
    }

    Ok(results)
}

/// Check which content IDs exist - scoped by app
pub fn check_content_exists(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    ids: &[String],
) -> Result<Vec<String>, StorageError> {
    if ids.is_empty() {
        return Ok(vec![]);
    }

    content::table
        .filter(content::app_id.eq(&ctx.app_id))
        .filter(content::id.eq_any(ids))
        .select(content::id)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create a single content item - scoped by app
pub fn create_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateContentInput,
) -> Result<ContentWithTags, StorageError> {
    conn.transaction(|conn| {
        // Insert content
        let new_content = NewContent {
            id: &input.id,
            app_id: &ctx.app_id,
            title: &input.title,
            description: input.description.as_deref(),
            content_type: &input.content_type,
            content_format: &input.content_format,
            blob_hash: input.blob_hash.as_deref(),
            blob_cid: input.blob_cid.as_deref(),
            content_size_bytes: input.content_size_bytes,
            metadata_json: input.metadata_json.as_deref(),
            reach: &input.reach,
            created_by: input.created_by.as_deref(),
        };

        diesel::insert_into(content::table)
            .values(&new_content)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

        // Insert tags
        for tag in &input.tags {
            let new_tag = NewContentTag {
                app_id: &ctx.app_id,
                content_id: &input.id,
                tag,
            };
            diesel::insert_or_ignore_into(content_tags::table)
                .values(&new_tag)
                .execute(conn)
                .map_err(|e| StorageError::Internal(format!("Tag insert failed: {}", e)))?;
        }

        // Return created content with tags
        let content = content::table
            .filter(content::app_id.eq(&ctx.app_id))
            .filter(content::id.eq(&input.id))
            .first(conn)
            .map_err(|e| StorageError::Internal(format!("Fetch failed: {}", e)))?;

        Ok(ContentWithTags {
            content,
            tags: input.tags,
        })
    })
}

/// Bulk create content items (for seeding) - scoped by app
pub fn bulk_create_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    items: Vec<CreateContentInput>,
) -> Result<BulkResult, StorageError> {
    let mut inserted = 0u64;
    let mut skipped = 0u64;
    let mut errors = vec![];

    conn.transaction(|conn| {
        for input in items {
            // Check if exists
            let exists: bool = content::table
                .filter(content::app_id.eq(&ctx.app_id))
                .filter(content::id.eq(&input.id))
                .select(diesel::dsl::count_star())
                .first::<i64>(conn)
                .map(|c| c > 0)
                .unwrap_or(false);

            if exists {
                skipped += 1;
                continue;
            }

            // Insert content
            let new_content = NewContent {
                id: &input.id,
                app_id: &ctx.app_id,
                title: &input.title,
                description: input.description.as_deref(),
                content_type: &input.content_type,
                content_format: &input.content_format,
                blob_hash: input.blob_hash.as_deref(),
                blob_cid: input.blob_cid.as_deref(),
                content_size_bytes: input.content_size_bytes,
                metadata_json: input.metadata_json.as_deref(),
                reach: &input.reach,
                created_by: input.created_by.as_deref(),
            };

            match diesel::insert_into(content::table)
                .values(&new_content)
                .execute(conn)
            {
                Ok(_) => {
                    // Insert tags
                    for tag in &input.tags {
                        let new_tag = NewContentTag {
                            app_id: &ctx.app_id,
                            content_id: &input.id,
                            tag,
                        };
                        let _ = diesel::insert_or_ignore_into(content_tags::table)
                            .values(&new_tag)
                            .execute(conn);
                    }
                    inserted += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", input.id, e));
                }
            }
        }

        Ok(BulkResult {
            inserted,
            skipped,
            errors,
        })
    })
}

/// Delete content by ID - scoped by app
pub fn delete_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<bool, StorageError> {
    let deleted = diesel::delete(
        content::table
            .filter(content::app_id.eq(&ctx.app_id))
            .filter(content::id.eq(content_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted > 0)
}

/// Get content by tag - scoped by app
pub fn get_content_by_tag(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    tag: &str,
    limit: i64,
) -> Result<Vec<ContentWithTags>, StorageError> {
    list_content(
        conn,
        ctx,
        &ContentQuery {
            tags: vec![tag.to_string()],
            limit,
            ..Default::default()
        },
    )
}

// ============================================================================
// Stats
// ============================================================================

/// Get content count for an app
pub fn content_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    content::table
        .filter(content::app_id.eq(&ctx.app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get unique tag count for an app
#[allow(deprecated)]
pub fn tag_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    content_tags::table
        .filter(content_tags::app_id.eq(&ctx.app_id))
        .select(diesel::dsl::count_distinct(content_tags::tag))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sqlite::SqliteConnection;
    use diesel::Connection;

    fn setup_test_db() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:")
            .expect("Failed to create in-memory database");

        // Create content table
        diesel::sql_query(
            r#"
            CREATE TABLE content (
                id TEXT PRIMARY KEY NOT NULL,
                app_id TEXT NOT NULL DEFAULT 'lamad',
                title TEXT NOT NULL,
                description TEXT,
                content_type TEXT NOT NULL DEFAULT 'concept',
                content_format TEXT NOT NULL DEFAULT 'markdown',
                blob_hash TEXT,
                blob_cid TEXT,
                content_size_bytes INTEGER,
                metadata_json TEXT,
                reach TEXT NOT NULL DEFAULT 'public',
                validation_status TEXT NOT NULL DEFAULT 'valid',
                created_by TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&mut conn)
        .expect("Failed to create content table");

        // Create content_tags table
        diesel::sql_query(
            r#"
            CREATE TABLE content_tags (
                app_id TEXT NOT NULL DEFAULT 'lamad',
                content_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (app_id, content_id, tag)
            )
            "#,
        )
        .execute(&mut conn)
        .expect("Failed to create content_tags table");

        conn
    }

    #[test]
    fn test_app_isolation() {
        let mut conn = setup_test_db();

        let lamad_ctx = AppContext::new("lamad");
        let elohim_ctx = AppContext::new("elohim");

        // Create content in lamad app
        let lamad_content = CreateContentInput {
            id: "manifesto".to_string(),
            title: "Lamad Manifesto".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "public".to_string(),
            created_by: None,
            tags: vec!["core".to_string()],
        };
        create_content(&mut conn, &lamad_ctx, lamad_content).unwrap();

        // Create content in elohim app
        let elohim_content = CreateContentInput {
            id: "resources".to_string(),
            title: "Elohim Resources".to_string(),
            description: None,
            content_type: "resource".to_string(),
            content_format: "json".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "public".to_string(),
            created_by: None,
            tags: vec!["infrastructure".to_string()],
        };
        create_content(&mut conn, &elohim_ctx, elohim_content).unwrap();

        // Verify lamad app can see only its content
        let lamad_count = content_count(&mut conn, &lamad_ctx).unwrap();
        assert_eq!(lamad_count, 1, "Lamad should have 1 content item");

        let lamad_manifesto = get_content(&mut conn, &lamad_ctx, "manifesto").unwrap();
        assert!(lamad_manifesto.is_some(), "Lamad should find manifesto");

        let lamad_resources = get_content(&mut conn, &lamad_ctx, "resources").unwrap();
        assert!(lamad_resources.is_none(), "Lamad should NOT find elohim's resources");

        // Verify elohim app can see only its content
        let elohim_count = content_count(&mut conn, &elohim_ctx).unwrap();
        assert_eq!(elohim_count, 1, "Elohim should have 1 content item");

        let elohim_resources = get_content(&mut conn, &elohim_ctx, "resources").unwrap();
        assert!(elohim_resources.is_some(), "Elohim should find resources");

        let elohim_manifesto = get_content(&mut conn, &elohim_ctx, "manifesto").unwrap();
        assert!(elohim_manifesto.is_none(), "Elohim should NOT find lamad's manifesto");
    }

    #[test]
    fn test_bulk_create_app_scoped() {
        let mut conn = setup_test_db();
        let lamad_ctx = AppContext::new("lamad");

        let items = vec![
            CreateContentInput {
                id: "content-1".to_string(),
                title: "Content 1".to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "public".to_string(),
                created_by: None,
                tags: vec![],
            },
            CreateContentInput {
                id: "content-2".to_string(),
                title: "Content 2".to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "public".to_string(),
                created_by: None,
                tags: vec![],
            },
        ];

        let result = bulk_create_content(&mut conn, &lamad_ctx, items).unwrap();
        assert_eq!(result.inserted, 2, "Should insert 2 items");
        assert_eq!(result.skipped, 0, "Should skip 0 items");

        // Try to insert same items again - should skip
        let items2 = vec![
            CreateContentInput {
                id: "content-1".to_string(),
                title: "Content 1 Duplicate".to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "public".to_string(),
                created_by: None,
                tags: vec![],
            },
        ];

        let result2 = bulk_create_content(&mut conn, &lamad_ctx, items2).unwrap();
        assert_eq!(result2.inserted, 0, "Should insert 0 items (duplicate)");
        assert_eq!(result2.skipped, 1, "Should skip 1 item");
    }
}
