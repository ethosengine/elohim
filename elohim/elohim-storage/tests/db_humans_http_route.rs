//! HTTP-routing test for `GET /db/humans` (humans-projection scope reconciliation).
//!
//! The handler must list the imagodei identity projection REGARDLESS of the
//! operating (content) scope the router passes. This is the read-scope artifact
//! that misled the 2026-06-19 dark-card probe (`/db/humans` returned empty under
//! the `lamad` ctx despite a populated, imagodei-scoped table). Reverting
//! `handle_list_humans_inner` to read `ctx.h_app_id` makes this test RED.

use std::sync::Arc;

use diesel::prelude::*;
use diesel::RunQueryDsl;

use elohim_storage::db::diesel_schema::humans;
use elohim_storage::db::models::NewHuman;
use elohim_storage::db::HUMANS_HAPP_ID;
use elohim_storage::http::HttpServer;
use elohim_storage::test_util::test_pool;

#[tokio::test]
async fn db_humans_lists_imagodei_under_a_lamad_operating_ctx() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        // A human under the imagodei (identity) scope — the production write scope.
        diesel::insert_into(humans::table)
            .values(&NewHuman {
                id: "h-ada".into(),
                agent_pub_key: Some("uhCAk-ada".into()),
                display_name: "Ada".into(),
                bio: None,
                affinities: "[]".into(),
                profile_reach: "commons".into(),
                location: None,
                profile_photo_url: None,
                h_app_id: HUMANS_HAPP_ID.into(),
                household_id: Some("hh-1".into()),
            })
            .execute(&mut conn)
            .unwrap();
    }

    // Minimal HttpServer with the DB pool (no blob store needed for /db routes).
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap(),
    );
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool);

    // The router passes the operating CONTENT scope ("lamad"); the handler must
    // still list the imagodei humans projection.
    let resp = server.test_list_db_humans("lamad").await;
    assert_eq!(resp.status, 200, "GET /db/humans must return 200");

    let body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(
        body["count"].as_u64(),
        Some(1),
        "GET /db/humans must list the imagodei human even under a lamad operating ctx \
         (read-scope fix); reverting the handler to ctx.h_app_id yields 0 here"
    );
    assert_eq!(
        body["items"].as_array().map(|a| a.len()),
        Some(1),
        "exactly the one seeded imagodei human is listed"
    );
}
