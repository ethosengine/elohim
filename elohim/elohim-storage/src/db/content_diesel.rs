//! Content CRUD operations using Diesel with app scoping
//!
//! All operations require an AppContext for multi-tenant isolation.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use serde::{Deserialize, Serialize};

use super::context::AppContext;
use super::diesel_schema::{content, content_tags};
use super::models::{Content, ContentWithTags, NewContent, NewContentTag};
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
    #[serde(default)]
    pub content_body: Option<String>,
}

/// Input for partially updating a content item (PATCH semantics).
/// All fields are `Option` — `None` means "no change".
#[derive(Debug, Default)]
pub struct UpdateContentInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content_body: Option<String>,
    pub content_format: Option<String>,
    /// Already-serialized JSON string. If provided, replaces metadata_json in the row.
    /// The caller (ContentService) is responsible for shallow-merging before calling this.
    pub metadata_json: Option<String>,
    /// If provided, replaces all existing tags (delete all + insert new).
    pub tags: Option<Vec<String>>,
    pub reach: Option<String>,
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

/// Deserialize a comma-separated string into a Vec<String>.
/// Handles `?tags=a,b,c` (Angular convention) via serde_urlencoded.
fn deserialize_comma_separated<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.map(|s| {
        s.split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect()
    })
    .unwrap_or_default())
}

/// Query parameters for listing content - camelCase for URL params.
/// Deserialized via `serde_urlencoded::from_str()` in the HTTP handler.
///
/// This struct carries ONLY client-controllable filter criteria. The
/// provenance gate is a separate server-side parameter on `list_content` /
/// `count_content` so that clients cannot disable it via URL params.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentQuery {
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub content_format: Option<String>,
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
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
///
/// `require_provenance`: when true, rows lacking both `dht_anchor_hash` and
/// `p2p_published_at` are filtered out — returning `Ok(None)` as if the row
/// did not exist. External HTTP handlers should pass `true`; internal drain
/// and replication paths should pass `false`.
pub fn get_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
    require_provenance: bool,
) -> Result<Option<Content>, StorageError> {
    let mut q = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(content_id))
        .into_boxed();

    if require_provenance {
        q = q.filter(
            content::dht_anchor_hash
                .is_not_null()
                .or(content::p2p_published_at.is_not_null()),
        );
    }

    q.first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get content with tags - scoped by app
///
/// `require_provenance`: see [`get_content`]. External HTTP handlers (e.g.
/// `/epr-head/{id}`) should pass `true`; internal replication/sync paths in
/// `p2p/mod.rs` should pass `false` so unpublished rows remain visible to the
/// drain loop and shard inventory.
pub fn get_content_with_tags(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
    require_provenance: bool,
) -> Result<Option<ContentWithTags>, StorageError> {
    let mut q = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(content_id))
        .into_boxed();

    if require_provenance {
        q = q.filter(
            content::dht_anchor_hash
                .is_not_null()
                .or(content::p2p_published_at.is_not_null()),
        );
    }

    let content_opt: Option<Content> = q
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    match content_opt {
        Some(c) => {
            let tags: Vec<String> = content_tags::table
                .filter(content_tags::h_app_id.eq(&ctx.h_app_id))
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
        .filter(content_tags::h_app_id.eq(&ctx.h_app_id))
        .filter(content_tags::content_id.eq(content_id))
        .select(content_tags::tag)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Tags query failed: {}", e)))
}

/// List content with filters - scoped by app
///
/// `require_provenance`: when true, rows lacking both `dht_anchor_hash` and
/// `p2p_published_at` are filtered out. External HTTP handlers MUST pass
/// `true`; internal drain/replication/sync paths pass `false`. This is a
/// server-side parameter — NOT part of `ContentQuery` — so that clients
/// cannot disable the gate via URL params.
pub fn list_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ContentQuery,
    require_provenance: bool,
) -> Result<Vec<ContentWithTags>, StorageError> {
    // Prepare search pattern if needed (must outlive the query)
    let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

    // Build base query with app scoping
    let mut base_query = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
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

    if let Some(ref reach) = query.reach {
        base_query = base_query.filter(content::reach.eq(reach));
    }

    // Provenance gate: exclude rows that have been neither notarized on Holochain
    // (dht_anchor_hash) nor published to libp2p Kad (p2p_published_at). Either marker
    // is sufficient. External HTTP reads set this to true; internal drain-loop
    // queries set it to false so the loop can see unpublished rows.
    if require_provenance {
        base_query = base_query.filter(
            content::dht_anchor_hash
                .is_not_null()
                .or(content::p2p_published_at.is_not_null()),
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
        .filter(content::h_app_id.eq(&ctx.h_app_id))
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
            h_app_id: &ctx.h_app_id,
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
            content_body: input.content_body.as_deref(),
        };

        diesel::insert_into(content::table)
            .values(&new_content)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

        // Insert tags
        for tag in &input.tags {
            let new_tag = NewContentTag {
                h_app_id: &ctx.h_app_id,
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
            .filter(content::h_app_id.eq(&ctx.h_app_id))
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
                .filter(content::h_app_id.eq(&ctx.h_app_id))
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
                h_app_id: &ctx.h_app_id,
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
                content_body: input.content_body.as_deref(),
            };

            match diesel::insert_into(content::table)
                .values(&new_content)
                .execute(conn)
            {
                Ok(_) => {
                    // Insert tags
                    for tag in &input.tags {
                        let new_tag = NewContentTag {
                            h_app_id: &ctx.h_app_id,
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

/// Update a content item with partial (PATCH) semantics — scoped by app.
///
/// Only fields present in `input` are applied. Tags, if provided, replace all existing tags.
/// Returns the updated `ContentWithTags`.
pub fn update_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: UpdateContentInput,
) -> Result<ContentWithTags, StorageError> {
    use super::models::current_timestamp;

    let id = &input.id;

    // Verify the row exists in this app scope. Internal update path — always
    // `require_provenance: false` so we can update rows pre-drain.
    let existing = get_content_with_tags(conn, ctx, id, false)?
        .ok_or_else(|| StorageError::NotFound(format!("Content not found: {}", id)))?;

    // Apply scalar field updates — use provided value or fall back to existing
    let new_title = input.title.as_deref().unwrap_or(&existing.content.title);
    let new_description = input
        .description
        .as_deref()
        .or(existing.content.description.as_deref());
    let new_content_body = input
        .content_body
        .as_deref()
        .or(existing.content.content_body.as_deref());
    let new_content_format = input
        .content_format
        .as_deref()
        .unwrap_or(&existing.content.content_format);
    let new_reach = input.reach.as_deref().unwrap_or(&existing.content.reach);
    let new_metadata_json = input
        .metadata_json
        .as_deref()
        .or(existing.content.metadata_json.as_deref());

    let now = current_timestamp();

    conn.transaction(|conn| {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set((
            content::title.eq(new_title),
            content::description.eq(new_description),
            content::content_body.eq(new_content_body),
            content::content_format.eq(new_content_format),
            content::metadata_json.eq(new_metadata_json),
            content::reach.eq(new_reach),
            content::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

        // Replace tags if provided
        if let Some(ref new_tags) = input.tags {
            diesel::delete(
                content_tags::table
                    .filter(content_tags::h_app_id.eq(&ctx.h_app_id))
                    .filter(content_tags::content_id.eq(id)),
            )
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Tag delete failed: {}", e)))?;

            for tag in new_tags {
                let new_tag = NewContentTag {
                    h_app_id: &ctx.h_app_id,
                    content_id: id,
                    tag,
                };
                diesel::insert_or_ignore_into(content_tags::table)
                    .values(&new_tag)
                    .execute(conn)
                    .map_err(|e| StorageError::Internal(format!("Tag insert failed: {}", e)))?;
            }
        }

        // Return updated record — internal fetch, no provenance gate.
        get_content_with_tags(conn, ctx, id, false)?
            .ok_or_else(|| StorageError::Internal("Failed to fetch updated content".into()))
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
            .filter(content::h_app_id.eq(&ctx.h_app_id))
            .filter(content::id.eq(content_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    Ok(deleted > 0)
}

/// Internal-only: lists content rows tagged with `tag`. The `require_provenance`
/// parameter must be passed by the caller; pass `false` for internal callers
/// (this function has no external HTTP route as of 2026-04-08).
pub fn get_content_by_tag(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    tag: &str,
    limit: i64,
) -> Result<Vec<ContentWithTags>, StorageError> {
    // Internal helper — ContentService exposes this only through internal
    // methods. External HTTP handlers do not route through this path.
    list_content(
        conn,
        ctx,
        &ContentQuery {
            tags: vec![tag.to_string()],
            limit,
            ..Default::default()
        },
        false,
    )
}

// ============================================================================
// Stats
// ============================================================================

/// Get content count for an app
pub fn content_count(conn: &mut SqliteConnection, ctx: &AppContext) -> Result<i64, StorageError> {
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Count content matching a query (respects filters, ignores limit/offset).
/// Used for pagination total counts.
///
/// `require_provenance`: see [`list_content`]. External paginators MUST pass
/// `true` so totals stay consistent with the gated row set.
pub fn count_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ContentQuery,
    require_provenance: bool,
) -> Result<i64, StorageError> {
    let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

    let mut base_query = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .into_boxed();

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
    if let Some(ref reach) = query.reach {
        base_query = base_query.filter(content::reach.eq(reach));
    }

    // Provenance gate: mirrors list_content. External paginators must set this
    // to true so totals stay consistent with the filtered row set.
    if require_provenance {
        base_query = base_query.filter(
            content::dht_anchor_hash
                .is_not_null()
                .or(content::p2p_published_at.is_not_null()),
        );
    }

    base_query
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Get unique tag count for an app
#[allow(deprecated)]
pub fn tag_count(conn: &mut SqliteConnection, ctx: &AppContext) -> Result<i64, StorageError> {
    content_tags::table
        .filter(content_tags::h_app_id.eq(&ctx.h_app_id))
        .select(diesel::dsl::count_distinct(content_tags::tag))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

/// Drain-loop query: return IDs of content rows that have not yet been
/// published to the libp2p Kad DHT. Scoped by app context. Internal use
/// only — does not apply the provenance gate (the drain loop IS the thing
/// that produces provenance). A non-positive limit returns an empty vec.
pub fn list_unpublished_content_ids(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
) -> Result<Vec<String>, StorageError> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::p2p_published_at.is_null())
        .select(content::id)
        .order((content::created_at.asc(), content::id.asc()))
        .limit(limit)
        .load::<String>(conn)
        .map_err(|e| StorageError::Internal(format!("list_unpublished_content_ids failed: {}", e)))
}

/// Drain-loop write: mark a content row as p2p_published at the current time.
/// Operates on the operational projection column only — does not touch
/// dht_anchor_hash or any notarized state. Idempotent — re-publishing an
/// already-published row just bumps the timestamp.
///
/// Returns `Ok(true)` if the row was marked, `Ok(false)` if the row was
/// concurrently deleted (e.g. by a purge between the caller's list query
/// and this call) — the latter is NOT an error; the DHT publish already
/// succeeded and there is nothing to mark.
pub fn mark_published(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<bool, StorageError> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let rows = diesel::update(
        content::table
            .filter(content::h_app_id.eq(&ctx.h_app_id))
            .filter(content::id.eq(content_id)),
    )
    .set(content::p2p_published_at.eq(now))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("mark_published failed: {}", e)))?;
    Ok(rows > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sqlite::SqliteConnection;
    use diesel::Connection;

    fn setup_test_db() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");

        // Create content table
        diesel::sql_query(
            r#"
            CREATE TABLE content (
                id TEXT PRIMARY KEY NOT NULL,
                h_app_id TEXT NOT NULL DEFAULT 'lamad',
                title TEXT NOT NULL,
                description TEXT,
                content_type TEXT NOT NULL DEFAULT 'concept',
                content_format TEXT NOT NULL DEFAULT 'markdown',
                content_body TEXT,
                blob_hash TEXT,
                blob_cid TEXT,
                content_size_bytes INTEGER,
                metadata_json TEXT,
                reach TEXT NOT NULL DEFAULT 'public',
                validation_status TEXT NOT NULL DEFAULT 'valid',
                created_by TEXT,
                dht_anchor_hash TEXT,
                p2p_published_at TEXT,
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
                h_app_id TEXT NOT NULL DEFAULT 'lamad',
                content_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (h_app_id, content_id, tag)
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
            content_body: None,
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
            content_body: None,
        };
        create_content(&mut conn, &elohim_ctx, elohim_content).unwrap();

        // Verify lamad app can see only its content
        let lamad_count = content_count(&mut conn, &lamad_ctx).unwrap();
        assert_eq!(lamad_count, 1, "Lamad should have 1 content item");

        let lamad_manifesto = get_content(&mut conn, &lamad_ctx, "manifesto", false).unwrap();
        assert!(lamad_manifesto.is_some(), "Lamad should find manifesto");

        let lamad_resources = get_content(&mut conn, &lamad_ctx, "resources", false).unwrap();
        assert!(
            lamad_resources.is_none(),
            "Lamad should NOT find elohim's resources"
        );

        // Verify elohim app can see only its content
        let elohim_count = content_count(&mut conn, &elohim_ctx).unwrap();
        assert_eq!(elohim_count, 1, "Elohim should have 1 content item");

        let elohim_resources = get_content(&mut conn, &elohim_ctx, "resources", false).unwrap();
        assert!(elohim_resources.is_some(), "Elohim should find resources");

        let elohim_manifesto = get_content(&mut conn, &elohim_ctx, "manifesto", false).unwrap();
        assert!(
            elohim_manifesto.is_none(),
            "Elohim should NOT find lamad's manifesto"
        );
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
                content_body: None,
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
                content_body: None,
            },
        ];

        let result = bulk_create_content(&mut conn, &lamad_ctx, items).unwrap();
        assert_eq!(result.inserted, 2, "Should insert 2 items");
        assert_eq!(result.skipped, 0, "Should skip 0 items");

        // Try to insert same items again - should skip
        let items2 = vec![CreateContentInput {
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
            content_body: None,
        }];

        let result2 = bulk_create_content(&mut conn, &lamad_ctx, items2).unwrap();
        assert_eq!(result2.inserted, 0, "Should insert 0 items (duplicate)");
        assert_eq!(result2.skipped, 1, "Should skip 1 item");
    }

    #[test]
    fn test_list_content_respects_require_provenance() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Insert two rows: one will be marked published, one not.
        let published = CreateContentInput {
            id: "cid-published".to_string(),
            title: "Published".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
        };
        let unpublished = CreateContentInput {
            id: "cid-unpublished".to_string(),
            ..published.clone()
        };
        create_content(&mut conn, &ctx, published).unwrap();
        create_content(&mut conn, &ctx, unpublished).unwrap();

        // Mark only the first as p2p_published.
        diesel::sql_query(
            "UPDATE content SET p2p_published_at = datetime('now') WHERE id = 'cid-published'",
        )
        .execute(&mut conn)
        .unwrap();

        // Default query (no provenance filter) — returns BOTH rows. Regression guard.
        let unrestricted = list_content(
            &mut conn,
            &ctx,
            &ContentQuery {
                limit: 10,
                offset: 0,
                ..Default::default()
            },
            false,
        )
        .unwrap();
        assert_eq!(
            unrestricted.len(),
            2,
            "unrestricted list should return all rows"
        );

        // Gated query — returns ONLY the published row.
        let gated = list_content(
            &mut conn,
            &ctx,
            &ContentQuery {
                limit: 10,
                offset: 0,
                ..Default::default()
            },
            true,
        )
        .unwrap();
        assert_eq!(
            gated.len(),
            1,
            "gated list should filter out unpublished rows"
        );
        assert_eq!(gated[0].content.id, "cid-published");
    }

    #[test]
    fn test_count_content_respects_require_provenance() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let published = CreateContentInput {
            id: "cid-published".to_string(),
            title: "Published".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
        };
        let unpublished = CreateContentInput {
            id: "cid-unpublished".to_string(),
            ..published.clone()
        };
        create_content(&mut conn, &ctx, published).unwrap();
        create_content(&mut conn, &ctx, unpublished).unwrap();

        diesel::sql_query(
            "UPDATE content SET p2p_published_at = datetime('now') WHERE id = 'cid-published'",
        )
        .execute(&mut conn)
        .unwrap();

        let unrestricted = count_content(&mut conn, &ctx, &ContentQuery::default(), false).unwrap();
        assert_eq!(unrestricted, 2, "unrestricted count should see both rows");

        let gated = count_content(&mut conn, &ctx, &ContentQuery::default(), true).unwrap();
        assert_eq!(gated, 1, "gated count should filter out unpublished rows");
    }

    #[test]
    fn test_get_content_respects_require_provenance() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let row = CreateContentInput {
            id: "cid-unpublished".to_string(),
            title: "Unpublished".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
        };
        create_content(&mut conn, &ctx, row).unwrap();

        // Gated fetch: row has no dht_anchor_hash and no p2p_published_at →
        // external reads must see None, as if the row did not exist.
        let gated = get_content(&mut conn, &ctx, "cid-unpublished", true).unwrap();
        assert!(
            gated.is_none(),
            "gated get_content must hide unpublished rows from external readers"
        );

        // Ungated fetch: internal callers (drain loop) must still see it.
        let ungated = get_content(&mut conn, &ctx, "cid-unpublished", false).unwrap();
        assert!(
            ungated.is_some(),
            "ungated get_content must still return unpublished rows for internal callers"
        );
        assert_eq!(ungated.unwrap().id, "cid-unpublished");
    }

    /// Regression guard for the A4 critical fix: the external HTTP handler
    /// `handle_db_content_list` MUST pass `require_provenance=true` so that
    /// unpublished rows never leak through `GET /db/content`. This test
    /// exercises the exact call pattern the handler uses.
    #[test]
    fn test_list_content_external_call_pattern_excludes_unpublished() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let row = CreateContentInput {
            id: "cid-unpublished".to_string(),
            title: "Unpublished".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
        };
        create_content(&mut conn, &ctx, row).unwrap();

        let q = ContentQuery {
            limit: 10,
            ..Default::default()
        };

        // External call pattern — what handle_db_content_list will invoke.
        let external = list_content(&mut conn, &ctx, &q, true).unwrap();
        assert!(
            external.is_empty(),
            "external list must not leak unpublished content"
        );

        // Internal call pattern — what p2p replication/drain paths invoke.
        let internal = list_content(&mut conn, &ctx, &q, false).unwrap();
        assert_eq!(
            internal.len(),
            1,
            "internal list must still see unpublished content"
        );
    }

    /// Regression guard for the A4 critical fix: the external HTTP handler
    /// `handle_db_content_by_id` MUST call `get_content_with_tags` with
    /// `require_provenance=true` so that `GET /db/content/{id}` never leaks
    /// unpublished rows.
    #[test]
    fn test_get_content_external_call_pattern_excludes_unpublished() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let row = CreateContentInput {
            id: "cid-unpublished".to_string(),
            title: "Unpublished".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
        };
        create_content(&mut conn, &ctx, row).unwrap();

        // External call pattern — what handle_db_content_by_id will invoke.
        let external = get_content_with_tags(&mut conn, &ctx, "cid-unpublished", true).unwrap();
        assert!(
            external.is_none(),
            "external get must not leak unpublished content"
        );

        // Internal call pattern.
        let internal = get_content_with_tags(&mut conn, &ctx, "cid-unpublished", false).unwrap();
        assert!(
            internal.is_some(),
            "internal get must still see unpublished content"
        );
    }

    #[test]
    fn test_list_unpublished_content_ids_returns_only_unpublished() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Three rows: A unpublished, B published, C unpublished.
        for id in ["a", "b", "c"] {
            create_content(
                &mut conn,
                &ctx,
                CreateContentInput {
                    id: id.into(),
                    title: id.to_uppercase(),
                    description: None,
                    content_type: "concept".into(),
                    content_format: "markdown".into(),
                    blob_hash: None,
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "commons".into(),
                    created_by: None,
                    tags: vec![],
                    content_body: None,
                },
            )
            .unwrap();
        }

        diesel::sql_query("UPDATE content SET p2p_published_at = datetime('now') WHERE id = 'b'")
            .execute(&mut conn)
            .unwrap();

        let pending = list_unpublished_content_ids(&mut conn, &ctx, 100).unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.contains(&"a".to_string()));
        assert!(pending.contains(&"c".to_string()));
    }

    #[test]
    fn test_mark_published_sets_timestamp() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        create_content(
            &mut conn,
            &ctx,
            CreateContentInput {
                id: "x".into(),
                title: "X".into(),
                description: None,
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "commons".into(),
                created_by: None,
                tags: vec![],
                content_body: None,
            },
        )
        .unwrap();

        let pending_before = list_unpublished_content_ids(&mut conn, &ctx, 10).unwrap();
        assert_eq!(pending_before.len(), 1);

        let marked = mark_published(&mut conn, &ctx, "x").unwrap();
        assert!(marked, "mark_published should return true for an existing row");

        let pending_after = list_unpublished_content_ids(&mut conn, &ctx, 10).unwrap();
        assert!(pending_after.is_empty());

        // Verify the timestamp column was actually written with the canonical RFC 3339 Zulu format.
        let ts_opt: Option<String> = content::table
            .filter(content::h_app_id.eq(&ctx.h_app_id))
            .filter(content::id.eq("x"))
            .select(content::p2p_published_at)
            .first(&mut conn)
            .unwrap();
        let ts = ts_opt.expect("p2p_published_at should be set");
        assert!(ts.ends_with('Z'), "expected Zulu-suffixed RFC 3339, got: {}", ts);
        chrono::DateTime::parse_from_rfc3339(&ts)
            .expect("p2p_published_at should parse as RFC 3339");

        // Also verify mark_published on a non-existent row returns false (not an error).
        let missing = mark_published(&mut conn, &ctx, "nonexistent").unwrap();
        assert!(!missing, "mark_published should return false for a missing row");
    }

    #[test]
    fn test_update_content_input_struct_exists() {
        let input = UpdateContentInput {
            id: "test-id".to_string(),
            title: None,
            description: None,
            content_body: None,
            content_format: None,
            metadata_json: None,
            tags: None,
            reach: None,
        };
        assert_eq!(input.id, "test-id");
    }
}
