//! Integration tests for `GET /db/rea_commitments?action=project-epr&doorwayId=X`.
//!
//! ## Bug being fixed
//!
//! `doorway/doorway-service/src/projection/epr_router.rs` fetches
//! `GET {storage}/db/rea_commitments?action=project-epr&doorwayId={id}` on
//! startup to populate its route table. Before this fix, `handle_db_request`
//! had no `rea_commitments` arm — the request fell through to the
//! `"Unknown database endpoint"` 404, so `replace_all()` was never called,
//! and `/lamad` and `/` returned 404 on every doorway even when the
//! project-epr commitments existed in SQL.
//!
//! ## Testing strategy
//!
//! Consistent with the codebase pattern (`api_placement_gaps.rs`,
//! `resilience_hub.rs`): tests exercise the DB layer directly via
//! `db::rea_commitments::find_active_projections` — the same code path the
//! new HTTP arm delegates to.  An HTTP routing test drives
//! `HttpServer::test_get_db_rea_commitments` (a thin test-helper that invokes
//! `handle_db_request` directly) to prove the routing arm exists and returns
//! 200 rather than 404.

use elohim_storage::db::context::AppContext;
use elohim_storage::db::rea_commitments::{
    create_commitment, find_active_projections, CreateReaCommitmentInput,
};
use elohim_storage::http::HttpServer;
use elohim_storage::test_util::test_pool;
use elohim_views::projection::EprProjectionView;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Seed helper
// ─────────────────────────────────────────────────────────────────────────────

fn seed_lamad_projection(
    conn: &mut elohim_storage::db::PooledConn,
    ctx: &AppContext,
    doorway_short: &str,
    url_path: &str,
) {
    let scope = format!("doorway:{}|epr:lamad-spa", doorway_short);
    let metadata = serde_json::json!({
        "urlPath": url_path,
        "mode": "cached",
        "reach": "commons",
        "baseHref": format!("{}/", url_path.trim_end_matches('/')),
        "entryFile": "index.html",
        "redirectsFrom": [],
        "gateHints": [],
        "deadEnd": false
    });

    create_commitment(
        conn,
        ctx,
        CreateReaCommitmentInput {
            id: Some(format!(
                "lamad-{}-{}",
                doorway_short,
                url_path.replace('/', "_")
            )),
            action: "project-epr".into(),
            provider: "test-steward".into(),
            receiver: "test-operator".into(),
            in_scope_of: Some(scope),
            note: Some("lamad-spa".into()),
            metadata_json: Some(metadata.to_string()),
            ..Default::default()
        },
    )
    .expect("seed project-epr commitment");
}

// ─────────────────────────────────────────────────────────────────────────────
// §1  DB-layer: find_active_projections returns the lamad projection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn db_layer_find_active_projections_returns_seeded_entry() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let ctx = AppContext::default_lamad();

    seed_lamad_projection(&mut conn, &ctx, "alpha-elohim-host", "/lamad");

    let views = find_active_projections(&mut conn, &ctx, "alpha-elohim-host")
        .expect("find_active_projections must succeed");

    assert_eq!(
        views.len(),
        1,
        "expected exactly one projection for alpha-elohim-host, got {}",
        views.len()
    );

    let v: &EprProjectionView = &views[0];
    assert_eq!(v.epr_id, "lamad-spa", "epr_id must be lamad-spa");
    assert_eq!(
        v.doorway_id, "doorway:alpha-elohim-host",
        "doorway_id must be the long form"
    );
    assert_eq!(v.url_path, "/lamad", "url_path must match seeded value");
    assert_eq!(v.reach, "commons");

    // Verify camelCase wire shape
    let json = serde_json::to_string(v).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        val.get("urlPath").is_some(),
        "camelCase urlPath must be present in JSON"
    );
    assert!(
        val.get("doorwayId").is_some(),
        "camelCase doorwayId must be present in JSON"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  HTTP routing: GET /db/rea_commitments?action=project-epr&doorwayId=X
//     must return 200 with the projection (not 404 "Unknown database endpoint")
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn http_get_db_rea_commitments_returns_200_with_projection() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::default_lamad();
        seed_lamad_projection(&mut conn, &ctx, "alpha-elohim-host", "/lamad");
    }

    // Build a minimal HttpServer with the DB pool (no blob store required for /db routes).
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap(),
    );
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool);

    // Drive the route under test via the test helper exposed by HttpServer.
    let resp = server
        .test_get_db_rea_commitments("project-epr", "alpha-elohim-host")
        .await;

    assert_eq!(
        resp.status,
        200,
        "GET /db/rea_commitments?action=project-epr&doorwayId=alpha-elohim-host \
         must return 200 (not 404 Unknown database endpoint); body: {:?}",
        std::str::from_utf8(&resp.body)
    );

    let body: serde_json::Value =
        serde_json::from_slice(&resp.body).expect("response body must be valid JSON");

    let items = body["items"]
        .as_array()
        .expect("response must have 'items' array");

    assert_eq!(
        items.len(),
        1,
        "must return exactly one projection, got {}",
        items.len()
    );

    assert_eq!(
        items[0]["urlPath"].as_str().unwrap_or(""),
        "/lamad",
        "urlPath must match seeded value"
    );
    assert_eq!(
        items[0]["eprId"].as_str().unwrap_or(""),
        "lamad-spa",
        "eprId must be lamad-spa"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Error cases: missing/wrong query params
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn http_get_db_rea_commitments_wrong_action_returns_400() {
    let pool = test_pool();
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap(),
    );
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool);

    let resp = server
        .test_get_db_rea_commitments("wrong-action", "alpha-elohim-host")
        .await;

    assert!(
        resp.status == 400 || resp.status == 422,
        "wrong action param must return 4xx, got {}",
        resp.status
    );
}

#[tokio::test]
async fn http_get_db_rea_commitments_missing_doorway_id_returns_400() {
    let pool = test_pool();
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap(),
    );
    let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool);

    // Pass empty doorwayId
    let resp = server.test_get_db_rea_commitments("project-epr", "").await;

    assert!(
        resp.status == 400 || resp.status == 422,
        "empty doorwayId must return 4xx, got {}",
        resp.status
    );
}
