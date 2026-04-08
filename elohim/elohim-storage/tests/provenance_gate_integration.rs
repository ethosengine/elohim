//! Integration test: HTTP list_content must not return rows that have
//! neither dht_anchor_hash nor p2p_published_at set.

use diesel::{Connection, RunQueryDsl, SqliteConnection};
use elohim_storage::db::content_diesel::{
    create_content, list_content, ContentQuery, CreateContentInput,
};
use elohim_storage::db::context::AppContext;

fn test_conn() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    // Operational test fixture. Source of truth for the real schema is the
    // migration file; this in-memory mirror replicates the essential columns
    // (including the dht_anchor_hash DHT anchor column and the new
    // p2p_published_at operational projection column) so the gate filter
    // can be exercised without a full DB.
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
    .unwrap();
    // Operational projection fixture for content_tags — source of truth is
    // the content_tags Links hanging off the parent Content DHT anchor
    // (no standalone entry type; see Category A2 in the p2p-design-gate skill).
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
    .unwrap();
    conn
}

#[test]
fn unpublished_content_is_invisible_to_external_reads() {
    let mut conn = test_conn();
    let ctx = AppContext::new("lamad");
    create_content(
        &mut conn,
        &ctx,
        CreateContentInput {
            id: "cid-unpublished".into(),
            title: "Unpublished".into(),
            description: None,
            content_type: "concept".into(),
            content_format: "markdown".into(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".into(),
            created_by: None,
            content_body: None,
            tags: vec![],
        },
    )
    .unwrap();

    let external = list_content(
        &mut conn,
        &ctx,
        &ContentQuery {
            limit: 10,
            offset: 0,
            require_provenance: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        external.is_empty(),
        "external reads must not return unpublished content"
    );

    let internal = list_content(
        &mut conn,
        &ctx,
        &ContentQuery {
            limit: 10,
            offset: 0,
            require_provenance: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(internal.len(), 1, "internal reads must still see the row");
}
