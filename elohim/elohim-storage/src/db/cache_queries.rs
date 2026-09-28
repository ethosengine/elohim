//! Cache-eligible content queries for the cache stream endpoint.
//!
//! These queries return content filtered by reach level — only commons/public
//! content is eligible for projection cache warm-up.

use diesel::prelude::*;

use super::context::{AppContext, HUMANS_HAPP_ID};
use super::diesel_schema::{content, content_tags, humans, relationships};
use super::models::{Content, ContentWithTags, Human, Relationship};
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

/// One page of [`list_cacheable_content`] WITH each row's tags — what the
/// cache stream sends, so the doorway's boot-warmed and swept cache documents
/// carry the same `tags` the `/db/content/{id}` read serves (the lamad app reads
/// them). A bare `Content` row carries no tags; `From<Content> for ContentView`
/// leaves them empty.
///
/// Two queries per PAGE, never one per row: the page, then every tag of the
/// page's ids in one `IN (…)` read (a page is at most `limit` ids — the stream
/// pages at 500, well under SQLite's bound-variable limit). Tags come back in
/// primary-key order `(h_app_id, content_id, tag)`, the order the single-row
/// read's index scan yields, so a streamed row and a read row agree.
pub fn list_cacheable_content_with_tags(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<ContentWithTags>, StorageError> {
    let rows = list_cacheable_content(conn, ctx, limit, offset)?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    let pairs: Vec<(String, String)> = content_tags::table
        .filter(content_tags::h_app_id.eq(&ctx.h_app_id))
        .filter(content_tags::content_id.eq_any(&ids))
        .order((content_tags::content_id.asc(), content_tags::tag.asc()))
        .select((content_tags::content_id, content_tags::tag))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable content tags query failed: {e}")))?;
    let mut by_id: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::with_capacity(rows.len());
    for (content_id, tag) in pairs {
        by_id.entry(content_id).or_default().push(tag);
    }
    Ok(rows
        .into_iter()
        .map(|content| {
            let tags = by_id.remove(&content.id).unwrap_or_default();
            ContentWithTags { content, tags }
        })
        .collect())
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
    use crate::db::content_diesel::{create_content, CreateContentInput};
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

    fn cacheable(id: &str, reach: &str, tags: &[&str]) -> CreateContentInput {
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
            reach: reach.into(),
            created_by: None,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            content_body: None,
            dht_anchor_hash: None,
        }
    }

    /// Streamed cache rows carry the same tags the `/db/content/{id}` read
    /// serves — per row, per page, in the read's order — and a row without
    /// tags streams `[]`, never a neighbour's.
    #[test]
    fn cacheable_content_pages_carry_each_rows_tags() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = crate::db::AppContext::new("lamad");
        create_content(
            &mut conn,
            &ctx,
            cacheable("a", "commons", &["zeta", "alpha"]),
        )
        .unwrap();
        create_content(&mut conn, &ctx, cacheable("b", "public", &[])).unwrap();
        create_content(&mut conn, &ctx, cacheable("c", "commons", &["only"])).unwrap();
        create_content(&mut conn, &ctx, cacheable("hidden", "private", &["secret"])).unwrap();
        // Another app scope's rows and tags never leak into this one.
        let other = crate::db::AppContext::new("elohim");
        create_content(
            &mut conn,
            &other,
            cacheable("x", "commons", &["other-scope"]),
        )
        .unwrap();

        let page = super::list_cacheable_content_with_tags(&mut conn, &ctx, 500, 0).unwrap();
        let mut got: Vec<(String, Vec<String>)> = page
            .iter()
            .map(|row| (row.content.id.clone(), row.tags.clone()))
            .collect();
        got.sort();
        assert_eq!(
            got,
            vec![
                (
                    "a".to_string(),
                    vec!["alpha".to_string(), "zeta".to_string()]
                ),
                ("b".to_string(), vec![]),
                ("c".to_string(), vec!["only".to_string()]),
            ]
        );
        for row in &page {
            let read =
                crate::db::content_diesel::get_content_tags(&mut conn, &ctx, &row.content.id)
                    .unwrap();
            assert_eq!(
                row.tags, read,
                "stream and read agree for {}",
                row.content.id
            );
        }

        // Paging still pages: tags follow their row onto the second page.
        let first = super::list_cacheable_content_with_tags(&mut conn, &ctx, 2, 0).unwrap();
        let second = super::list_cacheable_content_with_tags(&mut conn, &ctx, 2, 2).unwrap();
        assert_eq!(first.len() + second.len(), 3);
        assert!(
            super::list_cacheable_content_with_tags(&mut conn, &ctx, 2, 4)
                .unwrap()
                .is_empty()
        );
    }
}
