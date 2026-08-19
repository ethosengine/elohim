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
// Trust gate
// ============================================================================

/// Minimum trust tier a read admits — the "HTTP vs HTTPS" serving gate.
/// (CRDT:HTTP :: CRDT+DHT-notary:HTTPS — amber serves like HTTP "Not secure".)
///
/// This replaces the earlier binary `require_provenance: bool` on the content
/// read fns. The fixed migration mapping is `true → Amber`, `false → Invisible`
/// — behaviour-preserving for all current data because `crdt_converged_at` is
/// NULL on every row until the deploy-producer (A3) lands, so `Amber` equals
/// the old `dht_anchor_hash OR p2p_published_at` gate today while additionally
/// admitting future amber (converged-only) rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinTrust {
    /// No trust filter — internal callers (drain/sync/replication) see all rows.
    Invisible,
    /// Serving floor: converged OR published OR notarized (admits the amber tier).
    Amber,
    /// Peer-attested custody or notarized (no bare-converged).
    Blue,
    /// Notarized only — authority/attribution/economic reads.
    Green,
}

/// Apply the trust gate as a per-row WHERE filter on a boxed `content` query.
///
/// REQ-N7: this is a per-row WHERE filter, never a fail-closed collect. The
/// boxed query is returned so callers keep their `.into_boxed()` chain intact.
fn apply_min_trust<'a>(
    q: content::BoxedQuery<'a, diesel::sqlite::Sqlite>,
    min_trust: MinTrust,
) -> content::BoxedQuery<'a, diesel::sqlite::Sqlite> {
    match min_trust {
        MinTrust::Invisible => q,
        MinTrust::Amber => q.filter(
            content::dht_anchor_hash
                .is_not_null()
                .or(content::p2p_published_at.is_not_null())
                .or(content::crdt_converged_at.is_not_null()),
        ),
        MinTrust::Blue => q.filter(
            content::dht_anchor_hash
                .is_not_null()
                .or(content::p2p_published_at.is_not_null()),
        ),
        MinTrust::Green => q.filter(content::dht_anchor_hash.is_not_null()),
    }
}

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
    /// Content-derived provenance anchor written at ingest. See
    /// `CreateContentInputView::dht_anchor_hash`. None → column stays NULL
    /// (the peered drain or `ContentCommitted` projection stamps it later).
    #[serde(default)]
    pub dht_anchor_hash: Option<String>,
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
    /// Content-addressed SHA256 (e.g. "sha256-…") of the bundle this row
    /// projects. Written by Jenkinsfile:stageSpaBlob at deploy time. None
    /// means "no change" — preserves existing blob_hash on the row.
    pub blob_hash: Option<String>,
    /// Content-addressed hash of the Angular SSR *server* bundle this row
    /// projects (the browser bundle is `blob_hash`). Written by the Jenkins SSR
    /// PATCH at deploy time. None means "no change" — preserves the existing
    /// `server_blob_hash` on the row (no-clobber, mirrors `blob_hash`).
    pub server_blob_hash: Option<String>,
    /// RFC-3339 timestamp marking DHT publication. The drain loop is the
    /// canonical writer (see `mark_published`); this PATCH-path field lets the
    /// genesis seeder stamp it directly so household/local stacks with no DHT
    /// peers still satisfy the `require_provenance` read gate. None means
    /// "no change" — preserves the existing value on the row.
    pub p2p_published_at: Option<String>,
    /// RFC-3339 timestamp marking the "amber" tier — `blob_hash` populated by
    /// the deploy-time non-notarized producer (A3) or CRDT convergence, NOT by
    /// DHT notarization. `Some(_)` switches this update into the AMBER path,
    /// which additionally applies no-clobber precedence to `blob_hash`: the
    /// amber `blob_hash` is written ONLY if the existing row's `blob_hash` is
    /// NULL/empty, so a later notarized (green) `blob_hash` is never overwritten
    /// by amber. `None` = normal update path (no amber stamp, blob_hash
    /// coalesces as usual). NEVER pair this with a `dht_anchor_hash` write —
    /// amber must not launder into notarized provenance (Content is not in
    /// `is_integrity_kind`, so this diesel-direct write authors no DHT entry and
    /// is not reverted by the reconcile controller).
    pub crdt_converged_at: Option<String>,
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
/// `min_trust`: the trust gate. Rows below the tier are filtered out —
/// returning `Ok(None)` as if the row did not exist. External HTTP handlers
/// pass `MinTrust::Amber` (the serving floor); internal drain and replication
/// paths pass `MinTrust::Invisible`. See [`MinTrust`].
pub fn get_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
    min_trust: MinTrust,
) -> Result<Option<Content>, StorageError> {
    let q = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(content_id))
        .into_boxed();
    let q = apply_min_trust(q, min_trust);

    q.first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get content with tags - scoped by app
///
/// `min_trust`: see [`get_content`]. External HTTP handlers (e.g.
/// `/epr-head/{id}`) pass `MinTrust::Amber`; internal replication/sync paths in
/// `p2p/mod.rs` pass `MinTrust::Invisible` so unpublished rows remain visible to
/// the drain loop and shard inventory.
pub fn get_content_with_tags(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
    min_trust: MinTrust,
) -> Result<Option<ContentWithTags>, StorageError> {
    let q = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(content_id))
        .into_boxed();
    let q = apply_min_trust(q, min_trust);

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
/// Page ALL content rows across every app scope, provenance-ungated.
///
/// Used ONLY by the Automerge DocStore corpus back-fill
/// (`crate::sync::projector::backfill_content_docs`) — a projection REBUILD over
/// already-notarized content, NOT an HTTP read surface, hence no `h_app_id`
/// scoping and no provenance gate. Ordered by `id` for stable pagination across
/// pages. `offset`/`limit` are clamped to sane values.
pub fn list_all_content_rows(
    conn: &mut SqliteConnection,
    offset: i64,
    limit: i64,
) -> Result<Vec<Content>, StorageError> {
    content::table
        .order(content::id.asc())
        .offset(offset.max(0))
        .limit(limit.max(1))
        .load::<Content>(conn)
        .map_err(|e| StorageError::Internal(format!("list_all_content_rows failed: {}", e)))
}

pub fn list_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ContentQuery,
    min_trust: MinTrust,
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

    // Trust gate: exclude rows below the requested tier. `Amber` (the serving
    // floor) admits rows notarized on Holochain (dht_anchor_hash), published to
    // libp2p Kad (p2p_published_at), OR CRDT-converged (crdt_converged_at) —
    // any one marker suffices. External HTTP reads pass `Amber`; internal
    // drain-loop queries pass `Invisible` so the loop can see unpublished rows.
    base_query = apply_min_trust(base_query, min_trust);

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
            dht_anchor_hash: input.dht_anchor_hash.as_deref(),
            // SSR server bundle hash is deploy-PATCH populated, never at create time.
            server_blob_hash: None,
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
                dht_anchor_hash: input.dht_anchor_hash.as_deref(),
                // SSR server bundle hash is deploy-PATCH populated, never at seed time.
                server_blob_hash: None,
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
    // `MinTrust::Invisible` so we can update rows pre-drain.
    let existing = get_content_with_tags(conn, ctx, id, MinTrust::Invisible)?
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
    // blob_hash resolution. Normal path: coalesce (provided value wins, else
    // keep existing). AMBER path (`crdt_converged_at.is_some()`): GREEN is
    // inviolable — an amber write never overwrites a notarized blob_hash
    // (`dht_anchor_hash` present); only the reverse (green overwrites amber).
    // Amber MAY replace a non-green blob_hash: re-stages and CRDT heals must
    // converge set→set, else a peer holding a stale hash re-asserts it (and
    // re-serves a possibly-dead blob) forever. (Earlier write-iff-empty
    // semantics made the very first heal permanent — 2026-07-02 review
    // MAJOR-1, stale-restarter case.)
    let existing_is_green = existing.content.dht_anchor_hash.is_some();
    let is_amber_write = input.crdt_converged_at.is_some();
    let new_blob_hash = if is_amber_write && existing_is_green {
        // Amber must not clobber a notarized (green) blob_hash.
        existing.content.blob_hash.as_deref()
    } else {
        input
            .blob_hash
            .as_deref()
            .or(existing.content.blob_hash.as_deref())
    };
    // crdt_converged_at: no-clobber preserve on the normal path; stamp on the
    // amber path. Never touches dht_anchor_hash (this UPDATE has no anchor set).
    let new_crdt_converged_at = input
        .crdt_converged_at
        .as_deref()
        .or(existing.content.crdt_converged_at.as_deref());
    // No-clobber: a serverBlobHash-only PATCH falls back to the existing
    // server_blob_hash; a blob_hash-only PATCH leaves server_blob_hash untouched.
    let new_server_blob_hash = input
        .server_blob_hash
        .as_deref()
        .or(existing.content.server_blob_hash.as_deref());
    let new_p2p_published_at = input
        .p2p_published_at
        .as_deref()
        .or(existing.content.p2p_published_at.as_deref());

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
            content::blob_hash.eq(new_blob_hash),
            content::server_blob_hash.eq(new_server_blob_hash),
            content::p2p_published_at.eq(new_p2p_published_at),
            content::crdt_converged_at.eq(new_crdt_converged_at),
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

        // Return updated record — internal fetch, no trust gate.
        get_content_with_tags(conn, ctx, id, MinTrust::Invisible)?
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

/// Internal-only: lists content rows tagged with `tag`. Internal helper —
/// always `MinTrust::Invisible` (this function has no external HTTP route as of
/// 2026-04-08).
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
        MinTrust::Invisible,
    )
}

// ============================================================================
// Substrate projection — DNA-anchored upsert
// ============================================================================

/// Patch applied to a content row by the projection signal handler.
///
/// Mirrors the DHT Content entry's mutable fields. `None` means "absent
/// from the entry" (the entry's field was null/missing) — the upsert
/// preserves the existing SQL column unless the entry carried a value.
#[derive(Debug, Clone, Default)]
pub struct ContentProjectionPatch {
    pub blob_cid: Option<String>,
    /// SQL column is i32; DNA entry is u64 — caller downcasts.
    pub content_size_bytes: Option<i32>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content_type: Option<String>,
    pub content_format: Option<String>,
    pub reach: Option<String>,
    pub metadata_json: Option<String>,
}

/// Apply the present fields of a `ContentProjectionPatch` to an EXISTING content
/// row via targeted per-field UPDATEs (`None` fields preserve the existing
/// column). Mirrors `blob_cid` into the legacy `blob_hash` column so downstream
/// readers keyed on `blob_hash` (SSR fetch shim, list/get views) see the new
/// content address (same SHA256 per the Phase 0 refactor).
///
/// Shared by [`upsert_with_anchor`] and [`stamp_declared_head`] so the per-field
/// projection semantics stay byte-identical between the two write paths.
fn apply_content_patch_fields(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    patch: &ContentProjectionPatch,
) -> Result<(), StorageError> {
    if let Some(ref v) = patch.blob_cid {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set((content::blob_cid.eq(v), content::blob_hash.eq(v)))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update blob_cid failed: {}", e)))?;
    }
    if let Some(v) = patch.content_size_bytes {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set(content::content_size_bytes.eq(v))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update size failed: {}", e)))?;
    }
    if let Some(ref v) = patch.title {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set(content::title.eq(v))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update title failed: {}", e)))?;
    }
    if let Some(ref v) = patch.description {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set(content::description.eq(v))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update description failed: {}", e)))?;
    }
    if let Some(ref v) = patch.metadata_json {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set(content::metadata_json.eq(v))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update metadata failed: {}", e)))?;
    }
    Ok(())
}

/// Whether an own-conductor anchor projection also ELECTS the row's
/// notary-declared head.
///
/// The two acts were fused (every commit crowned itself); splitting them is the
/// substrate half of adopt-before-author. Callers pick explicitly — this is
/// deliberately NOT inferable from the row, because "the row already has a
/// declaration" is true both for a head this peer legitimately adopted and for
/// one it wrongly self-elected on a previous boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadElection {
    /// Deliberate, request-borne (or otherwise authority-bearing) write: the
    /// committed action BECOMES the declared head. `declared_head_at` is NULLed
    /// alongside it because a commit signal carries no declaration `Timestamp`,
    /// and a stale ordering left over from a prior declaration would misorder
    /// every later heal-monotonicity check (the next timestamped declare
    /// restores it).
    ///
    /// Reserved for channels whose whole job is to MOVE the head: the HTTP
    /// re-notarize PATCH, and the declare routes' own eager stamps.
    Declare,
    /// Heal-class write: stamp the ANCHOR, leave the DECLARATION exactly as it
    /// was — including leaving it ABSENT. A row with no declaration stays
    /// undeclared (a self-authored root is not an election, so any later
    /// canonical declaration wins outright); a row that already carries one
    /// keeps BOTH `declared_head_action_hash` and `declared_head_at` untouched.
    ///
    /// Every re-author path uses this: the boot re-anchor sweep, the ghost-anchor
    /// witness, and the `ContentCommitted` projection those sweeps trigger. Without
    /// the last one the fix would be cosmetic — the async signal would re-crown the
    /// row moments after the eager write preserved it.
    PreserveExistingDeclaration,
}

/// Upsert a content row with a DHT anchor hash. Used by the post-commit
/// signal handler in rea_projection to project ContentCommitted signals
/// from the lamad DNA into local SQL.
///
/// Behaviour:
/// - If the row exists: UPDATE the columns present in `patch` (None →
///   preserve existing) AND set `dht_anchor_hash`. Also mirrors `blob_cid`
///   to the legacy `blob_hash` column so downstream readers that still
///   key on `blob_hash` (e.g. SSR fetch shim) see the new content address.
/// - If the row does not exist: INSERT the minimum row using whatever the
///   patch provides, defaulting required-non-null columns. The substrate
///   should normally hit the "update existing" branch since seeded content
///   pre-existed in SQL via `bulk_create_content`; the insert path is the
///   defensive fallback for content that was authored on a peer that hadn't
///   yet seeded.
///
/// Anchor invariant: `dht_anchor_hash` is always set to the value passed,
/// even on the update branch, so re-projection after entry-update
/// (post update_content zome fn) advances the anchor to the new ActionHash.
///
/// HEAD-election rule: an own-conductor commit is NOT automatically a
/// declaration — the caller elects, via [`HeadElection`].
///
/// ## Why this is a parameter and not a rule (the adopt-before-author fix)
///
/// This path used to ALWAYS self-declare: every own-conductor commit set
/// `declared_head_action_hash = dht_anchor_hash` and NULLed `declared_head_at`.
/// The justification was a "single-author model" — the author's latest commit
/// IS the head. That justification is FALSE for cross-root ids: two peers can
/// each hold a root for `elohim-host-landing`, and each restart's re-anchor /
/// ghost-witness sweep re-authored a fresh root and immediately crowned it,
/// clobbering a canonical head the peer had already adopted. The peer could
/// never converge because its own heal loop kept re-electing itself.
///
/// The cure is to make authoring and declaring separate acts. A heal-class
/// author (re-anchor, ghost witness, projection of a commit those sweeps
/// caused) stamps the ANCHOR only and leaves the declaration alone; a
/// deliberate, request-borne channel declares. Which one applies is decided
/// EXPLICITLY at the call site — never inferred from row state, because the row
/// state is exactly what the bug corrupted.
///
/// The reconcile leg's verified-stamp entrypoint stays [`stamp_declared_head`];
/// the HEAD is NEVER written from CRDT/gossip input on any election.
pub fn upsert_with_anchor(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    patch: ContentProjectionPatch,
    dht_anchor_hash: &str,
    election: HeadElection,
) -> Result<(), StorageError> {
    use diesel::dsl::sql;
    use diesel::sql_types::Text;

    let existing = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(id))
        .select(diesel::dsl::count_star())
        .first::<i64>(conn)
        .map(|c| c > 0)
        .unwrap_or(false);

    if existing {
        // Build a single UPDATE that touches anchor + whichever patch fields
        // are present. Diesel doesn't generate dynamic SET clauses cleanly,
        // so we run targeted updates per field. Each diesel::update is
        // wrapped in the same transaction by the caller (the signal handler
        // takes a pooled connection; this fn runs inside a single statement
        // sequence).
        //
        // Two SET shapes rather than one dynamic clause: `PreserveExistingDeclaration`
        // must not NAME the declaration columns at all, so an in-flight declare
        // landing between these statements can never be clobbered by a stale value
        // this heal read a moment ago.
        match election {
            HeadElection::Declare => {
                diesel::update(
                    content::table
                        .filter(content::h_app_id.eq(&ctx.h_app_id))
                        .filter(content::id.eq(id)),
                )
                .set((
                    content::dht_anchor_hash.eq(dht_anchor_hash),
                    content::declared_head_action_hash.eq(dht_anchor_hash),
                    content::declared_head_at.eq(None::<i64>),
                    // The ELECTION is cleared for the same reason the ordering
                    // is: this commit crowns a NEW head, so any election stored
                    // here belongs to the declaration being replaced. Leaving it
                    // would let a superseded election out-rank the next real one
                    // in `canonical_move_verdict` — and this is exactly the
                    // channel (the deploy PATCH) whose NULL ordering created the
                    // live two-way-declared class, so it must stay honest about
                    // carrying no election rather than inherit a stale one.
                    content::canonical_declared_at.eq(None::<i64>),
                    content::canonical_earned.eq(None::<i32>),
                    content::updated_at.eq(sql::<Text>("CURRENT_TIMESTAMP")),
                ))
                .execute(conn)
                .map_err(|e| StorageError::Internal(format!("Update anchor failed: {}", e)))?;
            }
            HeadElection::PreserveExistingDeclaration => {
                diesel::update(
                    content::table
                        .filter(content::h_app_id.eq(&ctx.h_app_id))
                        .filter(content::id.eq(id)),
                )
                .set((
                    content::dht_anchor_hash.eq(dht_anchor_hash),
                    content::updated_at.eq(sql::<Text>("CURRENT_TIMESTAMP")),
                ))
                .execute(conn)
                .map_err(|e| StorageError::Internal(format!("Update anchor failed: {}", e)))?;
            }
        }

        apply_content_patch_fields(conn, ctx, id, &patch)?;
    } else {
        // Defensive insert path. Seeded content normally exists in SQL
        // before its first DHT projection; this branch handles content
        // authored on a peer that hadn't yet received the seed.
        let default_ct = default_content_type();
        let default_cf = default_content_format();
        let default_re = default_reach();
        let title = patch.title.as_deref().unwrap_or("");
        let content_type = patch.content_type.as_deref().unwrap_or(&default_ct);
        let content_format = patch.content_format.as_deref().unwrap_or(&default_cf);
        let reach = patch.reach.as_deref().unwrap_or(&default_re);
        let new_content = NewContent {
            id,
            h_app_id: &ctx.h_app_id,
            title,
            description: patch.description.as_deref(),
            content_type,
            content_format,
            blob_hash: patch.blob_cid.as_deref(),
            blob_cid: patch.blob_cid.as_deref(),
            content_size_bytes: patch.content_size_bytes,
            metadata_json: patch.metadata_json.as_deref(),
            reach,
            created_by: None,
            content_body: None,
            dht_anchor_hash: None,
            // SSR server bundle hash is deploy-PATCH populated, not set on this
            // defensive DHT-projection insert path.
            server_blob_hash: None,
        };

        diesel::insert_into(content::table)
            .values(&new_content)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

        // Set the anchor on the just-inserted row, and the declared HEAD only
        // when the caller's election says so. A freshly-inserted row has no
        // declaration to preserve, but `PreserveExistingDeclaration` still
        // leaves it UNDECLARED on purpose: an anchor this node authored during
        // a heal is not an election, and leaving the slot empty is what lets a
        // later canonical declaration win outright instead of being refused as
        // "row already declared".
        match election {
            HeadElection::Declare => {
                diesel::update(
                    content::table
                        .filter(content::h_app_id.eq(&ctx.h_app_id))
                        .filter(content::id.eq(id)),
                )
                .set((
                    content::dht_anchor_hash.eq(dht_anchor_hash),
                    content::declared_head_action_hash.eq(dht_anchor_hash),
                ))
                .execute(conn)
                .map_err(|e| {
                    StorageError::Internal(format!("Set anchor on insert failed: {}", e))
                })?;
            }
            HeadElection::PreserveExistingDeclaration => {
                diesel::update(
                    content::table
                        .filter(content::h_app_id.eq(&ctx.h_app_id))
                        .filter(content::id.eq(id)),
                )
                .set(content::dht_anchor_hash.eq(dht_anchor_hash))
                .execute(conn)
                .map_err(|e| {
                    StorageError::Internal(format!("Set anchor on insert failed: {}", e))
                })?;
            }
        }
    }

    Ok(())
}

/// One row of the cross-peer content anchor inventory (see
/// [`list_content_anchor_inventory`]). Named rather than a 4-tuple because the
/// anchor and the declaration are semantically different claims that read
/// identically as `String`s — a positional swap between them would be a silent
/// authority bug, not a compile error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentAnchorInventoryRow {
    pub id: String,
    /// "the action my projection last saw" — never empty here (the query filters
    /// `IS NOT NULL`), though `""` is admitted as a legal non-divergent value.
    pub dht_anchor_hash: String,
    /// "the action a canonical channel elected", when this peer has one.
    /// `None` on a peer that has only ever anchored, never declared.
    pub declared_head_action_hash: Option<String>,
    /// Ordering the declaration carried. NOT comparable across peers — see
    /// `ProjectionInventoryEntry::declared_head_at`.
    pub declared_head_at: Option<i64>,
}

/// Read the notary-declared HEAD this row currently carries, if any.
///
/// `Ok(None)` covers BOTH "no such row" and "row exists but is undeclared" —
/// the adopt-before-author decision treats them identically (nothing local to
/// preserve), so collapsing them keeps the caller's rule a pure three-input
/// function instead of a four-state one.
pub fn declared_head_for(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<String>, StorageError> {
    let found: Option<Option<String>> = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(id))
        .select(content::declared_head_action_hash)
        .first::<Option<String>>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("declared head lookup failed: {e}")))?;
    // An empty-string declaration is a corrupt write, not a declaration.
    Ok(found.flatten().filter(|h| !h.trim().is_empty()))
}

/// Read the content row's CURRENT `reach` tier, if the row exists.
///
/// Used by the projection-reconcile heal path's non-narrowing reach guard
/// (RC-4, `p2p::projection_reconcile::reach_patch_would_narrow`): heal must
/// know the row's LOCAL reach before deciding whether an incoming conductor
/// answer's reach would narrow it. `Ok(None)` means no such row (heal is
/// existing-row-only anyway, so this reads as "nothing to protect").
pub fn reach_for(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<String>, StorageError> {
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(id))
        .select(content::reach)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("reach lookup failed: {e}")))
}

/// [`declared_head_for`] plus WHETHER A DHT ELECTION STANDS BEHIND IT.
///
/// The second element is `content.canonical_declared_at IS NOT NULL`. The two
/// are read together because the adopt-before-author rule needs both to tell a
/// CONTESTABLE stalemate (this row declared on its own authority; no election
/// has run) from a SETTLED one (an election ran and this row is obeying it).
/// Splitting them into two queries would let the pair go inconsistent under a
/// concurrent stamp — the row could gain an election between the reads and be
/// contested anyway, re-minting a link against an already-settled question.
pub fn declared_head_with_election(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<(Option<String>, bool), StorageError> {
    let found: Option<(Option<String>, Option<i64>)> = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(id))
        .select((
            content::declared_head_action_hash,
            content::canonical_declared_at,
        ))
        .first::<(Option<String>, Option<i64>)>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("declared head lookup failed: {e}")))?;
    match found {
        None => Ok((None, false)),
        Some((declared, election)) => Ok((
            // An empty-string declaration is a corrupt write, not a declaration
            // (same filter as `declared_head_for`).
            declared.filter(|h| !h.trim().is_empty()),
            election.is_some(),
        )),
    }
}

/// Batch form of [`declared_head_for`]: the declared HEAD for every id in `ids`
/// that carries one, as `id → head`.
///
/// Ids with no row, a NULL declaration, or an empty-string declaration are
/// ABSENT from the map — the same "nothing local to preserve" collapse
/// [`declared_head_for`] makes, so a caller can read presence-in-map as "a
/// canonical channel owns this row's head".
///
/// Exists so the reconcile discovery leg can classify a whole sweep's divergence
/// set with ONE query instead of a per-id round trip (the divergent set is
/// thousands of rows on a seeded peer). Chunked under SQLite's bound-variable
/// limit exactly as [`content_ids_present`] is.
pub fn declared_heads_for(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    ids: &[String],
) -> Result<std::collections::HashMap<String, String>, StorageError> {
    const CHUNK: usize = 500;
    let mut out = std::collections::HashMap::new();
    for chunk in ids.chunks(CHUNK) {
        let rows: Vec<(String, Option<String>)> = content::table
            .filter(content::h_app_id.eq(&ctx.h_app_id))
            .filter(content::id.eq_any(chunk))
            .select((content::id, content::declared_head_action_hash))
            .load(conn)
            .map_err(|e| StorageError::Internal(format!("declared head batch lookup: {e}")))?;
        for (id, declared) in rows {
            if let Some(head) = declared.filter(|h| !h.trim().is_empty()) {
                out.insert(id, head);
            }
        }
    }
    Ok(out)
}

/// Stamp the notary-declared HEAD onto an EXISTING content row — the reconcile
/// leg's conductor-VERIFIED stamp entrypoint (HEAD-election projection, Plan C3).
///
/// Behaviour:
/// - EXISTING-row only: returns `Ok(false)` when no row exists for `id` (NO insert
///   — the reconcile leg never fabricates content; a missing row means there is
///   nothing to stamp).
/// - On an existing row: sets BOTH `declared_head_action_hash` AND `dht_anchor_hash`
///   to `head_action_hash`. A verified stamp is a green write — it advances the
///   notary anchor, so green-overwrites-amber precedence holds for the value fields
///   supplied in `patch` (same per-field semantics as [`upsert_with_anchor`], via
///   the shared [`apply_content_patch_fields`] — `blob_cid` mirrors to `blob_hash`).
/// - Bumps `updated_at`.
/// - Idempotent: re-stamping the same head with a no-change patch is a cheap
///   UPDATE that still returns `Ok(true)` (no no-op detection needed).
///
/// This is a conductor-VERIFIED path (the caller resolves the head via the
/// conductor before stamping) — it MUST NEVER be called from CRDT/gossip input,
/// which would launder un-witnessed peer state into the notary-authority HEAD.
pub fn stamp_declared_head(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    head_action_hash: &str,
    declared_at: Option<i64>,
    patch: Option<ContentProjectionPatch>,
) -> Result<bool, StorageError> {
    stamp_declared_head_mode(
        conn,
        ctx,
        id,
        head_action_hash,
        declared_at,
        patch,
        StampMode::Declare,
        // A deliberate declare channel has written a canonical link but has NOT
        // read back its notarized timestamp, so it carries no election to
        // record. The row's ordering is NULLed on a move (see the election
        // bookkeeping in `stamp_declared_head_mode`) and backfills on the next
        // canonical heal resolve — which answers with the election this declare
        // just created.
        None,
    )
    // `true` means "there was a row and we wrote it" — BOTH a converging stamp and
    // a same-head refresh qualify (this bool predates the Stamped/Refreshed split
    // and its callers only ask "was there a row to stamp?"). Callers that need to
    // know whether the HEAD actually MOVED must use `stamp_declared_head_mode`.
    .map(|o| matches!(o, StampOutcome::Stamped | StampOutcome::Refreshed))
}

/// How a declared-head stamp may interact with an already-declared row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampMode {
    /// Deliberate canonical-channel stamp (declare route, canonical-head
    /// propagation, own-conductor `ContentHeadDeclared` signal): always
    /// writes. These channels are request-borne or own-authored — moving a
    /// declared head (including deliberately BACKWARDS: revert is a
    /// legitimate authority act on a version DAG) is exactly their job.
    Declare,
    /// Heal stamp for a CANONICAL conductor answer (projection-reconcile with
    /// `ContentHeadWire.canonical == true`): fills an undeclared row,
    /// refreshes the same head, and may MOVE a declared row only when the
    /// answer's `declared_at` is provably NEWER than the stored ordering. A
    /// conductor that has not yet integrated a newer cross-root canonical
    /// link answers with the OLD canonical record — canonical, yet stale —
    /// and an unconditional Declare-mode stamp moved the head BACKWARDS
    /// (live regression 2026-07-12: elohim-host-landing converged at edge
    /// #1187's seam-smoke, healed back to the superseded head by #1188).
    /// Unknown ordering (either side NULL) refuses the move: heal converges
    /// rows forward; deliberate channels do everything else.
    HealCanonical,
    /// Heal stamp for a FALLBACK conductor answer (root-author election on a
    /// cold conductor while the canonical link is not yet retrievable): fills
    /// an UNDECLARED row only. In Declare semantics this RESURRECTED a
    /// superseded head over a previously-adopted canonical one (live
    /// regression 2026-07-11 20:42:40, two minutes after the first
    /// cross-conductor adoption). Heal fills absence; only canonical channels
    /// move a declared head.
    GapFill,
}

/// Outcome of [`stamp_declared_head_mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampOutcome {
    /// Row updated AND the declared HEAD actually MOVED — an undeclared row was
    /// filled, or a declared row was advanced to a different action. This is the
    /// only outcome that represents CONVERGENCE.
    Stamped,
    /// Row updated but the declared HEAD did NOT move: the row already carried
    /// this exact action. The write still did real work (value-field patch
    /// refresh, and — load-bearing — recording `declared_head_at` on a row whose
    /// ordering was NULL), but nothing converged.
    ///
    /// Distinguished from [`StampOutcome::Stamped`] because conflating them made
    /// the heal plane illegible: a peer whose own conductor answers with the head
    /// the row ALREADY holds re-stamps it every sweep and reported each one as
    /// `HEALED` (live 2026-07-26: matthew logged 1578 `HEALED content anchor`
    /// lines in 12h while `elohim-host-landing`'s head never moved —
    /// `updated_at` advanced 05:33:54 → 05:49:00 on an unchanged
    /// `uhCkkh_Gb…`). Cross-peer divergence cannot be healed from the OWN
    /// conductor when the two peers hold genuinely different roots; counting the
    /// no-op as a cure hid that permanently.
    Refreshed,
    /// GapFill mode: row already carries a DIFFERENT declared head — left
    /// untouched (canonical channels own moving it).
    SkippedDeclared,
    /// HealCanonical mode: row already carries a DIFFERENT declared head and
    /// the incoming answer is not provably newer (stale canonical record, or
    /// ordering unknown on either side) — left untouched. The deliberate
    /// canonical channels own any non-forward move.
    SkippedStale,
    /// No local row for this id (stamp is existing-row-only by design).
    NoRow,
}

/// The DHT election standing behind a canonical head answer: the winning
/// declaration LINK's notarized timestamp, and whether it was EARNED.
///
/// This is what `content_store::select_canonical_winner` arbitrated on, carried
/// verbatim to the projection. `None` = the answer carries no election.
pub type CanonicalOrdering = (i64, bool);

/// Why a [`StampMode::HealCanonical`] stamp declined to move an already-declared
/// row. Label values for `elohim_projection_heal_refused_stale_total{reason}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleReason {
    /// The INCOMING answer carries no election, and the stored row does. An
    /// un-elected answer never displaces an elected declaration.
    StoredNull,
    /// Both sides carry an election and the incoming one is not strictly newer.
    /// The converged steady state.
    NotNewer,
    /// Incoming is STAGING; the row holds an EARNED declaration. Earned is never
    /// displaced by the scaffold tier.
    Tier,
}

impl StaleReason {
    pub fn label(self) -> &'static str {
        match self {
            StaleReason::StoredNull => "stored_null",
            StaleReason::NotNewer => "not_newer",
            StaleReason::Tier => "tier",
        }
    }
}

/// May a CANONICAL heal answer MOVE an already-declared row? Pure and total, so
/// the rule that decides cross-peer head convergence is readable and testable
/// without a conductor, a pool, or a peer.
///
/// ## This obeys the DHT; it does not invent an ordering
///
/// The substrate trust contract says canonical channels alone move declared
/// heads, and heal FILLS or converges FORWARD — never backwards. Both hold here,
/// because this function only ever runs on an answer that already proved
/// `canonical == true`, i.e. one that came through
/// `select_canonical_winner`. What it replays is that selector's OWN ordering:
/// tier first, then the notarized link clock. It is emphatically NOT
/// newest-wins on a wall clock — the rejected design that made heads flap.
///
/// Three tiers, in order:
///
/// 1. **TIER.** An EARNED election beats an unearned/absent one outright, and a
///    STAGING answer never displaces an EARNED row. Mirrors
///    `select_canonical_winner` rule 1 exactly, so a gossip-lagged peer cannot
///    launder a staging declaration over an earned head via the heal path.
/// 2. **ELECTION PRESENCE.** An answer carrying an election beats a row carrying
///    NONE. This is the tier that unsticks the live two-way-declared class: those
///    rows were declared by channels that record no election at all (the deploy
///    PATCH's `HeadElection::Declare`, the timestamp-less `ContentHeadDeclared`
///    signal), so under the old `provably_newer` test — which refused on NULL
///    either side — a genuinely DHT-elected head could never be adopted. Moving
///    here is forward in AUTHORITY: notarized election over un-elected
///    self-declaration. It is the invariant's "key it on the authority the write
///    CARRIES", applied literally.
/// 3. **CLOCK.** Both sides elected ⇒ strictly-newer link timestamp wins; equal
///    or older refuses. This is what keeps a stale-canonical conductor (one that
///    has not yet integrated a newer link, answering with the OLD canonical
///    record) from moving the head BACKWARDS — the 2026-07-12 regression. Note
///    the comparison is now between two ELECTION clocks, which are the same
///    notarized values on every peer, so it is globally consistent in a way the
///    old head-action-timestamp comparison never was.
///
/// Returns `Ok(())` to move, `Err(reason)` to keep the adopted head.
pub fn canonical_move_verdict(
    incoming: Option<CanonicalOrdering>,
    stored: Option<CanonicalOrdering>,
) -> Result<(), StaleReason> {
    match (incoming, stored) {
        // (1) TIER — checked before anything else, both directions.
        (Some((_, true)), Some((_, false))) => Ok(()),
        (Some((_, false)), Some((_, true))) => Err(StaleReason::Tier),
        // (2) ELECTION PRESENCE.
        (Some(_), None) => Ok(()),
        (None, Some(_)) => Err(StaleReason::StoredNull),
        // (3) CLOCK — same tier on both sides.
        (Some((inc, _)), Some((sto, _))) => {
            if inc > sto {
                Ok(())
            } else {
                Err(StaleReason::NotNewer)
            }
        }
        // Neither side carries an election: nothing authorizes a move. This is
        // the pre-cure world, preserved exactly — the old guard refused on NULL
        // either side, and with no election anywhere there is still nothing to
        // order by. An EARNED answer with no clock cannot occur (the zome sets
        // both fields together or neither).
        (None, None) => Err(StaleReason::StoredNull),
    }
}

/// `canonical_ordering` is the DHT election behind a CANONICAL answer (see
/// [`CanonicalOrdering`]). `None` means the caller's answer carries no election:
/// every non-canonical channel, and the declare paths (which have written a link
/// but not read back its notarized timestamp). It is consulted ONLY by
/// [`StampMode::HealCanonical`]; the other modes ignore it for the move decision
/// but still PERSIST it when present, so the column backfills.
///
/// Eight parameters, one over the clippy threshold. Deliberately NOT bundled
/// into a params struct: every call site names these arguments explicitly, and
/// the two that decide authority — `mode` and `canonical_ordering` — are exactly
/// the ones a reader must be able to see at the call site without following a
/// struct literal somewhere else. Hiding them behind a builder is how a heal
/// path quietly acquires `Declare` semantics. Same `#[allow]` the crate already
/// uses for `rea_commitment_service`, `economic_events`, and `api::mod`.
#[allow(clippy::too_many_arguments)]
pub fn stamp_declared_head_mode(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    head_action_hash: &str,
    declared_at: Option<i64>,
    patch: Option<ContentProjectionPatch>,
    mode: StampMode,
    canonical_ordering: Option<CanonicalOrdering>,
) -> Result<StampOutcome, StorageError> {
    use diesel::dsl::sql;
    use diesel::sql_types::Text;

    #[allow(clippy::type_complexity)]
    let existing: Option<(Option<String>, Option<i64>, Option<i64>, Option<i32>)> = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(id))
        .select((
            content::declared_head_action_hash,
            content::declared_head_at,
            content::canonical_declared_at,
            content::canonical_earned,
        ))
        .first::<(Option<String>, Option<i64>, Option<i64>, Option<i32>)>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Stamp declared head lookup: {e}")))?;

    let (declared, _stored_declared_at, stored_canonical_at, stored_canonical_earned) =
        match existing {
            None => return Ok(StampOutcome::NoRow),
            Some(row) => row,
        };

    // The stored election, reassembled. Both columns are written together, so a
    // half-populated pair cannot occur; a defensive `zip` treats one as none.
    let stored_ordering: Option<CanonicalOrdering> =
        stored_canonical_at.map(|ts| (ts, stored_canonical_earned.unwrap_or(0) != 0));

    let moving_declared_row = matches!(
        declared.as_deref(),
        Some(current) if !current.is_empty() && current != head_action_hash
    );

    // The row already carries EXACTLY this head — the write below is a refresh
    // (patch fields + ordering backfill), not a convergence. Captured BEFORE the
    // update so the outcome can tell the caller which of the two happened.
    let same_declared_head = matches!(
        declared.as_deref(),
        Some(current) if current == head_action_hash
    );

    match mode {
        StampMode::Declare => {}
        StampMode::GapFill => {
            if moving_declared_row {
                return Ok(StampOutcome::SkippedDeclared);
            }
        }
        StampMode::HealCanonical => {
            if moving_declared_row {
                // Monotonic move, arbitrated by the DHT's OWN election — tier,
                // then election presence, then the notarized link clock. See
                // `canonical_move_verdict`: this replays `select_canonical_winner`
                // rather than inventing an ordering, and it never widens
                // `Declare`. A refusal is counted by reason so the two very
                // different stuck states (no election anywhere vs. lost the
                // clock comparison) stop being indistinguishable.
                if let Err(reason) = canonical_move_verdict(canonical_ordering, stored_ordering) {
                    crate::metrics::inc_projection_refused_stale(reason.label());
                    return Ok(StampOutcome::SkippedStale);
                }
            }
        }
    }

    diesel::update(
        content::table
            .filter(content::h_app_id.eq(&ctx.h_app_id))
            .filter(content::id.eq(id)),
    )
    .set((
        content::declared_head_action_hash.eq(head_action_hash),
        content::dht_anchor_hash.eq(head_action_hash),
        content::updated_at.eq(sql::<Text>("CURRENT_TIMESTAMP")),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Stamp declared head failed: {}", e)))?;

    // Ordering bookkeeping: a stamp that carries the declaration's zome
    // Timestamp records it; a stamp that MOVES the head without one must NULL
    // the stored ordering (it belongs to the prior declaration — keeping it
    // would misorder every later monotonicity check). Same-head stamps
    // without a timestamp keep whatever ordering is already known.
    if let Some(incoming) = declared_at {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set(content::declared_head_at.eq(Some(incoming)))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Stamp declared_head_at failed: {e}")))?;
    } else if moving_declared_row {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set(content::declared_head_at.eq(None::<i64>))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Clear declared_head_at failed: {e}")))?;
    }

    // ELECTION bookkeeping — the same discipline as `declared_head_at` above,
    // applied to the ordering that actually governs canonical moves. A stamp
    // that CARRIES an election records it (so the row can later be compared
    // clock-to-clock); a stamp that MOVES the head while carrying none must NULL
    // it, because the stored election belongs to the declaration being replaced
    // and keeping it would let a superseded election veto the next real one.
    // Same-head stamps carrying nothing keep whatever is known (a refresh must
    // not erase an election).
    if let Some((ts, earned)) = canonical_ordering {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set((
            content::canonical_declared_at.eq(Some(ts)),
            content::canonical_earned.eq(Some(i32::from(earned))),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Stamp canonical ordering failed: {e}")))?;
    } else if moving_declared_row {
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set((
            content::canonical_declared_at.eq(None::<i64>),
            content::canonical_earned.eq(None::<i32>),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Clear canonical ordering failed: {e}")))?;
    }

    if let Some(patch) = patch {
        apply_content_patch_fields(conn, ctx, id, &patch)?;
    }

    if same_declared_head {
        Ok(StampOutcome::Refreshed)
    } else {
        Ok(StampOutcome::Stamped)
    }
}

// ============================================================================
// Stats
// ============================================================================

/// Which of `ids` exist in the local content projection (acquisition presence diff).
///
/// Returns a HashSet of ids that are already present locally. Used by
/// `run_acquisition_reconcile` to diff the pin's desired set against what the
/// node already holds, so only genuine gaps are enqueued for fetch.
///
/// NOTE: callers must keep ids.len() under SQLITE_MAX_VARIABLE_NUMBER (~999 on
/// older SQLite); chunk the query if cluster-pin closures bring large id sets.
pub fn content_ids_present(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    ids: &[String],
) -> QueryResult<std::collections::HashSet<String>> {
    if ids.is_empty() {
        return Ok(Default::default());
    }
    let found: Vec<String> = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq_any(ids))
        .select(content::id)
        .load(conn)?;
    Ok(found.into_iter().collect())
}

/// Which of `ids` exist locally AND carry the given `reach` (provide-loop gate).
///
/// The provide reconciler must only offer to the commons what is GENUINELY
/// commons-reach: a non-commons locally-present item must NOT mint a spurious
/// `replicates-commons` commitment. `content_ids_present` answers "do I hold
/// it?"; this answers "do I hold it AND is it commons?" — the desired-set gate.
///
/// NOTE: same SQLITE_MAX_VARIABLE_NUMBER caveat as [`content_ids_present`] —
/// chunk large id sets.
pub fn content_ids_with_reach(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    ids: &[String],
    reach: &str,
) -> QueryResult<std::collections::HashSet<String>> {
    if ids.is_empty() {
        return Ok(Default::default());
    }
    let found: Vec<String> = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq_any(ids))
        .filter(content::reach.eq(reach))
        .select(content::id)
        .load(conn)?;
    Ok(found.into_iter().collect())
}

/// For each of `ids` that exists locally, return `(id, reach)` — the content's
/// own declared reach.
///
/// The reach-general provide-eligibility path needs per-content reach (NOT a
/// single fixed reach filter): a node may provide a household-reach record
/// only if it has embodied responsibility for that scope, and commons is the
/// only openly-providable reach. The caller builds the topic
/// `elohim/<pillar>/<reach>[/<collective>]` from each pair and runs
/// `classify_pre_authorization`. The pillar is supplied caller-side (there is
/// no `pillar` column on `content`); Stage-1 classification ignores everything
/// past the reach segment, so caller-side pillar derivation is sufficient and
/// the seam tightens with the graph walk in Stage 2/3.
///
/// NOTE: same SQLITE_MAX_VARIABLE_NUMBER caveat as [`content_ids_present`] —
/// chunk large id sets.
pub fn content_reaches_for_ids(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    ids: &[String],
) -> QueryResult<Vec<(String, String)>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq_any(ids))
        .select((content::id, content::reach))
        .load(conn)
}

/// `(id, reach, content_type)` for the subset of `ids` that are locally
/// present AND carry a NON-NULL `dht_anchor_hash` — the GHOST-ANCHOR
/// candidate query.
///
/// A row in this set claims notarized provenance in SQL. When this node's OWN
/// conductor cannot resolve the same id (`resolve_content_head` → `Ok(None)`),
/// the claim is a ghost: the anchor string outlived the conductor incarnation
/// that authored it (a DHT reset / reinstall / re-key wipes conductor state
/// while the SQLite projection persists on its PVC). The row then sits in a
/// blind spot: `reanchor_backfill`'s witness sweep only selects
/// `dht_anchor_hash IS NULL`, so an anchored-but-unwitnessed row is never a
/// candidate and can never green — while every canonical-head declaration
/// against it is refused by the zome (`no content found for id`, because
/// `gather_content_chain`'s `IdToContent` link does not exist locally either).
///
/// `content_type` rides along (widened alongside `reach`) so the caller can
/// apply the same non-canonical-vocabulary skip guard content_type gets in
/// [`list_unanchored_content_ids`] — a row whose `content_type` the conductor's
/// `validate()` would reject is not re-authorable either.
///
/// Deliberately a FOCUSED query, not a filter bolted onto
/// [`content_reaches_for_ids`]: the anchored predicate is the whole point of
/// this candidate class, and conflating the two would let a NULL-anchor row
/// (which the witness sweep already owns) enter the ghost path twice.
///
/// NOTE: same SQLITE_MAX_VARIABLE_NUMBER caveat as [`content_ids_present`] —
/// chunk large id sets.
pub fn anchored_content_reaches_for_ids(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    ids: &[String],
) -> QueryResult<Vec<(String, String, String)>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq_any(ids))
        .filter(content::dht_anchor_hash.is_not_null())
        .select((content::id, content::reach, content::content_type))
        .load(conn)
}

/// Reach tiers admitted into a CROSS-PEER content inventory — the broadcast
/// family only.
///
/// This is the SAME rule as `crate::sync::projector::reach_is_distribution_safe`
/// (`community` | `public` | `commons`): the content-anchor inventory advertised
/// over `/elohim/view-federation/1.0.0` is a cross-peer surface, so a scoped-tier
/// row (`private`/`self`/`intimate`/`trusted`/`familiar`) — or any UNKNOWN reach
/// value — must NEVER leak into it. The three literals are duplicated here rather
/// than imported from `sync::projector` deliberately: `db::content_diesel` is a
/// leaf persistence module and taking a dependency UP on the `sync` layer would
/// invert the module graph (a module-layer smell). The rule's canonical home is
/// `reach_is_distribution_safe`; this co-located mirror is guarded by the
/// `content_anchor_inventory_excludes_scoped_reach` test below.
const DISTRIBUTION_SAFE_REACH: [&str; 3] = ["community", "public", "commons"];

/// True when `reach` may cross a peer boundary.
///
/// The Rust-side twin of the `reach.eq_any(DISTRIBUTION_SAFE_REACH)` SQL filter
/// in [`list_content_anchor_inventory`], for cross-peer surfaces that resolve a
/// row by id rather than by query — currently the `ContentHeadRecord`
/// view-federation responder, which serves an action hash plus full serialized
/// DHT `Record` bytes and so carries exactly the same obligation.
///
/// Deliberately reads the SAME constant rather than restating the tiers: two
/// copies of a reach whitelist is precisely how a scoped tier eventually leaks
/// out of one of them.
pub fn is_distribution_safe_reach(reach: &str) -> bool {
    DISTRIBUTION_SAFE_REACH.contains(&reach)
}

/// `(id, dht_anchor_hash)` pairs for the CROSS-PEER content-anchor inventory
/// (notary-authority Leg 4 — the cross-peer reconcile arm).
///
/// Returns only rows that are BOTH:
/// - **anchored** (`dht_anchor_hash IS NOT NULL`) — an un-anchored bulk-seed row
///   carries no notary provenance to advertise; and
/// - **distribution-safe reach** (`community`/`public`/`commons`, per
///   [`DISTRIBUTION_SAFE_REACH`]) — scoped tiers must never enter a cross-peer
///   surface. This reach filter is a REQUIREMENT, not an optimization.
///
/// Ordered by `updated_at` DESCENDING, with `id` ascending as a deterministic
/// tiebreak; capped at `cap` (LIMIT). The ordering makes truncation — whether by
/// this `cap`, by the projection-inventory entry cap, or by the responder's
/// byte-budget trim — surface the **reconcile hot set first**: the
/// most-recently-updated anchors are the ones a peer is most likely still
/// missing. Precisely: a stamped (updated) row is exactly the row that just
/// LEFT the requester's gap set after a prior heal, so the advertised tail is
/// only ever needed for rows the requester still lacks — the stable tail
/// converges over later sweeps once its rows stop being gaps. (`updated_at` is
/// bumped by every anchor/HEAD stamp — see `upsert_with_anchor` /
/// `stamp_declared_head` — so "recently updated" tracks "recently (re)anchored".)
///
/// **Discovery-only (the notary invariant).** A peer consuming this inventory
/// learns WHICH content ids have a DHT anchor somewhere; the anchor VALUE it
/// writes into its own projection comes EXCLUSIVELY from its OWN conductor
/// (`content_store::resolve_content_head`), never from the advertised pair. Peer
/// bytes are never laundered into notary provenance — the same P1 discipline as
/// `rea_commitments::inventory_for_reconcile`. The `declared_head_*` columns
/// this now also carries are subject to the SAME rule with no exception: they
/// are a HINT that a peer HAS a declaration, which triggers a verified
/// conductor-side declare — never a value stamped into a row.
pub fn list_content_anchor_inventory(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    offset: i64,
    cap: i64,
) -> Result<(Vec<ContentAnchorInventoryRow>, i64), StorageError> {
    // Honest total: the TRUE count of anchored + distribution-safe rows (the same
    // predicates the page query below filters on), NOT the served page length.
    // A responder that reports served-count as total makes truncation invisible on
    // the wire — the requester cannot tell a full window from the whole corpus, so
    // the cold tail past the cap stays permanently undiscovered.
    let total: i64 = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::dht_anchor_hash.is_not_null())
        .filter(content::reach.eq_any(DISTRIBUTION_SAFE_REACH))
        .count()
        .get_result(conn)
        .map_err(|e| {
            StorageError::Internal(format!("content anchor inventory count failed: {e}"))
        })?;

    type InventoryTuple = (String, Option<String>, Option<String>, Option<i64>);
    let rows: Vec<InventoryTuple> = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::dht_anchor_hash.is_not_null())
        .filter(content::reach.eq_any(DISTRIBUTION_SAFE_REACH))
        .order((content::updated_at.desc(), content::id.asc()))
        .offset(offset.max(0))
        .limit(cap)
        .select((
            content::id,
            content::dht_anchor_hash,
            content::declared_head_action_hash,
            content::declared_head_at,
        ))
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("content anchor inventory load failed: {e}"))
        })?;

    // `IS NOT NULL` guarantees `Some`; `filter_map` discards defensively rather
    // than unwrap. An empty-string anchor (`""`, distinct from NULL) is admitted
    // as-is — the consumer's diff treats an empty peer anchor as non-divergence.
    let entries = rows
        .into_iter()
        .filter_map(|(id, anchor, declared_head, declared_at)| {
            anchor.map(|dht_anchor_hash| ContentAnchorInventoryRow {
                id,
                dht_anchor_hash,
                // An empty-string declaration is a corrupt write, not a
                // declaration — never advertise one as a hint.
                declared_head_action_hash: declared_head.filter(|h| !h.trim().is_empty()),
                declared_head_at: declared_at,
            })
        })
        .collect();
    Ok((entries, total))
}

/// The projection facts ONE content id contributes to a trust-pricing decision
/// (Q7, `trust::adoption_pricing`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHeadPricingRow {
    pub id: String,
    /// Raw reach string — parsed by
    /// `services::floor_protections::content_head_floor_class`, never here. An
    /// unparseable value must reach the classifier so it can decline to price
    /// the id; swallowing it in this layer would fail OPEN.
    pub reach: String,
    /// The row's current declaration. `None` (undeclared) is the ONLY state in
    /// which a peer-advertised head can be adopted directly — see
    /// `head_adoption::decide_head_action`'s `AdoptPeer` corner.
    pub declared_head_action_hash: Option<String>,
}

/// Batch-read the pricing inputs for a sweep's adopt slice — ONE local SQL
/// read for the whole slice, ZERO conductor or peer round-trips.
///
/// This is deliberately a batch, not a per-id helper: the pricing decision
/// exists to REMOVE round-trips, so buying one query per candidate to decide
/// whether to skip one RPC per candidate would be self-defeating.
///
/// Ids absent from the result (no row for this `h_app_id`) are absent from the
/// returned vec — the caller must treat an absent id as UNPRICEABLE (full
/// verification), never as "no floor applies."
///
/// Chunked at 500 bound parameters, comfortably under SQLite's 999 default
/// `SQLITE_MAX_VARIABLE_NUMBER`; the caller's slice is capped at
/// `WITNESS_MAX_PER_TICK` (200) today, so this is defensive rather than load-
/// bearing — but a future cap raise must not turn into a runtime bind error.
pub fn content_head_pricing_inputs(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    ids: &[String],
) -> Result<Vec<ContentHeadPricingRow>, StorageError> {
    const CHUNK: usize = 500;
    let mut out = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(CHUNK) {
        let rows: Vec<(String, String, Option<String>)> = content::table
            .filter(content::h_app_id.eq(&ctx.h_app_id))
            .filter(content::id.eq_any(chunk))
            .select((
                content::id,
                content::reach,
                content::declared_head_action_hash,
            ))
            .load(conn)
            .map_err(|e| {
                StorageError::Internal(format!("content head pricing inputs load failed: {e}"))
            })?;
        out.extend(rows.into_iter().map(|(id, reach, declared)| {
            ContentHeadPricingRow {
                id,
                reach,
                // An empty-string declaration is a corrupt write, not a
                // declaration — the same normalization
                // `list_content_anchor_inventory` applies to the advertised
                // hint, so the two sides of the AdoptPeer corner agree.
                declared_head_action_hash: declared.filter(|h| !h.trim().is_empty()),
            }
        }));
    }
    Ok(out)
}

/// Head-plane corpus digest (T4, head-plane trust-gradient program plan §3
/// L2): a stable fingerprint of every anchored, distribution-safe content
/// row's `(id, dhtAnchorHash)` pair — the SAME relation
/// [`list_content_anchor_inventory`] serves, unpaginated and hashed instead of
/// paged.
///
/// Reuses [`DISTRIBUTION_SAFE_REACH`] — never restates the reach tiers; a
/// second copy is exactly how a scoped tier eventually leaks into a
/// cross-peer digest. Hashed via the SAME fold
/// [`crate::p2p::reconcile_rails::digest_of_entry_lines`] the sync-round
/// corpus digest (`p2p::sync_round::corpus_digest`) uses, so two independent
/// corpora (the DocStore-backed sync round vs. this SQL content table) share
/// one sort+hash implementation rather than two that could silently drift
/// apart.
///
/// **Derived on demand, never stored.** Sub-ms over ~3.5k rows (the 2026-08-08
/// evidence pass), so there is no dirty-flag or cache to invalidate — a
/// staleness bug in a cache would be strictly worse than the query cost it
/// would save. If profiling ever proves otherwise, the cache key MUST be
/// `(count(*), max(updated_at))` over this same relation, never a dirty flag.
///
/// This is the L2 join key: a peer whose OWN `head_corpus_digest` equals the
/// digest another peer advertises (on `ViewFederationRequest.head_corpus_digest`,
/// or inside a signed `HeadSetSnapshot`) is already in sync with that peer's
/// head plane at zero verification cost — see `trust::snapshot::HeadSetSnapshot`.
fn load_head_corpus_rows(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<(String, Option<String>)>, StorageError> {
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::reach.eq_any(DISTRIBUTION_SAFE_REACH))
        .select((content::id, content::dht_anchor_hash))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("head corpus digest load failed: {e}")))
}

/// Whether the distribution-safe head corpus is stable enough to advertise as
/// a cross-peer equality shortcut.
///
/// `Amber` is derived from the presence of distribution-safe rows that the DHT
/// has not witnessed yet (`dht_anchor_hash IS NULL`). During a bulk-seed witness
/// sweep that set shrinks on every tick, so hashing only the anchored subset
/// would produce a sequence of technically-correct but predictably mismatching
/// digests. Callers must abstain until the relation is `Ready`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadCorpusDigestReadiness {
    /// At least one cross-peer-visible row is still awaiting DHT witness.
    Amber { pending: usize },
    /// Every cross-peer-visible row is witnessed; the digest names the whole
    /// stable relation at this database snapshot.
    Ready { digest: String, total: usize },
}

/// Read digest eligibility and, when eligible, the digest itself from ONE SQL
/// snapshot. A count query followed by a digest query would leave a race where
/// a newly inserted amber row could arrive between the two and turn a partial
/// relation into an advertised "whole corpus" fingerprint.
pub fn head_corpus_digest_readiness(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<HeadCorpusDigestReadiness, StorageError> {
    let rows = load_head_corpus_rows(conn, ctx)?;
    let pending = rows.iter().filter(|(_, anchor)| anchor.is_none()).count();
    if pending > 0 {
        return Ok(HeadCorpusDigestReadiness::Amber { pending });
    }

    let total = rows.len();
    let entries = rows
        .into_iter()
        .filter_map(|(id, anchor)| anchor.map(|a| format!("{id}={a}")))
        .collect();
    Ok(HeadCorpusDigestReadiness::Ready {
        digest: crate::p2p::reconcile_rails::digest_of_entry_lines(entries),
        total,
    })
}

pub fn head_corpus_digest(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<String, StorageError> {
    let rows = load_head_corpus_rows(conn, ctx)?;

    // The raw digest remains useful for diagnostics and snapshot verification
    // during the amber window. Cross-peer requesters MUST use
    // `head_corpus_digest_readiness` and abstain while it reports Amber.
    let entries: Vec<String> = rows
        .into_iter()
        .filter_map(|(id, anchor)| anchor.map(|a| format!("{id}={a}")))
        .collect();
    Ok(crate::p2p::reconcile_rails::digest_of_entry_lines(entries))
}

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
/// `min_trust`: see [`list_content`]. External paginators MUST pass
/// `MinTrust::Amber` so totals stay consistent with the gated row set.
pub fn count_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ContentQuery,
    min_trust: MinTrust,
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

    // Trust gate: mirrors list_content. External paginators must pass
    // `MinTrust::Amber` so totals stay consistent with the filtered row set.
    base_query = apply_min_trust(base_query, min_trust);

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

/// Re-anchor backfill query: return IDs of content rows that were never
/// DHT-authored (`dht_anchor_hash IS NULL`). These are the rows a cold-conductor
/// seed left provenance-only: the seeder's reach-stamp PATCH routed through the
/// conductor while its cells were `CellDisabled`, so the entry never landed in
/// the DHT and reach was never notarized → no `content:<reach>` provide rows →
/// the resilience card stays dark.
///
/// The re-anchor backfill re-authors each of these via the conductor's
/// `create_content` (the same null-anchor path `update_via_conductor` uses),
/// which on `ContentCommitted` projection stamps `dht_anchor_hash`. Internal
/// use only — does NOT apply the provenance read gate (this query IS part of
/// the machinery that produces provenance). A non-positive limit returns empty.
///
/// `created_at` ascending so the oldest un-anchored rows heal first and the
/// ordering is stable across sweeps (restart-safe, idempotent: an id that
/// gained an anchor on a prior sweep is no longer a candidate).
/// Returns `(id, reach, content_type)` for each NULL-anchor row so the
/// re-anchor sweep can skip rows whose stored reach OR content_type is
/// non-canonical (the conductor's `Content::validate()` would reject either
/// on every sweep — see `generated_enums::ALL_CONTENT_TYPES` and the
/// `attestation:`/`governance-action:` prefix exception). Only the
/// reanchor_backfill service calls this.
pub fn list_unanchored_content_ids(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
) -> Result<Vec<(String, String, String)>, StorageError> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::dht_anchor_hash.is_null())
        .select((content::id, content::reach, content::content_type))
        .order((content::created_at.asc(), content::id.asc()))
        .limit(limit)
        .load::<(String, String, String)>(conn)
        .map_err(|e| StorageError::Internal(format!("list_unanchored_content_ids failed: {}", e)))
}

/// Ids of locally-present content rows that carry a DHT anchor — REACH-AGNOSTIC.
///
/// The deliberately UNFILTERED twin of [`list_content_anchor_inventory`]'s id
/// set, and LOCAL-ONLY: this never becomes a cross-peer surface (the advertised
/// inventory keeps its [`DISTRIBUTION_SAFE_REACH`] filter, which is a
/// requirement, not an optimization).
///
/// It exists so the cross-peer gap classifier can ask its two questions
/// SEPARATELY — "is this row anchored at all?" (this query) and "may it cross a
/// peer boundary?" (the reach filter). Collapsing them is what froze alpha:
/// probing presence reach-agnostically while probing anchoredness through the
/// distribution-safe map classifies every anchored-but-scoped row as an
/// un-anchored gap. That gap is unhealable by construction — heal stamps
/// anchors and never widens reach — so the population is permanent and
/// self-renewing. See `projection_reconcile::ContentGap::ReachScoped`.
pub fn anchored_content_ids_any_reach(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<std::collections::HashSet<String>, StorageError> {
    let ids: Vec<String> = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::dht_anchor_hash.is_not_null())
        .select(content::id)
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("anchored_content_ids_any_reach failed: {e}"))
        })?;
    Ok(ids.into_iter().collect())
}

/// Count content rows that were never DHT-authored (`dht_anchor_hash IS NULL`),
/// scoped by app context. The re-anchor backfill uses this to report the
/// remaining `pending` count on `/p2p/status` after a bounded sweep.
pub fn count_unanchored_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::dht_anchor_hash.is_null())
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("count_unanchored_content failed: {}", e)))
}

/// Drain-loop write: mark a content row as p2p_published at the current time.
/// Operates on the operational projection column only — does not touch
/// dht_anchor_hash or any notarized state. Idempotent — re-publishing an
/// already-published row just bumps the timestamp.
///
/// Note: the drain query (list_unpublished_content_ids) filters only on
/// p2p_published_at IS NULL, so this function also runs on content that
/// has a dht_anchor_hash set (Holochain-notarized content that was never
/// drained through the libp2p path). That is the intended behaviour —
/// Kademlia publish and Holochain notarization are independent provenance
/// markers.
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

/// Drain debug: count total, published rows scoped by app.
/// Reads only the operational projection (content table) — no DHT lookup.
/// Uses a single SQL query with FILTER (SQLite 3.30+) so the two counts
/// are atomic against a concurrent drain tick. Returns (total, published);
/// callers compute `pending = total - published` and clamp to i32 for wire.
pub fn count_publish_state(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<(i64, i64), StorageError> {
    use diesel::sql_types::{BigInt, Text};

    #[derive(diesel::QueryableByName)]
    struct PublishCounts {
        #[diesel(sql_type = BigInt)]
        total: i64,
        #[diesel(sql_type = BigInt)]
        published: i64,
    }

    let row: PublishCounts = diesel::sql_query(
        "SELECT COUNT(*) AS total, \
         COUNT(*) FILTER (WHERE p2p_published_at IS NOT NULL) AS published \
         FROM content WHERE h_app_id = ?",
    )
    .bind::<Text, _>(&ctx.h_app_id)
    .get_result(conn)
    .map_err(|e| StorageError::Internal(format!("count_publish_state failed: {}", e)))?;

    Ok((row.total, row.published))
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
                server_blob_hash TEXT,
                blob_cid TEXT,
                content_size_bytes INTEGER,
                metadata_json TEXT,
                reach TEXT NOT NULL DEFAULT 'public',
                validation_status TEXT NOT NULL DEFAULT 'valid',
                created_by TEXT,
                dht_anchor_hash TEXT,
                p2p_published_at TEXT,
                crdt_converged_at TEXT,
                declared_head_action_hash TEXT,
                declared_head_at BIGINT,
                canonical_declared_at BIGINT,
                canonical_earned INTEGER,
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
            dht_anchor_hash: None,
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
            dht_anchor_hash: None,
        };
        create_content(&mut conn, &elohim_ctx, elohim_content).unwrap();

        // Verify lamad app can see only its content
        let lamad_count = content_count(&mut conn, &lamad_ctx).unwrap();
        assert_eq!(lamad_count, 1, "Lamad should have 1 content item");

        let lamad_manifesto =
            get_content(&mut conn, &lamad_ctx, "manifesto", MinTrust::Invisible).unwrap();
        assert!(lamad_manifesto.is_some(), "Lamad should find manifesto");

        let lamad_resources =
            get_content(&mut conn, &lamad_ctx, "resources", MinTrust::Invisible).unwrap();
        assert!(
            lamad_resources.is_none(),
            "Lamad should NOT find elohim's resources"
        );

        // Verify elohim app can see only its content
        let elohim_count = content_count(&mut conn, &elohim_ctx).unwrap();
        assert_eq!(elohim_count, 1, "Elohim should have 1 content item");

        let elohim_resources =
            get_content(&mut conn, &elohim_ctx, "resources", MinTrust::Invisible).unwrap();
        assert!(elohim_resources.is_some(), "Elohim should find resources");

        let elohim_manifesto =
            get_content(&mut conn, &elohim_ctx, "manifesto", MinTrust::Invisible).unwrap();
        assert!(
            elohim_manifesto.is_none(),
            "Elohim should NOT find lamad's manifesto"
        );
    }

    #[test]
    fn content_ids_with_reach_excludes_non_commons_present_item() {
        // Provide-loop gate: a locally-present item that is NOT commons-reach must
        // be excluded from the desired set, so it never mints a spurious
        // replicates-commons commitment.
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let mk = |id: &str, reach: &str| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
            dht_anchor_hash: None,
        };
        create_content(&mut conn, &ctx, mk("epr:commons-1", "commons")).unwrap();
        create_content(&mut conn, &ctx, mk("epr:private-1", "private")).unwrap();
        create_content(&mut conn, &ctx, mk("epr:public-1", "public")).unwrap();

        let ids = vec![
            "epr:commons-1".to_string(),
            "epr:private-1".to_string(),
            "epr:public-1".to_string(),
            "epr:absent".to_string(), // not present at all
        ];

        // Presence (reach-agnostic) sees the three present items.
        let present = content_ids_present(&mut conn, &ctx, &ids).unwrap();
        assert_eq!(present.len(), 3, "three of four ids are locally present");

        // The commons gate sees ONLY the commons item — the private/public
        // present items are excluded.
        let commons = content_ids_with_reach(&mut conn, &ctx, &ids, "commons").unwrap();
        assert_eq!(
            commons.len(),
            1,
            "only the commons-reach item passes the gate"
        );
        assert!(commons.contains("epr:commons-1"));
        assert!(
            !commons.contains("epr:private-1"),
            "a non-commons present item must be excluded from the commons gate"
        );
        assert!(!commons.contains("epr:public-1"));
        assert!(!commons.contains("epr:absent"));

        // Empty ids → empty set (no all-rows leak).
        let empty = content_ids_with_reach(&mut conn, &ctx, &[], "commons").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn content_reaches_for_ids_returns_per_content_reach() {
        // The reach-aware eligibility path needs per-content reach (not a fixed
        // filter): each present id maps to its own declared reach so the caller
        // can classify reach-aware.
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let mk = |id: &str, reach: &str| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
            dht_anchor_hash: None,
        };
        create_content(&mut conn, &ctx, mk("epr:commons-2", "commons")).unwrap();
        create_content(&mut conn, &ctx, mk("epr:household-2", "household")).unwrap();

        let ids = vec![
            "epr:commons-2".to_string(),
            "epr:household-2".to_string(),
            "epr:absent-2".to_string(), // not present
        ];
        let pairs = content_reaches_for_ids(&mut conn, &ctx, &ids).unwrap();
        let map: std::collections::HashMap<String, String> = pairs.into_iter().collect();
        assert_eq!(map.len(), 2, "only present ids are returned");
        assert_eq!(
            map.get("epr:commons-2").map(String::as_str),
            Some("commons")
        );
        assert_eq!(
            map.get("epr:household-2").map(String::as_str),
            Some("household"),
            "household content reports its own reach, not commons"
        );
        assert!(!map.contains_key("epr:absent-2"));

        // Empty ids → empty.
        assert!(content_reaches_for_ids(&mut conn, &ctx, &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn anchored_content_reaches_for_ids_selects_only_the_ghost_candidate_class() {
        // The ghost-anchor witness class is EXACTLY "present + anchored". A
        // NULL-anchor row belongs to `reanchor_backfill`'s existing sweep and
        // must NOT be double-claimed here; an absent row is the acquisition
        // plane's business and must never be fabricated.
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let mk = |id: &str, reach: &str, anchor: Option<&str>| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
            dht_anchor_hash: anchor.map(str::to_string),
        };
        create_content(
            &mut conn,
            &ctx,
            mk("ghost-1", "commons", Some("uhCkk-ghost")),
        )
        .unwrap();
        create_content(&mut conn, &ctx, mk("unanchored-1", "commons", None)).unwrap();

        let ids = vec![
            "ghost-1".to_string(),
            "unanchored-1".to_string(),
            "absent-1".to_string(),
        ];
        let rows = anchored_content_reaches_for_ids(&mut conn, &ctx, &ids).unwrap();
        let map: std::collections::HashMap<String, (String, String)> = rows
            .into_iter()
            .map(|(id, reach, content_type)| (id, (reach, content_type)))
            .collect();
        assert_eq!(
            map.len(),
            1,
            "only the present+anchored row is a ghost candidate"
        );
        assert_eq!(
            map.get("ghost-1")
                .map(|(reach, ct)| (reach.as_str(), ct.as_str())),
            Some(("commons", "concept"))
        );
        assert!(
            !map.contains_key("unanchored-1"),
            "a NULL-anchor row is the existing witness sweep's candidate, not a ghost"
        );
        assert!(
            !map.contains_key("absent-1"),
            "an absent id must never enter the authoring path"
        );

        // Empty ids → empty (no all-rows leak).
        assert!(anchored_content_reaches_for_ids(&mut conn, &ctx, &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn content_anchor_inventory_excludes_scoped_reach() {
        // Notary-authority Leg 4: the cross-peer content-anchor inventory must
        // advertise ONLY anchored, distribution-safe (community/public/commons)
        // rows. Two exclusions are REQUIREMENTS, not optimizations:
        //   - un-anchored rows (no notary provenance to advertise); and
        //   - scoped-tier rows (must never leak into a cross-peer surface).
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let mk = |id: &str, reach: &str, anchor: Option<&str>| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
            dht_anchor_hash: anchor.map(str::to_string),
        };

        // Set a distinct `updated_at` on each row so the `updated_at DESC`
        // ordering is deterministic. `create_content` stamps `datetime('now')`
        // on every row (indistinguishable at test speed), so we overwrite here.
        let stamp = |conn: &mut SqliteConnection, id: &str, ts: &str| {
            diesel::update(content::table.filter(content::id.eq(id)))
                .set(content::updated_at.eq(ts))
                .execute(conn)
                .unwrap();
        };

        // Distribution-safe + anchored → INCLUDED. Distinct updated_at values
        // chosen so the hot set (most-recently-updated) is `c:public`, then
        // `c:community`, then `c:commons` — NOT their id-ascending order, so the
        // test would fail if the ORDER BY silently reverted to id-asc.
        create_content(
            &mut conn,
            &ctx,
            mk("c:public", "public", Some("anc-public")),
        )
        .unwrap();
        create_content(
            &mut conn,
            &ctx,
            mk("c:commons", "commons", Some("anc-commons")),
        )
        .unwrap();
        create_content(
            &mut conn,
            &ctx,
            mk("c:community", "community", Some("anc-community")),
        )
        .unwrap();
        stamp(&mut conn, "c:public", "2026-01-03T00:00:00Z"); // newest
        stamp(&mut conn, "c:community", "2026-01-02T00:00:00Z");
        stamp(&mut conn, "c:commons", "2026-01-01T00:00:00Z"); // oldest
                                                               // Distribution-safe but UN-anchored → EXCLUDED (no provenance).
        create_content(&mut conn, &ctx, mk("c:public-noanchor", "public", None)).unwrap();
        // Scoped-tier reach, even though anchored → EXCLUDED (must not leak).
        create_content(
            &mut conn,
            &ctx,
            mk("c:private", "private", Some("anc-private")),
        )
        .unwrap();
        create_content(&mut conn, &ctx, mk("c:self", "self", Some("anc-self"))).unwrap();
        create_content(
            &mut conn,
            &ctx,
            mk("c:trusted", "trusted", Some("anc-trusted")),
        )
        .unwrap();

        let (inv, total) = list_content_anchor_inventory(&mut conn, &ctx, 0, i64::MAX).unwrap();
        let ids: Vec<&str> = inv.iter().map(|r| r.id.as_str()).collect();

        // Exactly the three anchored distribution-safe rows, ordered updated_at
        // DESC (hot set first) — the reconcile-hot ordering, not id-asc.
        assert_eq!(
            ids,
            vec!["c:public", "c:community", "c:commons"],
            "only anchored distribution-safe rows, ordered by updated_at desc (hot set first)"
        );
        // Honest total counts exactly the anchored distribution-safe rows.
        assert_eq!(total, 3, "total counts the anchored distribution-safe rows");
        // Anchor value carried through verbatim (discovery pair).
        let map: std::collections::HashMap<String, String> =
            inv.into_iter().map(|r| (r.id, r.dht_anchor_hash)).collect();
        assert_eq!(map.get("c:public").map(String::as_str), Some("anc-public"));
        // Scoped-tier ids must NEVER appear in a cross-peer inventory.
        assert!(!map.contains_key("c:private"));
        assert!(!map.contains_key("c:self"));
        assert!(!map.contains_key("c:trusted"));
        // Un-anchored distribution-safe id is excluded (no provenance).
        assert!(!map.contains_key("c:public-noanchor"));

        // Cap is respected (LIMIT) AND truncation surfaces the hot set: capping
        // at 2 keeps the two most-recently-updated rows, drops the cold tail —
        // but the honest `total` still reports the full 3 so truncation is
        // visible on the wire (the structural non-convergence fix).
        let (capped, capped_total) = list_content_anchor_inventory(&mut conn, &ctx, 0, 2).unwrap();
        let capped_ids: Vec<&str> = capped.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            capped_ids,
            vec!["c:public", "c:community"],
            "cap limits the rows AND surfaces the hot set (drops the coldest tail)"
        );
        assert_eq!(
            capped_total, 3,
            "honest total exceeds the served page count under a cap (truncation is visible)"
        );

        // Offset advances a rotating window: page (offset=2, cap=2) covers the
        // cold tail the first page dropped — so successive sweeps see the whole
        // corpus, not just the perpetual hot set.
        let (windowed, windowed_total) =
            list_content_anchor_inventory(&mut conn, &ctx, 2, 2).unwrap();
        let windowed_ids: Vec<&str> = windowed.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            windowed_ids,
            vec!["c:commons"],
            "offset window reaches the cold tail beyond the first page"
        );
        assert_eq!(
            windowed_total, 3,
            "total is offset-invariant (whole-corpus count)"
        );
    }

    #[test]
    fn content_anchor_inventory_tiebreaks_by_id_when_updated_at_equal() {
        // When updated_at ties, the `id` ASC tiebreak makes the order
        // deterministic (no arbitrary row shuffling under truncation).
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let mk = |id: &str| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
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
            dht_anchor_hash: Some(format!("anc-{id}")),
        };
        create_content(&mut conn, &ctx, mk("c:zeta")).unwrap();
        create_content(&mut conn, &ctx, mk("c:alpha")).unwrap();
        create_content(&mut conn, &ctx, mk("c:mid")).unwrap();
        // Identical updated_at across all three → the id-asc tiebreak decides.
        for id in ["c:zeta", "c:alpha", "c:mid"] {
            diesel::update(content::table.filter(content::id.eq(id)))
                .set(content::updated_at.eq("2026-02-01T00:00:00Z"))
                .execute(&mut conn)
                .unwrap();
        }

        let (inv, _total) = list_content_anchor_inventory(&mut conn, &ctx, 0, i64::MAX).unwrap();
        let ids: Vec<&str> = inv.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["c:alpha", "c:mid", "c:zeta"],
            "equal updated_at falls back to id ascending"
        );
    }

    #[test]
    fn head_corpus_digest_is_stable_and_order_independent() {
        // T4 (head-plane trust-gradient program): two identical corpora must
        // produce the SAME digest regardless of insertion order — the sort
        // step in `digest_of_entry_lines` is what the whole `in_sync`
        // shortcut rests on. Mirrors
        // `p2p::sync_round::tests::corpus_digest_matches_the_pre_refactor_pinned_value`
        // for the OTHER corpus this fold now serves.
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let mk = |id: &str, anchor: &str| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
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
            dht_anchor_hash: Some(anchor.to_string()),
        };
        create_content(&mut conn, &ctx, mk("c:a", "anc-a")).unwrap();
        create_content(&mut conn, &ctx, mk("c:b", "anc-b")).unwrap();
        let digest_ab = head_corpus_digest(&mut conn, &ctx).unwrap();
        assert!(digest_ab.starts_with("sha256:"));

        let mut conn2 = setup_test_db();
        let ctx2 = AppContext::new("lamad");
        // Reverse insertion order.
        create_content(&mut conn2, &ctx2, mk("c:b", "anc-b")).unwrap();
        create_content(&mut conn2, &ctx2, mk("c:a", "anc-a")).unwrap();
        let digest_ba = head_corpus_digest(&mut conn2, &ctx2).unwrap();
        assert_eq!(
            digest_ab, digest_ba,
            "insertion order must not affect the digest — both peers hashing the \
             same logical corpus must agree regardless of row-arrival order"
        );
    }

    #[test]
    fn head_corpus_digest_excludes_scoped_and_unanchored_rows_and_changes_on_anchor_change() {
        // Reuses the SAME DISTRIBUTION_SAFE_REACH gate as
        // `list_content_anchor_inventory` — a scoped-tier or un-anchored row
        // must never perturb the cross-peer digest either way.
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let mk = |id: &str, reach: &str, anchor: Option<&str>| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
            dht_anchor_hash: anchor.map(str::to_string),
        };
        create_content(&mut conn, &ctx, mk("c:pub", "public", Some("anc-pub"))).unwrap();
        let baseline = head_corpus_digest(&mut conn, &ctx).unwrap();

        create_content(&mut conn, &ctx, mk("c:priv", "private", Some("anc-priv"))).unwrap();
        create_content(&mut conn, &ctx, mk("c:noanchor", "public", None)).unwrap();
        let after_noise = head_corpus_digest(&mut conn, &ctx).unwrap();
        assert_eq!(
            baseline, after_noise,
            "a scoped-tier row and an un-anchored row must not change the digest — \
             neither is eligible for the cross-peer inventory this digest fingerprints"
        );

        // Changing the eligible row's anchor DOES change the digest.
        diesel::update(content::table.filter(content::id.eq("c:pub")))
            .set(content::dht_anchor_hash.eq("anc-pub-CHANGED"))
            .execute(&mut conn)
            .unwrap();
        let after_change = head_corpus_digest(&mut conn, &ctx).unwrap();
        assert_ne!(
            baseline, after_change,
            "a changed anchor on an eligible row MUST change the digest — this is the \
             signal the in_sync shortcut relies on to catch divergence"
        );
    }

    /// The SQL half of the alpha-freeze cure: the three predicates the cross-peer
    /// gap classifier consumes must disagree in exactly the diagnosed way, so a
    /// row that is present AND anchored but scoped is never mistakable for an
    /// un-anchored one.
    ///
    /// Live shape this pins (matthew, 2026-08-19): `gaps{content}=2585` against
    /// `reanchorPending=1`. Reading "not in the distribution-safe map" as
    /// "un-anchored" turned ~2584 scoped rows into a permanent gap population.
    #[test]
    fn the_anchored_id_set_is_reach_agnostic_while_the_advertised_inventory_is_not() {
        let mut conn = setup_test_db();
        let ctx = AppContext::default_lamad();
        let mk = |id: &str, reach: &str, anchor: Option<&str>| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
            dht_anchor_hash: anchor.map(str::to_string),
        };
        // scoped + ANCHORED — the class that froze the fleet.
        create_content(&mut conn, &ctx, mk("c:scoped", "trusted", Some("anc-s"))).unwrap();
        // distribution-safe + anchored — advertisable.
        create_content(&mut conn, &ctx, mk("c:safe", "public", Some("anc-p"))).unwrap();
        // distribution-safe + UN-anchored — the genuine anchor gap (scenario 2).
        create_content(&mut conn, &ctx, mk("c:null", "public", None)).unwrap();

        let ids: Vec<String> = ["c:scoped", "c:safe", "c:null"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let present = content_ids_present(&mut conn, &ctx, &ids).unwrap();
        assert!(
            present.contains("c:scoped"),
            "presence is reach-agnostic — the scoped row IS held locally"
        );

        let (advertised, _total) =
            list_content_anchor_inventory(&mut conn, &ctx, 0, i64::MAX).unwrap();
        let advertised: std::collections::HashSet<String> =
            advertised.into_iter().map(|r| r.id).collect();
        assert!(
            !advertised.contains("c:scoped"),
            "the advertised inventory MUST stay distribution-safe filtered — this is a \
             requirement, not an optimization"
        );

        let anchored = anchored_content_ids_any_reach(&mut conn, &ctx).unwrap();
        assert!(
            anchored.contains("c:scoped"),
            "the scoped row is ANCHORED; only a reach-agnostic set can say so, and without \
             it the classifier reads the row as un-anchored forever"
        );
        assert!(
            !anchored.contains("c:null"),
            "a genuinely un-anchored row must stay OUT of the anchored set, or the fix \
             would silence the real scenario-2 heal"
        );
        assert!(advertised.contains("c:safe") && anchored.contains("c:safe"));
    }

    #[test]
    fn head_corpus_digest_readiness_abstains_until_distribution_safe_rows_are_witnessed() {
        let mut conn = setup_test_db();
        let ctx = AppContext::default_lamad();
        let mk = |id: &str, reach: &str, anchor: Option<&str>| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: reach.to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
            dht_anchor_hash: anchor.map(str::to_string),
        };
        create_content(&mut conn, &ctx, mk("c:green", "public", Some("anc-green"))).unwrap();
        create_content(&mut conn, &ctx, mk("c:amber", "commons", None)).unwrap();
        create_content(&mut conn, &ctx, mk("c:private", "private", None)).unwrap();

        assert_eq!(
            head_corpus_digest_readiness(&mut conn, &ctx).unwrap(),
            HeadCorpusDigestReadiness::Amber { pending: 1 },
            "only a distribution-safe NULL-anchor row holds the digest in amber; scoped rows never cross this peer boundary"
        );

        diesel::update(content::table.filter(content::id.eq("c:amber")))
            .set(content::dht_anchor_hash.eq("anc-amber"))
            .execute(&mut conn)
            .unwrap();

        let expected = head_corpus_digest(&mut conn, &ctx).unwrap();
        assert_eq!(
            head_corpus_digest_readiness(&mut conn, &ctx).unwrap(),
            HeadCorpusDigestReadiness::Ready {
                digest: expected,
                total: 2,
            },
            "the shortcut becomes eligible exactly when the whole distribution-safe relation is witnessed"
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
                dht_anchor_hash: None,
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
                dht_anchor_hash: None,
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
            dht_anchor_hash: None,
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
            dht_anchor_hash: None,
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
            MinTrust::Invisible,
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
            MinTrust::Amber,
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
            dht_anchor_hash: None,
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

        let unrestricted = count_content(
            &mut conn,
            &ctx,
            &ContentQuery::default(),
            MinTrust::Invisible,
        )
        .unwrap();
        assert_eq!(unrestricted, 2, "unrestricted count should see both rows");

        let gated =
            count_content(&mut conn, &ctx, &ContentQuery::default(), MinTrust::Amber).unwrap();
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
            dht_anchor_hash: None,
        };
        create_content(&mut conn, &ctx, row).unwrap();

        // Gated fetch: row has no dht_anchor_hash and no p2p_published_at →
        // external reads must see None, as if the row did not exist.
        let gated = get_content(&mut conn, &ctx, "cid-unpublished", MinTrust::Amber).unwrap();
        assert!(
            gated.is_none(),
            "gated get_content must hide unpublished rows from external readers"
        );

        // Ungated fetch: internal callers (drain loop) must still see it.
        let ungated = get_content(&mut conn, &ctx, "cid-unpublished", MinTrust::Invisible).unwrap();
        assert!(
            ungated.is_some(),
            "ungated get_content must still return unpublished rows for internal callers"
        );
        assert_eq!(ungated.unwrap().id, "cid-unpublished");
    }

    #[test]
    fn test_create_content_with_dht_anchor_passes_provenance_at_ingest() {
        // The seed/ingest fix (seed-provenance-anchor-gap): content created WITH
        // a content-derived `dht_anchor_hash` must satisfy the
        // `require_provenance=true` read gate IMMEDIATELY — no libp2p publish
        // drain round-trip — so hub-optional / peer-starved seed stacks can serve
        // it. Content created WITHOUT an anchor (and never published) stays hidden.
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let anchored = CreateContentInput {
            id: "cid-anchored".to_string(),
            title: "Anchored at ingest".to_string(),
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
            content_body: Some("hello".to_string()),
            dht_anchor_hash: Some("bafy-content-derived-anchor".to_string()),
        };
        let unanchored = CreateContentInput {
            id: "cid-unanchored".to_string(),
            dht_anchor_hash: None,
            ..anchored.clone()
        };
        create_content(&mut conn, &ctx, anchored).unwrap();
        create_content(&mut conn, &ctx, unanchored).unwrap();

        // Gated (external) read: the ingest-anchored row is visible WITHOUT any
        // p2p_published_at stamp; the un-anchored row is hidden.
        let anchored_read = get_content(&mut conn, &ctx, "cid-anchored", MinTrust::Amber).unwrap();
        assert!(
            anchored_read.is_some(),
            "ingest dht_anchor_hash must satisfy the Amber trust gate at create time"
        );
        let unanchored_read =
            get_content(&mut conn, &ctx, "cid-unanchored", MinTrust::Amber).unwrap();
        assert!(
            unanchored_read.is_none(),
            "content with neither anchor nor publish must stay hidden from external reads"
        );
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
            dht_anchor_hash: None,
        };
        create_content(&mut conn, &ctx, row).unwrap();

        let q = ContentQuery {
            limit: 10,
            ..Default::default()
        };

        // External call pattern — what handle_db_content_list will invoke.
        let external = list_content(&mut conn, &ctx, &q, MinTrust::Amber).unwrap();
        assert!(
            external.is_empty(),
            "external list must not leak unpublished content"
        );

        // Internal call pattern — what p2p replication/drain paths invoke.
        let internal = list_content(&mut conn, &ctx, &q, MinTrust::Invisible).unwrap();
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
            dht_anchor_hash: None,
        };
        create_content(&mut conn, &ctx, row).unwrap();

        // External call pattern — what handle_db_content_by_id will invoke.
        let external =
            get_content_with_tags(&mut conn, &ctx, "cid-unpublished", MinTrust::Amber).unwrap();
        assert!(
            external.is_none(),
            "external get must not leak unpublished content"
        );

        // Internal call pattern.
        let internal =
            get_content_with_tags(&mut conn, &ctx, "cid-unpublished", MinTrust::Invisible).unwrap();
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
                    dht_anchor_hash: None,
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
                dht_anchor_hash: None,
            },
        )
        .unwrap();

        let pending_before = list_unpublished_content_ids(&mut conn, &ctx, 10).unwrap();
        assert_eq!(pending_before.len(), 1);

        let marked = mark_published(&mut conn, &ctx, "x").unwrap();
        assert!(
            marked,
            "mark_published should return true for an existing row"
        );

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
        assert!(
            ts.ends_with('Z'),
            "expected Zulu-suffixed RFC 3339, got: {}",
            ts
        );
        chrono::DateTime::parse_from_rfc3339(&ts)
            .expect("p2p_published_at should parse as RFC 3339");

        // Also verify mark_published on a non-existent row returns false (not an error).
        let missing = mark_published(&mut conn, &ctx, "nonexistent").unwrap();
        assert!(
            !missing,
            "mark_published should return false for a missing row"
        );
    }

    #[test]
    fn test_count_publish_state_returns_correct_counts() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Empty table: (0, 0)
        let (total, published) = count_publish_state(&mut conn, &ctx).unwrap();
        assert_eq!((total, published), (0, 0));

        // Insert 2 rows, mark 1 as published.
        for id in ["row1", "row2"] {
            create_content(
                &mut conn,
                &ctx,
                CreateContentInput {
                    id: id.into(),
                    title: id.into(),
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
                    dht_anchor_hash: None,
                },
            )
            .unwrap();
        }
        mark_published(&mut conn, &ctx, "row1").unwrap();

        let (total, published) = count_publish_state(&mut conn, &ctx).unwrap();
        assert_eq!((total, published), (2, 1));

        // All published: (2, 2) — this is the E2 seeder's termination condition.
        mark_published(&mut conn, &ctx, "row2").unwrap();
        let (total, published) = count_publish_state(&mut conn, &ctx).unwrap();
        assert_eq!((total, published), (2, 2));
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
            blob_hash: None,
            server_blob_hash: None,
            p2p_published_at: None,
            crdt_converged_at: None,
        };
        assert_eq!(input.id, "test-id");
    }

    /// PATCH semantics: stamping only `p2p_published_at` satisfies the
    /// provenance read gate (`dht_anchor_hash OR p2p_published_at`) without
    /// touching any other field. This is the genesis-seeder path for
    /// household/local stacks with no DHT peers — see the local-stack
    /// DHT-anchor gap.
    #[test]
    fn test_update_content_p2p_published_at_only_satisfies_provenance() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Seed a row with NO provenance markers (the bulk-seed state).
        let create = CreateContentInput {
            id: "patch-prov-test".to_string(),
            title: "Seeded".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "public".to_string(),
            created_by: None,
            content_body: None,
            dht_anchor_hash: None,
            tags: vec![],
        };
        bulk_create_content(&mut conn, &ctx, vec![create]).unwrap();

        // Provenance gate hides it: require_provenance=true → None.
        assert!(
            get_content_with_tags(&mut conn, &ctx, "patch-prov-test", MinTrust::Amber)
                .unwrap()
                .is_none(),
            "pre-stamp: provenance gate must hide an unpublished row"
        );

        // PATCH only p2p_published_at.
        let stamp = "2026-06-04T00:00:00Z";
        let update = UpdateContentInput {
            id: "patch-prov-test".to_string(),
            p2p_published_at: Some(stamp.to_string()),
            ..Default::default()
        };
        let result = update_content(&mut conn, &ctx, update).unwrap();
        assert_eq!(result.content.p2p_published_at.as_deref(), Some(stamp));
        assert_eq!(result.content.title, "Seeded");

        // Provenance gate now passes — the row is visible to external reads.
        assert!(
            get_content_with_tags(&mut conn, &ctx, "patch-prov-test", MinTrust::Amber)
                .unwrap()
                .is_some(),
            "post-stamp: provenance gate must reveal a stamped row"
        );
    }

    /// PATCH semantics: setting only blob_hash leaves other fields untouched.
    /// This is the deploy-time stageSpaBlob path — see
    /// genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md.
    #[test]
    fn test_update_content_blob_hash_only_preserves_other_fields() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Seed a row with a known title + blob_hash="".
        let create = CreateContentInput {
            id: "patch-bh-test".to_string(),
            title: "Original Title".to_string(),
            description: Some("Original Description".to_string()),
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: Some("".to_string()),
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "public".to_string(),
            created_by: None,
            content_body: None,
            dht_anchor_hash: None,
            tags: vec![],
        };
        bulk_create_content(&mut conn, &ctx, vec![create]).unwrap();

        // PATCH only blob_hash.
        let update = UpdateContentInput {
            id: "patch-bh-test".to_string(),
            blob_hash: Some("sha256-deadbeef".to_string()),
            ..Default::default()
        };
        let result = update_content(&mut conn, &ctx, update).unwrap();

        assert_eq!(result.content.blob_hash.as_deref(), Some("sha256-deadbeef"));
        // Other fields unchanged.
        assert_eq!(result.content.title, "Original Title");
        assert_eq!(
            result.content.description.as_deref(),
            Some("Original Description")
        );
    }

    /// PATCH semantics (SSR row collapse T2): setting only `server_blob_hash`
    /// persists it, returns it on the subsequent read, and does NOT clobber
    /// `blob_hash` or other fields — and vice-versa, a later `blob_hash`-only
    /// PATCH retains the previously-set `server_blob_hash`. This is the deploy-time
    /// SSR PATCH path (browser bundle → blob_hash, server bundle → server_blob_hash,
    /// patched independently per host).
    #[test]
    fn test_update_content_server_blob_hash_only_preserves_and_no_clobber() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Seed the one landing row with a browser blob_hash already set; no SSR yet.
        let create = CreateContentInput {
            id: "elohim-host-landing".to_string(),
            title: "Landing".to_string(),
            description: Some("Landing desc".to_string()),
            content_type: "concept".to_string(),
            content_format: "spa-bundle".to_string(),
            blob_hash: Some("sha256-browser".to_string()),
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: None,
            content_body: None,
            dht_anchor_hash: None,
            tags: vec![],
        };
        bulk_create_content(&mut conn, &ctx, vec![create]).unwrap();
        // Pre-state: server_blob_hash absent (the normal pre-deploy state).
        let pre =
            get_content_with_tags(&mut conn, &ctx, "elohim-host-landing", MinTrust::Invisible)
                .unwrap()
                .unwrap();
        assert_eq!(pre.content.server_blob_hash, None);

        // PATCH only server_blob_hash — must persist it and leave blob_hash/title alone.
        let update = UpdateContentInput {
            id: "elohim-host-landing".to_string(),
            server_blob_hash: Some("sha256-ssrserver".to_string()),
            ..Default::default()
        };
        let result = update_content(&mut conn, &ctx, update).unwrap();
        assert_eq!(
            result.content.server_blob_hash.as_deref(),
            Some("sha256-ssrserver"),
            "serverBlobHash-only PATCH persists serverBlobHash"
        );
        assert_eq!(
            result.content.blob_hash.as_deref(),
            Some("sha256-browser"),
            "serverBlobHash-only PATCH must NOT clobber blob_hash"
        );
        assert_eq!(result.content.title, "Landing");

        // GET-equivalent re-read returns serverBlobHash (round-trip).
        let got =
            get_content_with_tags(&mut conn, &ctx, "elohim-host-landing", MinTrust::Invisible)
                .unwrap()
                .unwrap();
        assert_eq!(
            got.content.server_blob_hash.as_deref(),
            Some("sha256-ssrserver")
        );

        // Now PATCH only blob_hash — must update blob_hash AND retain serverBlobHash.
        let update2 = UpdateContentInput {
            id: "elohim-host-landing".to_string(),
            blob_hash: Some("sha256-browser-v2".to_string()),
            ..Default::default()
        };
        let result2 = update_content(&mut conn, &ctx, update2).unwrap();
        assert_eq!(
            result2.content.blob_hash.as_deref(),
            Some("sha256-browser-v2"),
            "blob_hash-only PATCH updates blob_hash"
        );
        assert_eq!(
            result2.content.server_blob_hash.as_deref(),
            Some("sha256-ssrserver"),
            "blob_hash-only PATCH must retain the previously-set serverBlobHash (no clobber)"
        );
    }

    /// A2 proof (RED before the tri-state gate): a row with ONLY
    /// `crdt_converged_at` set — no `dht_anchor_hash`, no `p2p_published_at` —
    /// is the "amber" tier. `MinTrust::Amber` (the serving floor) must admit it;
    /// `MinTrust::Green` (notarized-only, for authority/attribution/economic
    /// reads) must exclude it. A `dht_anchor_hash`-bearing row is admitted by
    /// BOTH. The old binary provenance gate could not express this distinction.
    #[test]
    fn amber_admitted_green_excluded() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let mk = |id: &str| CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
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
            dht_anchor_hash: None,
        };

        // Amber row: converged only (no anchor, no publish stamp).
        create_content(&mut conn, &ctx, mk("cid-amber")).unwrap();
        diesel::sql_query(
            "UPDATE content SET crdt_converged_at = datetime('now') WHERE id = 'cid-amber'",
        )
        .execute(&mut conn)
        .unwrap();

        // Green row: DHT-notarized (dht_anchor_hash set at ingest).
        let mut anchored = mk("cid-green");
        anchored.dht_anchor_hash = Some("bafy-notarized-anchor".to_string());
        create_content(&mut conn, &ctx, anchored).unwrap();

        let q = ContentQuery {
            limit: 10,
            ..Default::default()
        };

        // Amber tier admits BOTH the converged row and the notarized row.
        let amber = list_content(&mut conn, &ctx, &q, MinTrust::Amber).unwrap();
        let amber_ids: std::collections::HashSet<&str> =
            amber.iter().map(|c| c.content.id.as_str()).collect();
        assert!(
            amber_ids.contains("cid-amber"),
            "Amber must admit a converged-only row"
        );
        assert!(
            amber_ids.contains("cid-green"),
            "Amber must admit a notarized row"
        );

        // Green tier excludes the bare-converged amber row, admits only notarized.
        let green = list_content(&mut conn, &ctx, &q, MinTrust::Green).unwrap();
        let green_ids: std::collections::HashSet<&str> =
            green.iter().map(|c| c.content.id.as_str()).collect();
        assert!(
            !green_ids.contains("cid-amber"),
            "Green must EXCLUDE a bare-converged (non-notarized) row"
        );
        assert!(
            green_ids.contains("cid-green"),
            "Green must admit a notarized row"
        );

        // Cross-check the same distinction on get_content (single-row path).
        assert!(
            get_content(&mut conn, &ctx, "cid-amber", MinTrust::Amber)
                .unwrap()
                .is_some(),
            "Amber get_content admits the converged row"
        );
        assert!(
            get_content(&mut conn, &ctx, "cid-amber", MinTrust::Green)
                .unwrap()
                .is_none(),
            "Green get_content hides the converged row"
        );
    }

    /// Build a bare SPA-bundle content row (no blob_hash, no provenance) — the
    /// exact pre-deploy state that 404s on the live host today.
    fn mk_bundle(id: &str) -> CreateContentInput {
        CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            content_type: "app".to_string(),
            content_format: "spa-bundle".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: None,
            tags: vec![],
            content_body: None,
            dht_anchor_hash: None,
        }
    }

    /// The app-bundle slug query `lookup_slug_blob_hash` runs (content_format in
    /// the app-bundle set, MinTrust::Amber). Returns the resolved blob_hash for
    /// `id`, or None if the row is below the Amber floor / has no hash.
    fn slug_lookup_amber(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
    ) -> Option<String> {
        let q = ContentQuery {
            content_format: Some("spa-bundle".to_string()),
            limit: 100,
            ..Default::default()
        };
        list_content(conn, ctx, &q, MinTrust::Amber)
            .unwrap()
            .into_iter()
            .find(|c| c.content.id == id)
            .and_then(|c| c.content.blob_hash.filter(|h| !h.is_empty()))
    }

    /// Amber serving floor: a row at the amber tier (blob_hash + crdt_converged_at
    /// stamped, NEVER dht_anchor_hash) resolves through the SAME query
    /// `lookup_slug_blob_hash` runs (MinTrust::Amber) — so the SPA mount serves
    /// 200 during the convergence window. Amber rows now arise ONLY from the
    /// DocStore reverse-projection drift-heal (`reverse_project_content_doc`),
    /// never from a deploy write (the per-host `deployTier=amber` write was
    /// retired in favor of one conductor-authored head). Authority reads (Green)
    /// still exclude the row — amber is a provisional, un-witnessed signal, never
    /// authoritative.
    #[test]
    fn amber_tier_row_serves_at_amber_floor() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let id = "epr:elohim-host-landing";
        create_content(&mut conn, &ctx, mk_bundle(id)).unwrap();

        // RED: pre-amber the row is below the Amber floor (no provenance marker)
        // → the slug lookup resolves nothing → 404.
        assert_eq!(
            slug_lookup_amber(&mut conn, &ctx, id),
            None,
            "pre-amber: no provenance marker → slug lookup 404s"
        );

        // An amber-tier write at the db layer — what the drift-heal
        // (reverse_project_content_doc) emits: set blob_hash + stamp
        // crdt_converged_at, never dht_anchor_hash.
        let amber = UpdateContentInput {
            id: id.to_string(),
            blob_hash: Some("sha256-amberbundle".to_string()),
            crdt_converged_at: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        };
        update_content(&mut conn, &ctx, amber).unwrap();

        // Row state: blob_hash + crdt_converged_at set, dht_anchor_hash STILL NULL.
        let row = get_content(&mut conn, &ctx, id, MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(row.blob_hash.as_deref(), Some("sha256-amberbundle"));
        assert!(row.crdt_converged_at.is_some(), "amber tier stamped");
        assert!(
            row.dht_anchor_hash.is_none(),
            "amber write must NEVER set dht_anchor_hash (no laundering into notarized)"
        );

        // GREEN: the amber row now resolves at the slug lookup → mount serves 200.
        assert_eq!(
            slug_lookup_amber(&mut conn, &ctx, id),
            Some("sha256-amberbundle".to_string()),
            "amber row resolves at the Amber-floor slug lookup → SPA mount serves 200"
        );

        // Defense-in-depth: an authority read at Green must NOT see the amber row.
        let green = list_content(
            &mut conn,
            &ctx,
            &ContentQuery {
                content_format: Some("spa-bundle".to_string()),
                limit: 100,
                ..Default::default()
            },
            MinTrust::Green,
        )
        .unwrap();
        assert!(
            green.iter().all(|c| c.content.id != id),
            "Green (authority/attribution) read must exclude the amber row"
        );
    }

    /// A3 precedence guard: once a row carries a notarized (green) blob_hash, a
    /// later amber write must NOT overwrite it — green wins, amber never the
    /// reverse. The crdt marker may still be stamped (harmless), and the existing
    /// dht_anchor_hash is preserved.
    #[test]
    fn amber_write_never_clobbers_notarized_blob_hash() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let id = "epr:pinned-landing";

        let mut green = mk_bundle(id);
        green.blob_hash = Some("sha256-greenbundle".to_string());
        green.dht_anchor_hash = Some("bafy-notarized-anchor".to_string());
        create_content(&mut conn, &ctx, green).unwrap();

        // Amber write tries to set a DIFFERENT blob_hash.
        let amber = UpdateContentInput {
            id: id.to_string(),
            blob_hash: Some("sha256-amberbundle".to_string()),
            crdt_converged_at: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        };
        update_content(&mut conn, &ctx, amber).unwrap();

        let row = get_content(&mut conn, &ctx, id, MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.blob_hash.as_deref(),
            Some("sha256-greenbundle"),
            "amber must NOT clobber a present (notarized) blob_hash"
        );
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("bafy-notarized-anchor"),
            "existing notarized anchor preserved"
        );
    }

    /// A base plain-content input (no anchor, no provenance) for HEAD-election tests.
    fn mk_plain(id: &str) -> CreateContentInput {
        CreateContentInput {
            id: id.to_string(),
            title: id.to_string(),
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
            dht_anchor_hash: None,
        }
    }

    /// HEAD-election (i), `Declare` mode: unchanged behavior — the committed
    /// action advances `declared_head_action_hash` alongside `dht_anchor_hash`.
    /// This is the regression guard for the DELIBERATE channels (the HTTP
    /// re-notarize PATCH); the adopt-before-author fix must not disarm them.
    #[test]
    fn upsert_with_anchor_sets_declared_head() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        create_content(&mut conn, &ctx, mk_plain("cid-head")).unwrap();
        let pre = get_content(&mut conn, &ctx, "cid-head", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert!(
            pre.declared_head_action_hash.is_none(),
            "no declared head before the first own-conductor projection"
        );

        upsert_with_anchor(
            &mut conn,
            &ctx,
            "cid-head",
            ContentProjectionPatch {
                title: Some("Head v2".to_string()),
                ..Default::default()
            },
            "uhCkk-head-action-1",
            HeadElection::Declare,
        )
        .unwrap();

        let row = get_content(&mut conn, &ctx, "cid-head", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkk-head-action-1"));
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-head-action-1"),
            "Declare mode must advance declared_head_action_hash to the committed action"
        );
        assert_eq!(row.title, "Head v2", "patch value field still applied");
    }

    /// Adopt-before-author (a): `PreserveExistingDeclaration` on a row that
    /// ALREADY carries a declaration stamps the anchor and leaves BOTH
    /// declaration columns untouched.
    ///
    /// This is the exact shape of the live defect: peer B had adopted alpha-A's
    /// canonical head for `elohim-host-landing`, then its own restart sweep
    /// re-authored a local root and the projection crowned that root — silently
    /// un-adopting A's head. The anchor may move (that IS what the sweep
    /// healed); the crown may not.
    #[test]
    fn preserve_election_leaves_an_existing_declaration_untouched() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let id = "elohim-host-landing";

        create_content(&mut conn, &ctx, mk_plain(id)).unwrap();
        // Peer B adopted alpha-A's canonical head, ordering and all.
        stamp_declared_head(
            &mut conn,
            &ctx,
            id,
            "uhCkk-ALPHA-A-CANONICAL",
            Some(1_753_000_000_000_000),
            None,
        )
        .unwrap();

        // A restart's re-anchor sweep re-authors a LOCAL root.
        upsert_with_anchor(
            &mut conn,
            &ctx,
            id,
            ContentProjectionPatch {
                title: Some("Re-authored on boot".to_string()),
                ..Default::default()
            },
            "uhCkk-LOCAL-REAUTHORED-ROOT",
            HeadElection::PreserveExistingDeclaration,
        )
        .unwrap();

        let row = get_content(&mut conn, &ctx, id, MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkk-LOCAL-REAUTHORED-ROOT"),
            "the anchor still advances — the row IS now witnessed locally"
        );
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-ALPHA-A-CANONICAL"),
            "the adopted canonical head must survive a local re-author"
        );
        assert_eq!(
            row.declared_head_at,
            Some(1_753_000_000_000_000),
            "declaration ORDERING must survive too — NULLing it silently disarms \
             every later heal-monotonicity check"
        );
        assert_eq!(row.title, "Re-authored on boot", "value fields still patch");
    }

    /// Adopt-before-author (b): `PreserveExistingDeclaration` on an UNDECLARED
    /// row stamps the anchor and leaves the row still undeclared.
    ///
    /// Deliberate: a self-authored root is not an election. Leaving the crown
    /// empty is what lets a later canonical declaration win OUTRIGHT rather than
    /// being refused as "row already declared" (`StampOutcome::SkippedDeclared`).
    #[test]
    fn preserve_election_does_not_create_a_declaration() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let id = "cid-fresh-heal";

        create_content(&mut conn, &ctx, mk_plain(id)).unwrap();
        upsert_with_anchor(
            &mut conn,
            &ctx,
            id,
            ContentProjectionPatch::default(),
            "uhCkk-HEAL-AUTHORED",
            HeadElection::PreserveExistingDeclaration,
        )
        .unwrap();

        let row = get_content(&mut conn, &ctx, id, MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkk-HEAL-AUTHORED"),
            "anchor stamped — the row reaches trust=green (which gates on the \
             ANCHOR, not the declaration)"
        );
        assert!(
            row.declared_head_action_hash.is_none(),
            "a heal-class author must NOT crown itself"
        );
        assert!(row.declared_head_at.is_none());

        // And the defensive INSERT branch behaves identically.
        upsert_with_anchor(
            &mut conn,
            &ctx,
            "cid-never-seeded",
            ContentProjectionPatch {
                title: Some("Authored on a peer that never seeded".to_string()),
                ..Default::default()
            },
            "uhCkk-INSERTED",
            HeadElection::PreserveExistingDeclaration,
        )
        .unwrap();
        let inserted = get_content(&mut conn, &ctx, "cid-never-seeded", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(inserted.dht_anchor_hash.as_deref(), Some("uhCkk-INSERTED"));
        assert!(
            inserted.declared_head_action_hash.is_none(),
            "the insert branch must respect the election too — it is the same \
             heal-class write on a row that simply did not exist yet"
        );
    }

    /// The inventory a peer advertises carries its DECLARATION alongside its
    /// anchor, and never advertises an empty-string declaration as a hint.
    #[test]
    fn anchor_inventory_advertises_the_declaration() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        let mut declared = mk_plain("cid-declared");
        declared.dht_anchor_hash = Some("uhCkk-anchor".to_string());
        create_content(&mut conn, &ctx, declared).unwrap();
        stamp_declared_head(
            &mut conn,
            &ctx,
            "cid-declared",
            "uhCkk-CANONICAL",
            Some(1_753_000_000_000_000),
            None,
        )
        .unwrap();

        let mut anchored_only = mk_plain("cid-anchored-only");
        anchored_only.dht_anchor_hash = Some("uhCkk-anchor-2".to_string());
        create_content(&mut conn, &ctx, anchored_only).unwrap();

        let (rows, _total) = list_content_anchor_inventory(&mut conn, &ctx, 0, 100).unwrap();

        let declared_row = rows
            .iter()
            .find(|r| r.id == "cid-declared")
            .expect("declared row advertised");
        assert_eq!(
            declared_row.declared_head_action_hash.as_deref(),
            Some("uhCkk-CANONICAL"),
            "a peer must advertise its declaration so a restarting peer can ADOPT it"
        );
        assert_eq!(declared_row.declared_head_at, Some(1_753_000_000_000_000));

        let plain_row = rows
            .iter()
            .find(|r| r.id == "cid-anchored-only")
            .expect("anchored-only row advertised");
        assert!(
            plain_row.declared_head_action_hash.is_none(),
            "anchored-but-undeclared advertises no hint — it has no crown to offer"
        );
    }

    /// HEAD-election (ii): `stamp_declared_head` stamps an EXISTING row (both hashes)
    /// and returns `Ok(false)` (no insert) for a missing row; re-stamp is idempotent.
    #[test]
    fn declared_heads_for_batches_the_skipped_declared_predicate() {
        // The reconcile discovery leg classifies a whole sweep's divergence on
        // this one query, so it must agree EXACTLY with the per-id
        // `declared_head_for` that `stamp_declared_head_mode` consults before
        // returning `SkippedDeclared` — a row in the map is a row heal is
        // forbidden to move.
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        create_content(&mut conn, &ctx, mk_plain("declared")).unwrap();
        create_content(&mut conn, &ctx, mk_plain("undeclared")).unwrap();
        stamp_declared_head(&mut conn, &ctx, "declared", "uhCkk-head-1", None, None).unwrap();

        let ids = vec![
            "declared".to_string(),
            "undeclared".to_string(),
            "no-such-row".to_string(),
        ];
        let map = declared_heads_for(&mut conn, &ctx, &ids).unwrap();

        assert_eq!(
            map.get("declared").map(String::as_str),
            Some("uhCkk-head-1")
        );
        assert!(
            !map.contains_key("undeclared"),
            "an undeclared row has nothing local to preserve — heal MAY fill it"
        );
        assert!(
            !map.contains_key("no-such-row"),
            "a missing row is absence, not a declaration"
        );

        // Agreement with the per-id predicate, both directions.
        for id in ["declared", "undeclared", "no-such-row"] {
            assert_eq!(
                declared_head_for(&mut conn, &ctx, id).unwrap(),
                map.get(id).cloned(),
                "batch and per-id predicates disagree on {id}"
            );
        }
    }

    #[test]
    fn stamp_declared_head_stamps_existing_and_false_for_missing() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Missing row → Ok(false), no fabrication.
        let missing =
            stamp_declared_head(&mut conn, &ctx, "cid-absent", "uhCkk-x", None, None).unwrap();
        assert!(
            !missing,
            "stamp on a missing row must return Ok(false) and NOT insert"
        );
        assert!(
            get_content(&mut conn, &ctx, "cid-absent", MinTrust::Invisible)
                .unwrap()
                .is_none(),
            "stamp must never fabricate a row"
        );

        // Existing row → Ok(true), both hashes set to the verified head.
        create_content(&mut conn, &ctx, mk_plain("cid-present")).unwrap();
        let stamped =
            stamp_declared_head(&mut conn, &ctx, "cid-present", "uhCkk-head-9", None, None)
                .unwrap();
        assert!(stamped, "stamp on an existing row must return Ok(true)");

        let row = get_content(&mut conn, &ctx, "cid-present", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-head-9")
        );
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkk-head-9"),
            "a verified stamp is a green write — it advances the notary anchor too"
        );

        // Idempotent re-stamp still returns Ok(true).
        let restamped =
            stamp_declared_head(&mut conn, &ctx, "cid-present", "uhCkk-head-9", None, None)
                .unwrap();
        assert!(restamped, "idempotent re-stamp returns Ok(true)");
    }

    /// HEAD-election (iv) — the boot-resurrection guard (2026-07-11 20:42:40
    /// regression): a GapFill stamp must NEVER move a row that already carries
    /// a DIFFERENT declared head (a fresh-boot conductor resolve falls through
    /// to the root-author election and would resurrect the superseded head);
    /// Declare mode still moves it, and GapFill still fills an undeclared row.
    #[test]
    fn gapfill_stamp_never_resurrects_over_a_declared_head() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        create_content(&mut conn, &ctx, mk_plain("cid-declared")).unwrap();

        // GapFill on an UNDECLARED row fills it (heal's legitimate job).
        let filled = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-declared",
            "uhCkk-canonical-NEW",
            None,
            None,
            StampMode::GapFill,
            None,
        )
        .unwrap();
        assert_eq!(filled, StampOutcome::Stamped);

        // GapFill with a DIFFERENT (older / fallback) head must skip.
        let skipped = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-declared",
            "uhCkk-superseded-OLD",
            None,
            None,
            StampMode::GapFill,
            None,
        )
        .unwrap();
        assert_eq!(skipped, StampOutcome::SkippedDeclared);
        let row = get_content(&mut conn, &ctx, "cid-declared", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-canonical-NEW"),
            "GapFill must not resurrect a superseded head over the adopted canonical"
        );

        // Same-head GapFill re-stamp is an idempotent REFRESH — it writes, but the
        // HEAD does not move, so it is `Refreshed`, never `Stamped` (convergence).
        let same = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-declared",
            "uhCkk-canonical-NEW",
            None,
            None,
            StampMode::GapFill,
            None,
        )
        .unwrap();
        assert_eq!(same, StampOutcome::Refreshed);

        // Declare mode (canonical channels) still moves the declared head.
        let moved = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-declared",
            "uhCkk-canonical-NEWER",
            None,
            None,
            StampMode::Declare,
            None,
        )
        .unwrap();
        assert_eq!(moved, StampOutcome::Stamped);
    }

    /// HEAD-election (v) — the stale-canonical resurrection guard (2026-07-12
    /// live regression: elohim-host-landing converged at edge #1187's
    /// seam-smoke, healed BACKWARDS to the superseded head by #1188): a
    /// HealCanonical stamp may fill, refresh, or move FORWARD, but must never
    /// move a declared row backwards.
    ///
    /// RE-AUTHORED 2026-08-02. The ordering compared is now the DHT ELECTION
    /// clock (`canonical_ordering`, arg 8) rather than `declared_at` (arg 5).
    /// The guarded property is unchanged and still asserted below; what changed
    /// is that the comparison is now between two values that are the SAME on
    /// every peer, instead of between head-action timestamps that are not.
    #[test]
    fn heal_canonical_stamp_is_monotonic() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        create_content(&mut conn, &ctx, mk_plain("cid-mono")).unwrap();

        // Fill an undeclared row (heal's legitimate job) — records the election.
        let filled = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-old",
            Some(1_000),
            None,
            StampMode::HealCanonical,
            Some((1_000, false)),
        )
        .unwrap();
        assert_eq!(filled, StampOutcome::Stamped);

        // A NEWER election (the steward re-declared; the link gossiped in).
        let newer = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-new",
            Some(2_000),
            None,
            StampMode::HealCanonical,
            Some((2_000, false)),
        )
        .unwrap();
        assert_eq!(newer, StampOutcome::Stamped);

        // THE REGRESSION: a conductor that has not integrated the newer
        // canonical link answers with the OLD canonical record — canonical, yet
        // stale. Its election clock is older, so the move is refused.
        let stale = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-old",
            Some(1_000),
            None,
            StampMode::HealCanonical,
            Some((1_000, false)),
        )
        .unwrap();
        assert_eq!(
            stale,
            StampOutcome::SkippedStale,
            "stale canonical answer must never move a declared head backwards"
        );
        let row = get_content(&mut conn, &ctx, "cid-mono", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(row.declared_head_action_hash.as_deref(), Some("uhCkk-new"));

        // An answer carrying NO election cannot displace a row that carries one:
        // an un-elected claim never outranks an elected declaration.
        let unelected = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-old",
            None,
            None,
            StampMode::HealCanonical,
            None,
        )
        .unwrap();
        assert_eq!(unelected, StampOutcome::SkippedStale);

        // Equal election clocks are NOT a move — the same election cannot elect
        // two different heads, so this is evidence of confusion, not progress.
        let tied = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-other",
            Some(9_999),
            None,
            StampMode::HealCanonical,
            Some((2_000, false)),
        )
        .unwrap();
        assert_eq!(tied, StampOutcome::SkippedStale);

        // Same-head refresh is idempotent, never a skip — and never `Stamped`:
        // the HEAD did not move, so it reports `Refreshed` (no convergence).
        let same = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-new",
            Some(2_000),
            None,
            StampMode::HealCanonical,
            Some((2_000, false)),
        )
        .unwrap();
        assert_eq!(same, StampOutcome::Refreshed);

        // A newer election moves the row forward — exactly how a peer converges
        // once the winning declaration link gossips in.
        let forward = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-newest",
            Some(3_000),
            None,
            StampMode::HealCanonical,
            Some((3_000, false)),
        )
        .unwrap();
        assert_eq!(forward, StampOutcome::Stamped);
        let row = get_content(&mut conn, &ctx, "cid-mono", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-newest")
        );

        // TIER precedence: an EARNED election beats a staging one outright, even
        // with an OLDER clock — mirrors `select_canonical_winner` rule 1.
        let earned = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-earned",
            Some(500),
            None,
            StampMode::HealCanonical,
            Some((500, true)),
        )
        .unwrap();
        assert_eq!(
            earned,
            StampOutcome::Stamped,
            "earned tier must win regardless of staging recency"
        );

        // …and staging must never displace it, however new.
        let staging_over_earned = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-mono",
            "uhCkk-newest",
            Some(9_000),
            None,
            StampMode::HealCanonical,
            Some((9_000, false)),
        )
        .unwrap();
        assert_eq!(
            staging_over_earned,
            StampOutcome::SkippedStale,
            "the scaffold tier never overrides an earned canonical"
        );
    }

    /// THE CURE (2026-08-02) — the two-way-declared class. A row declared by a
    /// channel that records NO election (the deploy PATCH's
    /// `HeadElection::Declare`, or a timestamp-less `ContentHeadDeclared`
    /// signal) MUST be movable by a DHT-elected canonical answer.
    ///
    /// Under the previous rule this refused forever: the guard required a
    /// non-NULL stored ordering and every such channel NULLs it, so ~11.4k rows
    /// fleet-wide were elected-but-unconsumable. Moving here is forward in
    /// AUTHORITY (notarized election over un-elected self-declaration), which is
    /// what "key it on the authority the write CARRIES" means.
    #[test]
    fn an_elected_answer_moves_a_row_whose_declaration_has_no_election() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        create_content(&mut conn, &ctx, mk_plain("cid-unelected")).unwrap();

        // The deploy PATCH shape: a declare channel crowns a head and records no
        // election at all.
        assert!(stamp_declared_head(
            &mut conn,
            &ctx,
            "cid-unelected",
            "uhCkk-self-declared",
            None,
            None
        )
        .unwrap());
        let row = get_content(&mut conn, &ctx, "cid-unelected", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert!(
            row.canonical_declared_at.is_none(),
            "precondition: the declare channel records no election"
        );

        // A canonical answer carrying a real election now moves it.
        let moved = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-unelected",
            "uhCkk-elected",
            Some(42),
            None,
            StampMode::HealCanonical,
            Some((7_000, false)),
        )
        .unwrap();
        assert_eq!(
            moved,
            StampOutcome::Stamped,
            "a DHT-elected head must be consumable by an already-declared row — \
             refusing here is the live stalemate this fix exists to break"
        );

        let row = get_content(&mut conn, &ctx, "cid-unelected", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-elected")
        );
        assert_eq!(
            row.canonical_declared_at,
            Some(7_000),
            "the election clock must persist, or the row stays contestable and \
             re-mints a link every sweep (the quiescence gate reads this column)"
        );

        // QUIESCENCE: now that an election stands behind the row, an answer
        // carrying no election cannot move it back.
        let back = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            "cid-unelected",
            "uhCkk-self-declared",
            None,
            None,
            StampMode::HealCanonical,
            None,
        )
        .unwrap();
        assert_eq!(back, StampOutcome::SkippedStale);
    }

    /// The pure rule, exhaustively — every ordered pair of (election, no
    /// election) × (earned, staging) resolves without ambiguity, and the two
    /// directions are never both moves (which would be a flap).
    #[test]
    fn canonical_move_verdict_is_antisymmetric_and_total() {
        let cases = [
            (Some((10, false)), None, true),
            (None, Some((10, false)), false),
            (Some((10, true)), Some((99, false)), true),
            (Some((99, false)), Some((10, true)), false),
            (Some((20, false)), Some((10, false)), true),
            (Some((10, false)), Some((20, false)), false),
            (Some((10, false)), Some((10, false)), false),
            (None, None, false),
        ];
        for (incoming, stored, expect_move) in cases {
            assert_eq!(
                canonical_move_verdict(incoming, stored).is_ok(),
                expect_move,
                "verdict for incoming={incoming:?} stored={stored:?}"
            );
        }

        // ANTISYMMETRY: for any two DIFFERENT elections, at most one direction
        // moves. If both moved, two peers would swap heads forever.
        let elections = [(10, false), (20, false), (10, true), (20, true)];
        for a in elections {
            for b in elections {
                if a == b {
                    continue;
                }
                let ab = canonical_move_verdict(Some(a), Some(b)).is_ok();
                let ba = canonical_move_verdict(Some(b), Some(a)).is_ok();
                assert!(!(ab && ba), "flap: {a:?} and {b:?} each displace the other");
            }
        }
    }

    /// Refusal reasons are distinct and stable — they are metric label values,
    /// and conflating them re-creates exactly the ambiguity ("no election
    /// anywhere" vs "lost the comparison") that made this class undiagnosable.
    #[test]
    fn stale_reasons_are_distinct_and_labelled() {
        assert_eq!(
            canonical_move_verdict(None, Some((1, false))).unwrap_err(),
            StaleReason::StoredNull
        );
        assert_eq!(
            canonical_move_verdict(Some((1, false)), Some((2, false))).unwrap_err(),
            StaleReason::NotNewer
        );
        assert_eq!(
            canonical_move_verdict(Some((9, false)), Some((1, true))).unwrap_err(),
            StaleReason::Tier
        );
        let labels = [
            StaleReason::StoredNull.label(),
            StaleReason::NotNewer.label(),
            StaleReason::Tier.label(),
        ];
        assert_eq!(
            labels
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            3,
            "metric label values must be distinct"
        );
    }

    /// The SPIN signature (live 2026-07-26): when two peers hold genuinely
    /// different roots for the same id, each peer's own conductor keeps answering
    /// with ITS OWN head — the head the row already holds. The stamp still writes
    /// (patch refresh + ordering backfill) and `updated_at` advances, but the HEAD
    /// never moves and the cross-peer divergence is untouched.
    ///
    /// This MUST report `Refreshed`, never `Stamped`. Conflating them is what let
    /// matthew log 1578 `HEALED content anchor` lines in 12h while
    /// `elohim-host-landing` sat on an unchanged `uhCkkh_Gb…` and elohim.host sat
    /// on a different root — a spinning heal plane that read as a working one.
    #[test]
    fn same_head_restamp_is_refreshed_not_stamped_so_spin_is_legible() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let id = "epr:two-roots";
        create_content(&mut conn, &ctx, mk_bundle(id)).unwrap();

        // This peer adopts its own root (the fill — real convergence from empty).
        let filled = stamp_declared_head_mode(
            &mut conn,
            &ctx,
            id,
            "uhCkk-own-root",
            Some(1_000),
            None,
            StampMode::HealCanonical,
            None,
        )
        .unwrap();
        assert_eq!(
            filled,
            StampOutcome::Stamped,
            "filling an empty row converges"
        );

        // Every subsequent sweep: the peer advertises a DIFFERENT root, so this row
        // is re-enqueued as divergent — but the own conductor answers `uhCkk-own-root`
        // again. The write happens; the head does not move.
        for sweep in 0..3 {
            let spun = stamp_declared_head_mode(
                &mut conn,
                &ctx,
                id,
                "uhCkk-own-root",
                Some(1_000),
                None,
                StampMode::HealCanonical,
                None,
            )
            .unwrap();
            assert_eq!(
                spun,
                StampOutcome::Refreshed,
                "sweep {sweep}: a same-head re-stamp is a no-op refresh, NOT a heal"
            );
        }

        // The head is exactly where it started — nothing converged.
        let row = get_content(&mut conn, &ctx, id, MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-own-root"),
            "the peer's divergent root was never adopted — only a canonical channel can do that"
        );

        // The legacy bool wrapper still reports `true` (there WAS a row and we
        // wrote it) — the Stamped/Refreshed split must not change that contract.
        assert!(
            stamp_declared_head(&mut conn, &ctx, id, "uhCkk-own-root", Some(1_000), None).unwrap(),
            "same-head refresh still counts as 'a row was written' for the bool callers"
        );
    }

    /// HEAD-election (iii): a verified `stamp_declared_head` carrying a patch
    /// overwrites an amber row's value fields (green-over-amber preserved) and
    /// promotes the row to notarized.
    #[test]
    fn stamp_declared_head_overwrites_amber_value_fields() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        let id = "epr:amber-then-stamped";

        // Amber row: converged blob_hash, no anchor, no declared head.
        create_content(&mut conn, &ctx, mk_bundle(id)).unwrap();
        update_content(
            &mut conn,
            &ctx,
            UpdateContentInput {
                id: id.to_string(),
                blob_hash: Some("sha256-amberbundle".to_string()),
                crdt_converged_at: Some(chrono::Utc::now().to_rfc3339()),
                ..Default::default()
            },
        )
        .unwrap();

        let pre = get_content(&mut conn, &ctx, id, MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(pre.blob_hash.as_deref(), Some("sha256-amberbundle"));
        assert!(pre.dht_anchor_hash.is_none(), "amber row is not notarized");
        assert!(pre.declared_head_action_hash.is_none());

        // Verified stamp carries a green blob_cid — green overwrites amber value fields.
        let stamped = stamp_declared_head(
            &mut conn,
            &ctx,
            id,
            "uhCkk-verified-head",
            None,
            Some(ContentProjectionPatch {
                blob_cid: Some("sha256-greenbundle".to_string()),
                ..Default::default()
            }),
        )
        .unwrap();
        assert!(stamped);

        let row = get_content(&mut conn, &ctx, id, MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(
            row.blob_hash.as_deref(),
            Some("sha256-greenbundle"),
            "verified (green) stamp overwrites the amber blob_hash value field"
        );
        assert_eq!(row.blob_cid.as_deref(), Some("sha256-greenbundle"));
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-verified-head")
        );
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkk-verified-head"),
            "stamp promotes the row to notarized (green)"
        );
        assert!(
            row.crdt_converged_at.is_some(),
            "amber marker is harmless residue — never cleared by the stamp"
        );
    }
}
