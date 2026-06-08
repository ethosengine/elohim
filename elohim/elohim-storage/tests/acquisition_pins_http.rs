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
//! 1. POST /api/v1/pins {"headRef":"epr:x"} → 201, kind=="item", status=="active",
//!    headRef returned as bare "x" (epr: prefix stripped at the boundary).
//! 2. POST /api/v1/pins {"headRef":"epr:x"} again → 201 idempotent, SAME id.
//! 3. POST /api/v1/pins {"headRef":"epr:c","kind":"cluster"} → 501, mentions slice-3.
//! 4. GET /api/v1/pins → 200, exactly one active pin for bare "x", pull field present.
//! 5. DELETE /api/v1/pins/{id} → 200; GET shows that pin status=="removed".
//! 6. POST /api/v1/pins with malformed body → 400 JSON error (not 500).
//! 7. HTTP boundary normalization: POST {"headRef":"epr:foo"} → stored/returned
//!    headRef is bare "foo". (Full reconcile→completion proof is in
//!    acquisition_pull_e2e.rs, which uses bare ids end-to-end.)

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
    // The epr: prefix must be stripped at the POST boundary so the stored/returned
    // headRef is the bare content id that the reconcile/completion loop keys on.
    assert_eq!(
        val["headRef"], "x",
        "headRef must be the bare id (epr: stripped)"
    );
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
        "exactly one active pin (bare \"x\"); got: {:?}",
        pins
    );
    // epr: prefix stripped at POST boundary — stored/returned headRef is bare.
    assert_eq!(active[0]["headRef"], "x");
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

// ─────────────────────────────────────────────────────────────────────────────
// §6  POST with malformed body → 400 JSON error (not 500)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_malformed_json_returns_400() {
    let server = build_server().await;

    // Send a body that is not valid JSON at all.
    let resp = server.test_handle_create_pin_raw("{not json").await;

    assert_eq!(
        resp.status,
        400,
        "malformed POST body must return 400, not 500; body: {:?}",
        std::str::from_utf8(&resp.body)
    );

    // The response body must be valid JSON with an "error" field.
    let body: serde_json::Value =
        serde_json::from_slice(&resp.body).expect("400 response must itself be valid JSON");
    assert!(
        body.get("error").is_some(),
        "400 body must contain an 'error' field; got: {}",
        body
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §7  HTTP boundary normalization: epr: prefix stripped from stored/returned headRef
//
// Proves that the POST boundary canonicalizes `epr:`-prefixed refs to the bare
// content id — the form that content_ids_present / reconcile / mark_completed
// all key on. The full reconcile→completion proof (using bare ids end-to-end)
// lives in acquisition_pull_e2e.rs; together these two tests cover the full seam.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn post_pin_strips_epr_prefix_from_head_ref() {
    let server = build_server().await;

    let resp = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:foo"}"#)
        .await;

    assert_eq!(
        resp.status,
        201,
        "POST /api/v1/pins must return 201; body: {:?}",
        std::str::from_utf8(&resp.body)
    );

    let val: serde_json::Value = serde_json::from_slice(&resp.body).expect("valid JSON body");
    assert_eq!(
        val["headRef"], "foo",
        "headRef must be the bare id 'foo', not 'epr:foo'; got: {}",
        val["headRef"]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §8  GET /api/v1/pins/{eprId}/pull — own-node-only per-EPR progress (T13)
//
// Without a wired p2p handle there is no live AcquisitionState to read, so the
// handler returns 404 for any EPR. This proves the airplane-mode + own-node-only
// shape: the route exists, touches only local DB for the pin→head map, and
// yields a sane "nothing pulling" 404 (not a 500) when there is no pull state.
// The grouping/dedupe math itself is unit-tested in p2p::acquisition::tests
// (per_epr_groups_by_head_ref_counting_shared_content_once et al.).
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn epr_pull_unknown_epr_is_404() {
    // Built without a wired p2p handle → no AcquisitionState → 404 for any EPR.
    let server = build_server().await;
    let resp = server.test_handle_epr_pull("epr:does-not-exist").await;
    assert_eq!(
        resp.status,
        404,
        "unknown/un-tracked EPR must be 404, body: {:?}",
        String::from_utf8_lossy(&resp.body)
    );
}

#[tokio::test]
async fn epr_pull_pinned_but_no_p2p_is_404() {
    // Even with a pin present in the local table, the absence of a live p2p
    // AcquisitionState means there is no pull progress to report → 404 (the
    // own-node-only airplane-mode contract; never a 500).
    let server = build_server().await;
    let create = server
        .test_handle_create_pin_raw(r#"{"headRef":"epr:x"}"#)
        .await;
    assert_eq!(create.status, 201, "seed pin must succeed");

    // Query by the bare id and by the epr:-prefixed id — both normalize to "x"
    // at the boundary and both 404 without a wired AcquisitionState.
    let bare = server.test_handle_epr_pull("x").await;
    assert_eq!(
        bare.status,
        404,
        "no p2p → no pull state → 404; body: {:?}",
        String::from_utf8_lossy(&bare.body)
    );
    let prefixed = server.test_handle_epr_pull("epr:x").await;
    assert_eq!(
        prefixed.status,
        404,
        "epr: prefix must normalize to the bare id; body: {:?}",
        String::from_utf8_lossy(&prefixed.body)
    );
}
