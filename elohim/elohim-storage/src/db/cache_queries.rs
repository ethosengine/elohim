//! Cache-eligible content queries for the cache stream endpoint.
//!
//! These queries return content filtered by reach level — only commons/public
//! content is eligible for projection cache warm-up.

use diesel::prelude::*;

use super::context::AppContext;
use super::diesel_schema::{content, humans, paths, relationships};
use super::models::{Content, Human, Path, Relationship};
use crate::error::StorageError;

/// List content with reach = 'commons' (cacheable for projection)
pub fn list_cacheable_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Content>, StorageError> {
    content::table
        .filter(content::app_id.eq(&ctx.app_id))
        .filter(content::reach.eq("commons"))
        .order(content::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable content query failed: {e}")))
}

/// List all paths (all are public per cache rules)
pub fn list_cacheable_paths(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Path>, StorageError> {
    paths::table
        .filter(paths::app_id.eq(&ctx.app_id))
        .order(paths::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable paths query failed: {e}")))
}

/// List humans with profile_reach = 'public'
pub fn list_cacheable_humans(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Human>, StorageError> {
    humans::table
        .filter(humans::app_id.eq(&ctx.app_id))
        .filter(humans::profile_reach.eq("public"))
        .order(humans::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable humans query failed: {e}")))
}

/// List relationships with reach = 'commons'
pub fn list_cacheable_relationships(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Relationship>, StorageError> {
    relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .filter(relationships::reach.eq("commons"))
        .order(relationships::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable relationships query failed: {e}")))
}
