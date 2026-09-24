//! `GET /db/content` serves each row's tags (plan Lane S, ruling R-S5).
//!
//! The list route used to convert `ContentWithTags -> ContentView` through
//! `c.content.into()`, discarding the tags `list_content` had just loaded — so
//! every lamad client read `tags: undefined`, and tag scoring plus the byTag
//! facet ran on empty arrays. Reverting the conversion to drop tags makes this
//! test RED.

use std::sync::Arc;

use elohim_storage::db::content_diesel::{create_content, CreateContentInput};
use elohim_storage::db::AppContext;
use elohim_storage::http::HttpServer;
use elohim_storage::test_util::test_pool;

#[tokio::test]
async fn db_content_list_serves_tags_per_row() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        create_content(
            &mut conn,
            &AppContext::default_lamad(),
            CreateContentInput {
                id: "tagged-concept".into(),
                title: "Tagged concept".into(),
                description: None,
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "commons".into(),
                created_by: None,
                // Two content_tags rows land beside the content row.
                tags: vec!["a".into(), "b".into()],
                content_body: Some("body".into()),
                // An ingest anchor puts the row above the serving floor
                // (MinTrust::Amber) that external `/db/content` reads apply.
                dht_anchor_hash: Some("uhCkk-tagged-concept-anchor".into()),
            },
        )
        .expect("seed one tagged content row");
    }

    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap(),
    );
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool);

    let resp = server.test_list_db_content("", None).await;
    assert_eq!(resp.status, 200, "GET /db/content must return 200");

    let body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(
        body["count"].as_u64(),
        Some(1),
        "the seeded row is listed: {body}"
    );
    let mut tags: Vec<String> = serde_json::from_value(body["items"][0]["tags"].clone())
        .unwrap_or_else(|e| panic!("items[0].tags must be a string array ({e}): {body}"));
    tags.sort();
    assert_eq!(tags, vec!["a".to_string(), "b".to_string()]);
}
