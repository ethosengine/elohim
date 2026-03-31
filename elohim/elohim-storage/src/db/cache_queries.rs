//! Cache-eligible content queries for the cache stream endpoint.
//!
//! These queries return content filtered by reach level — only commons/public
//! content is eligible for projection cache warm-up.

use diesel::prelude::*;

use super::context::AppContext;
use super::diesel_schema::{content, humans, relationships};
use super::models::{Content, Human, Relationship};
use crate::error::StorageError;

/// List content with reach = 'commons' or 'public' (cacheable for projection).
/// This now includes paths (contentType = 'path') since paths are content rows.
pub fn list_cacheable_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Content>, StorageError> {
    content::table
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .filter(content::reach.eq_any(["commons", "public"]))
        .order(content::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable content query failed: {e}")))
}

/// List humans with profile_reach = 'public'
pub fn list_cacheable_humans(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Human>, StorageError> {
    humans::table
        .filter(humans::h_app_id.eq(&ctx.h_app_id))
        .filter(humans::profile_reach.eq("public"))
        .order(humans::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable humans query failed: {e}")))
}

/// List relationships with reach = 'commons' or 'public'
pub fn list_cacheable_relationships(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Relationship>, StorageError> {
    relationships::table
        .filter(relationships::h_app_id.eq(&ctx.h_app_id))
        .filter(relationships::reach.eq_any(["commons", "public"]))
        .order(relationships::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable relationships query failed: {e}")))
}
