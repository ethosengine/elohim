//! Integration tests for GET/POST/DELETE /api/v1/pins.
//!
//! ## Airplane-mode property
//!
//! These tests assert that pin CRUD works with NO p2p networking — all routes
//! touch only the local `acquisition_pins` SQLite table (Category B local store).
//! The `pull` field in the list response is null without p2p (correct behaviour
//! per spec §4.4 tri-state contract).
//!
//! ## Tested assertions
//!
//! 1. POST /api/v1/pins {"headRef":"epr:x"} → 201, kind=="item", status=="active".
//! 2. POST /api/v1/pins {"headRef":"epr:x"} again → 201 idempotent, SAME id.
//! 3. POST /api/v1/pins {"headRef":"epr:c","kind":"cluster"} → 501, mentions slice-3.
//! 4. GET /api/v1/pins → 200, exactly one active pin for epr:x, pull field present.
//! 5. DELETE /api/v1/pins/{id} → 200; GET shows that pin status=="removed".

use elohim_storage::http::HttpServer;
use elohim_storage::test_util::test_pool;
use std::sync::Arc;

/// Build a minimal HttpServer backed by a fresh in-memory test pool.
/// No p2p, no blob store required for /api/v1/pins routes.
async fn build_server() -> HttpServer {
    let blob_store = Arc::new(
        elohim_storage::blob_store::BlobStore::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
        )
        .await
        .unwrap(),
    );
    HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(test_pool())
}

// ─────────────────────────────────────────────────────────────────────────────
// §1  POST creates a pin with kind=="item" and status=="active"
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_pin_creates_item_pin_201() {
    let server = build_server().await;
    let resp = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:x"}"#)
        .await;

    assert_eq!(
        resp.status,
        201,
        "POST /api/v1/pins must return 201; body: {:?}",
        std::str::from_utf8(&resp.body)
    );

    let val: serde_json::Value = serde_json::from_slice(&resp.body).expect("valid JSON body");
    assert_eq!(val["kind"], "item", "kind must default to item");
    assert_eq!(val["status"], "active", "status must be active");
    assert_eq!(val["headRef"], "epr:x", "headRef must match");
    assert!(val["id"].as_i64().is_some(), "id must be an integer");
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  Re-posting the same headRef is idempotent — same id returned
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_pin_idempotent_same_id() {
    let server = build_server().await;

    let r1 = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:x"}"#)
        .await;
    let r2 = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:x"}"#)
        .await;

    assert!(
        r1.status == 201 || r1.status == 200,
        "first POST must return 2xx"
    );
    assert!(
        r2.status == 201 || r2.status == 200,
        "second POST must return 2xx"
    );

    let v1: serde_json::Value = serde_json::from_slice(&r1.body).unwrap();
    let v2: serde_json::Value = serde_json::from_slice(&r2.body).unwrap();

    assert_eq!(
        v1["id"], v2["id"],
        "idempotent re-pin must return the SAME database id"
    );
    assert_eq!(v2["status"], "active", "re-pinned pin must be active");
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  POST with kind=="cluster" → 501 with slice-3 mention
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_cluster_pin_returns_501() {
    let server = build_server().await;
    let resp = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:c","kind":"cluster"}"#)
        .await;

    assert_eq!(
        resp.status,
        501,
        "cluster pin must return 501 Not Implemented; body: {:?}",
        std::str::from_utf8(&resp.body)
    );

    let body_str = std::str::from_utf8(&resp.body).expect("UTF-8 body");
    assert!(
        body_str.contains("slice-3"),
        "501 body must mention slice-3; got: {}",
        body_str
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  GET /api/v1/pins returns all pins; pull field present (may be null)
//     cluster pin was rejected → only epr:x in the list
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_pins_returns_active_pin_and_pull_field() {
    let server = build_server().await;

    // Seed one item pin.
    let create_resp = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:x"}"#)
        .await;
    assert_eq!(create_resp.status, 201, "seed pin must succeed");

    // Attempt a cluster pin — should be rejected (501), NOT stored.
    let cluster_resp = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:c","kind":"cluster"}"#)
        .await;
    assert_eq!(cluster_resp.status, 501, "cluster must be rejected");

    // List all pins.
    let list_resp = server.test_handle_list_pins().await;
    assert_eq!(
        list_resp.status,
        200,
        "GET /api/v1/pins must return 200; body: {:?}",
        std::str::from_utf8(&list_resp.body)
    );

    let body: serde_json::Value = serde_json::from_slice(&list_resp.body).expect("valid JSON");

    // Must have a "pins" array.
    let pins = body["pins"].as_array().expect("pins must be an array");

    // Only item pins were accepted — count the active ones.
    let active: Vec<_> = pins.iter().filter(|p| p["status"] == "active").collect();
    assert_eq!(
        active.len(),
        1,
        "exactly one active pin (epr:x); got: {:?}",
        pins
    );
    assert_eq!(active[0]["headRef"], "epr:x");
    assert_eq!(active[0]["kind"], "item");

    // The "pull" key must be present in the response (may be null without p2p).
    assert!(
        body.get("pull").is_some(),
        "response must have a 'pull' key (may be null); body: {}",
        body
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  DELETE /api/v1/pins/{id} soft-deletes; GET shows status=="removed"
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_pin_soft_deletes_and_remains_in_list() {
    let server = build_server().await;

    // Create a pin and capture its id.
    let create_resp = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:x"}"#)
        .await;
    assert_eq!(create_resp.status, 201);
    let pin: serde_json::Value = serde_json::from_slice(&create_resp.body).unwrap();
    let pin_id = pin["id"].as_i64().expect("id must be an integer");

    // Delete it.
    let del_resp = server.test_handle_remove_pin(&pin_id.to_string()).await;
    assert_eq!(
        del_resp.status,
        200,
        "DELETE must return 200; body: {:?}",
        std::str::from_utf8(&del_resp.body)
    );
    let del_body: serde_json::Value = serde_json::from_slice(&del_resp.body).unwrap();
    assert_eq!(del_body["removed"], pin_id, "removed id must match");

    // GET must still include the pin, but with status=="removed"
    // (list_all_pins returns every row regardless of status).
    let list_resp = server.test_handle_list_pins().await;
    assert_eq!(list_resp.status, 200);
    let list_body: serde_json::Value = serde_json::from_slice(&list_resp.body).unwrap();
    let pins = list_body["pins"].as_array().expect("pins array");

    let removed: Vec<_> = pins.iter().filter(|p| p["id"] == pin_id).collect();
    assert_eq!(
        removed.len(),
        1,
        "deleted pin must still appear in list_all"
    );
    assert_eq!(
        removed[0]["status"], "removed",
        "status must be 'removed' after DELETE"
    );

    // No active pins remain.
    let active: Vec<_> = pins.iter().filter(|p| p["status"] == "active").collect();
    assert!(
        active.is_empty(),
        "no active pins after delete; got: {:?}",
        active
    );
}
