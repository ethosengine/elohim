//! Phase 4 T5 — tests for imagodei_lookup::resolve_display_name.

use diesel::prelude::*;
use elohim_storage::services::imagodei_lookup::resolve_display_name;
use elohim_storage::test_util::test_pool;

fn insert_human(conn: &mut diesel::SqliteConnection, id: &str, display_name: &str) {
    use elohim_storage::db::diesel_schema::humans::dsl as h;
    diesel::insert_or_ignore_into(h::humans)
        .values((
            h::id.eq(id),
            h::display_name.eq(display_name),
            h::affinities.eq("[]"),
            h::profile_reach.eq("private"),
            h::h_app_id.eq("lamad"),
            h::created_at.eq("2026-05-11T00:00:00Z"),
            h::updated_at.eq("2026-05-11T00:00:00Z"),
        ))
        .execute(conn)
        .expect("insert human");
}

#[tokio::test]
async fn resolves_display_name_for_known_agent() {
    let pool = test_pool();
    {
        let mut conn = pool.get().expect("conn");
        insert_human(&mut conn, "agent-matthew", "Matthew Manager");
    }

    let name = resolve_display_name(&pool, "agent-matthew").await;
    assert_eq!(name, Some("Matthew Manager".to_string()));
}

#[tokio::test]
async fn returns_none_for_unknown_agent() {
    let pool = test_pool();
    let name = resolve_display_name(&pool, "agent-unknown").await;
    assert_eq!(name, None);
}
