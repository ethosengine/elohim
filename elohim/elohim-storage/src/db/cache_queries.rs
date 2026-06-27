//! Cache-eligible content queries for the cache stream endpoint.
//!
//! These queries return content filtered by reach level — only commons/public
//! content is eligible for projection cache warm-up.

use diesel::prelude::*;

use super::context::{AppContext, HUMANS_HAPP_ID};
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
    _ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Human>, StorageError> {
    // Humans are scoped to the canonical identity (imagodei) scope, NOT the
    // operating ctx — public humans must be cacheable from the content context.
    humans::table
        .filter(humans::h_app_id.eq(HUMANS_HAPP_ID))
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

#[cfg(test)]
mod tests {
    use crate::db::context::HUMANS_HAPP_ID;
    use crate::db::diesel_schema::humans;
    use crate::db::models::NewHuman;
    use diesel::prelude::*;

    /// Public humans live under the imagodei scope, but the doorway projection
    /// cache operates in the content (lamad) context. `list_cacheable_humans` must
    /// read humans by the canonical scope, NOT the operating ctx, or the public
    /// humans cache is empty. RED before the fix (filter was `&ctx.h_app_id`).
    #[test]
    fn list_cacheable_humans_finds_imagodei_public_humans() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        diesel::insert_into(humans::table)
            .values(&NewHuman {
                id: "h-1".into(),
                agent_pub_key: Some("uhCAk-a".into()),
                display_name: "A".into(),
                bio: None,
                affinities: "[]".into(),
                profile_reach: "public".into(),
                location: None,
                profile_photo_url: None,
                h_app_id: HUMANS_HAPP_ID.into(),
                household_id: None,
            })
            .execute(&mut conn)
            .unwrap();
        let ctx = crate::db::AppContext::new("lamad");
        let rows = super::list_cacheable_humans(&mut conn, &ctx, 100, 0).unwrap();
        assert_eq!(
            rows.len(),
            1,
            "public imagodei humans must be cacheable from the lamad context"
        );
    }
}
