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
/// HEAD-election rule (author-only auto-declare, Plan C3): every `ContentCommitted`
/// also advances `declared_head_action_hash` to the committed action. In the
/// single-author model the author's latest commit IS the declared HEAD, and this
/// path fires ONLY for own-conductor-witnessed commits (the authoring conductor
/// emits `ContentCommitted` only for locally-authored commits), so writing the
/// notary-declared HEAD here is authorized by construction. Thus both
/// `dht_anchor_hash` AND `declared_head_action_hash` are set to the passed action.
/// The reconcile leg's verified-stamp entrypoint is [`stamp_declared_head`]; the
/// HEAD is NEVER written from CRDT/gossip input.
pub fn upsert_with_anchor(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    patch: ContentProjectionPatch,
    dht_anchor_hash: &str,
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
        diesel::update(
            content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .filter(content::id.eq(id)),
        )
        .set((
            content::dht_anchor_hash.eq(dht_anchor_hash),
            // HEAD-election rule: an own-conductor-witnessed commit advances the
            // notary-declared HEAD to the committed action (see the invariant doc
            // above). Author-only auto-declare — set alongside the anchor.
            content::declared_head_action_hash.eq(dht_anchor_hash),
            content::updated_at.eq(sql::<Text>("CURRENT_TIMESTAMP")),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update anchor failed: {}", e)))?;

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

        // Set anchor + notary-declared HEAD on the just-inserted row (author-only
        // auto-declare — this defensive insert path is still an own-conductor
        // ContentCommitted projection, so the HEAD election applies).
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
        .map_err(|e| StorageError::Internal(format!("Set anchor on insert failed: {}", e)))?;
    }

    Ok(())
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
    patch: Option<ContentProjectionPatch>,
) -> Result<bool, StorageError> {
    use diesel::dsl::sql;
    use diesel::sql_types::Text;

    let existing = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::id.eq(id))
        .select(diesel::dsl::count_star())
        .first::<i64>(conn)
        .map(|c| c > 0)
        .unwrap_or(false);

    if !existing {
        return Ok(false);
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

    if let Some(patch) = patch {
        apply_content_patch_fields(conn, ctx, id, &patch)?;
    }

    Ok(true)
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
/// Ordered by `id` ascending (deterministic), capped at `cap` (LIMIT).
///
/// **Discovery-only (the notary invariant).** A peer consuming this inventory
/// learns WHICH content ids have a DHT anchor somewhere; the anchor VALUE it
/// writes into its own projection comes EXCLUSIVELY from its OWN conductor
/// (`content_store::resolve_content_head`), never from the advertised pair. Peer
/// bytes are never laundered into notary provenance — the same P1 discipline as
/// `rea_commitments::inventory_for_reconcile`.
pub fn list_content_anchor_inventory(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    cap: i64,
) -> Result<Vec<(String, String)>, StorageError> {
    let rows: Vec<(String, Option<String>)> = content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::dht_anchor_hash.is_not_null())
        .filter(content::reach.eq_any(DISTRIBUTION_SAFE_REACH))
        .order(content::id.asc())
        .limit(cap)
        .select((content::id, content::dht_anchor_hash))
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("content anchor inventory load failed: {e}"))
        })?;

    // `IS NOT NULL` guarantees `Some`; `filter_map` discards defensively rather
    // than unwrap. An empty-string anchor (`""`, distinct from NULL) is admitted
    // as-is — the consumer's diff treats an empty peer anchor as non-divergence.
    Ok(rows
        .into_iter()
        .filter_map(|(id, anchor)| anchor.map(|a| (id, a)))
        .collect())
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
/// Returns `(id, reach)` for each NULL-anchor row so the re-anchor sweep can
/// skip rows whose stored reach is non-canonical (the conductor would reject
/// them on every sweep). Only the reanchor_backfill service calls this.
pub fn list_unanchored_content_ids(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
) -> Result<Vec<(String, String)>, StorageError> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::dht_anchor_hash.is_null())
        .select((content::id, content::reach))
        .order((content::created_at.asc(), content::id.asc()))
        .limit(limit)
        .load::<(String, String)>(conn)
        .map_err(|e| StorageError::Internal(format!("list_unanchored_content_ids failed: {}", e)))
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

        // Distribution-safe + anchored → INCLUDED.
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

        let inv = list_content_anchor_inventory(&mut conn, &ctx, i64::MAX).unwrap();
        let ids: Vec<&str> = inv.iter().map(|(id, _)| id.as_str()).collect();

        // Exactly the three anchored distribution-safe rows, id-ascending.
        assert_eq!(
            ids,
            vec!["c:commons", "c:community", "c:public"],
            "only anchored distribution-safe rows, ordered by id asc"
        );
        // Anchor value carried through verbatim (discovery pair).
        let map: std::collections::HashMap<String, String> = inv.into_iter().collect();
        assert_eq!(map.get("c:public").map(String::as_str), Some("anc-public"));
        // Scoped-tier ids must NEVER appear in a cross-peer inventory.
        assert!(!map.contains_key("c:private"));
        assert!(!map.contains_key("c:self"));
        assert!(!map.contains_key("c:trusted"));
        // Un-anchored distribution-safe id is excluded (no provenance).
        assert!(!map.contains_key("c:public-noanchor"));

        // Cap is respected (LIMIT).
        let capped = list_content_anchor_inventory(&mut conn, &ctx, 2).unwrap();
        assert_eq!(capped.len(), 2, "cap limits the returned rows");
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

    /// A3 proof (RED before A3): a seeded SPA-bundle row with no blob_hash and no
    /// conductor bridge 404s at the slug lookup today. The deploy-producer amber
    /// write (marker set, `(true, None)` branch) records `blob_hash` +
    /// `crdt_converged_at` diesel-direct, NEVER `dht_anchor_hash`. The row then
    /// resolves through the SAME query `lookup_slug_blob_hash` runs
    /// (MinTrust::Amber) — so the SPA mount serves 200. Authority reads (Green)
    /// still exclude it (amber is never authoritative).
    #[test]
    fn amber_patch_no_conductor_serves_200() {
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

        // The amber write — exactly what ContentService::update_amber does: set
        // blob_hash + stamp crdt_converged_at, never dht_anchor_hash.
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

    /// HEAD-election (i): `upsert_with_anchor` (the own-conductor `ContentCommitted`
    /// projection) now advances `declared_head_action_hash` to the committed action
    /// alongside `dht_anchor_hash` — author-only auto-declare.
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
        )
        .unwrap();

        let row = get_content(&mut conn, &ctx, "cid-head", MinTrust::Invisible)
            .unwrap()
            .unwrap();
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkk-head-action-1"));
        assert_eq!(
            row.declared_head_action_hash.as_deref(),
            Some("uhCkk-head-action-1"),
            "upsert_with_anchor must advance declared_head_action_hash to the committed action"
        );
        assert_eq!(row.title, "Head v2", "patch value field still applied");
    }

    /// HEAD-election (ii): `stamp_declared_head` stamps an EXISTING row (both hashes)
    /// and returns `Ok(false)` (no insert) for a missing row; re-stamp is idempotent.
    #[test]
    fn stamp_declared_head_stamps_existing_and_false_for_missing() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Missing row → Ok(false), no fabrication.
        let missing = stamp_declared_head(&mut conn, &ctx, "cid-absent", "uhCkk-x", None).unwrap();
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
            stamp_declared_head(&mut conn, &ctx, "cid-present", "uhCkk-head-9", None).unwrap();
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
            stamp_declared_head(&mut conn, &ctx, "cid-present", "uhCkk-head-9", None).unwrap();
        assert!(restamped, "idempotent re-stamp returns Ok(true)");
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
