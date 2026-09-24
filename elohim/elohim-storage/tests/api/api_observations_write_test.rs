//! `POST /api/v1/observations` — the self-row write path (plan task A3,
//! ruling R-A2 of the post-station-4 sprint).
//!
//! The handler reads the explicit `X-Agent-Cid` header and the body, then
//! delegates to `api::observations::accept_observation`, which these tests
//! drive directly (the codebase has no hyper test harness; see
//! `api_placement_gaps.rs`). Each error is mapped through
//! `services::response::error_response`, so the asserted status IS the status
//! the route answers with.
//!
//! What the route promises:
//! - the observer is the explicit header, verbatim: no header → 401, a body
//!   naming another observer → 403;
//! - the kind is manifest-declared (400 otherwise) and the payload matches the
//!   kind's declared field map (400 with the registry's reason);
//! - the row is unsigned (`signature_b64 == ""`) and the ack says so
//!   (`signed: "absent"`, `observerCidNamespace: "as-asserted"`).

use std::sync::Arc;

use diesel::prelude::*;
use elohim_storage::api::observations::accept_observation;
use elohim_storage::db::diesel_schema::observations;
use elohim_storage::db::models::ObservationRow;
use elohim_storage::error::StorageError;
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::services::observation_kinds::ObservationKindRegistry;
use elohim_storage::services::response::error_response;
use elohim_storage::test_util::test_pool;
use hyper::StatusCode;
use serde_json::json;

const NOW: i64 = 1_790_000_000;

fn manager() -> ObservationManagerBackend {
    ObservationManagerBackend::new()
        .with_kind_registry(Arc::new(ObservationKindRegistry::embedded()))
}

fn content_viewed(dwell_ms: u64, depth: u8) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "observationKind": "lamad:content-viewed",
        "subjectCid": "bafy-node-1",
        "subjectKind": "content",
        "payloadJson": json!({
            "ref_cid": "bafy-node-1",
            "dwell_ms": dwell_ms,
            "scroll_depth_pct": depth,
        }).to_string(),
    }))
    .unwrap()
}

fn status_of(err: StorageError) -> StatusCode {
    error_response(err).status()
}

#[tokio::test]
async fn post_without_agent_header_is_401() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let err = accept_observation(&manager(), &mut conn, None, &content_viewed(3000, 50), NOW)
        .await
        .unwrap_err();
    assert_eq!(status_of(err), StatusCode::UNAUTHORIZED);

    // An empty header asserts nothing either.
    let err = accept_observation(
        &manager(),
        &mut conn,
        Some(""),
        &content_viewed(3000, 50),
        NOW,
    )
    .await
    .unwrap_err();
    assert_eq!(status_of(err), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_with_body_observer_cid_other_than_header_is_403() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mut body: serde_json::Value = serde_json::from_slice(&content_viewed(3000, 50)).unwrap();
    body["observerCid"] = json!("human-james");
    let err = accept_observation(
        &manager(),
        &mut conn,
        Some("human-jessica"),
        &serde_json::to_vec(&body).unwrap(),
        NOW,
    )
    .await
    .unwrap_err();
    assert_eq!(status_of(err), StatusCode::FORBIDDEN);

    let count: i64 = observations::table.count().get_result(&mut conn).unwrap();
    assert_eq!(count, 0, "a refused write must leave no row");

    // Naming yourself is redundant, not refused.
    body["observerCid"] = json!("human-jessica");
    accept_observation(
        &manager(),
        &mut conn,
        Some("human-jessica"),
        &serde_json::to_vec(&body).unwrap(),
        NOW,
    )
    .await
    .expect("a body naming the header's own observer is accepted");
}

#[tokio::test]
async fn post_unknown_kind_is_400() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let body = serde_json::to_vec(&json!({
        "observationKind": "lamad:never-declared",
        "payloadJson": "{}",
    }))
    .unwrap();
    let err = accept_observation(&manager(), &mut conn, Some("human-jessica"), &body, NOW)
        .await
        .unwrap_err();
    let message = err.to_string();
    assert_eq!(status_of(err), StatusCode::BAD_REQUEST);
    assert!(
        message.contains("unknown observation kind lamad:never-declared"),
        "{message}"
    );
}

#[tokio::test]
async fn post_payload_violating_declared_schema_is_400() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let control = accept_observation(
        &manager(),
        &mut conn,
        Some("human-jessica"),
        &content_viewed(3000, 0),
        NOW,
    )
    .await;
    assert!(control.is_ok(), "control: a declared payload is accepted");

    // scroll_depth_pct is declared u8: 256 overflows.
    let body = serde_json::to_vec(&json!({
        "observationKind": "lamad:content-viewed",
        "payloadJson": json!({ "ref_cid": "bafy", "dwell_ms": 10, "scroll_depth_pct": 256 }).to_string(),
    }))
    .unwrap();
    let err = accept_observation(&manager(), &mut conn, Some("human-jessica"), &body, NOW)
        .await
        .unwrap_err();
    let message = err.to_string();
    assert_eq!(status_of(err), StatusCode::BAD_REQUEST);
    assert!(message.contains("scroll_depth_pct"), "{message}");

    // A payload that is not JSON at all is a 400, never a 500.
    let body = serde_json::to_vec(&json!({
        "observationKind": "lamad:content-viewed",
        "payloadJson": "not json",
    }))
    .unwrap();
    let err = accept_observation(&manager(), &mut conn, Some("human-jessica"), &body, NOW)
        .await
        .unwrap_err();
    assert_eq!(status_of(err), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_valid_appends_row_with_header_as_observer_and_empty_signature() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = manager();

    let first = accept_observation(
        &mgr,
        &mut conn,
        Some("human-jessica"),
        &content_viewed(3200, 64),
        NOW,
    )
    .await
    .expect("valid observation accepted");
    assert_eq!(first.observer_cid, "human-jessica");
    assert_eq!(first.observer_cid_namespace, "as-asserted");
    assert_eq!(first.signed, "absent");
    assert_eq!(first.log_offset, 0);
    assert!(first.log_cid.starts_with("blake3:"), "{}", first.log_cid);

    let second = accept_observation(
        &mgr,
        &mut conn,
        Some("human-jessica"),
        &content_viewed(900, 10),
        NOW + 5,
    )
    .await
    .unwrap();
    assert_eq!(second.log_offset, 1);
    assert_ne!(second.log_cid, first.log_cid, "the log root advances");

    // The wire ack is camelCase.
    let wire = serde_json::to_value(&second).unwrap();
    for key in [
        "observerCid",
        "observerCidNamespace",
        "logCid",
        "logOffset",
        "seq",
        "signed",
    ] {
        assert!(wire.get(key).is_some(), "ack missing {key}: {wire}");
    }

    let rows: Vec<ObservationRow> = observations::table
        .order(observations::log_offset.asc())
        .load(&mut conn)
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].observer_cid, "human-jessica");
    assert_eq!(
        rows[0].signature_b64, "",
        "the row is unsigned until the signing graduation"
    );
    assert_eq!(rows[0].observation_kind, "lamad:content-viewed");
    assert_eq!(rows[0].subject_cid.as_deref(), Some("bafy-node-1"));
    // observed_at absent from the intent → server-stamped.
    assert_eq!(rows[0].observed_at, NOW);
    assert_eq!(rows[1].observed_at, NOW + 5);
}

#[tokio::test]
async fn observed_at_from_the_intent_is_kept() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mut body: serde_json::Value = serde_json::from_slice(&content_viewed(3000, 50)).unwrap();
    body["observedAt"] = json!(NOW - 60);
    accept_observation(
        &manager(),
        &mut conn,
        Some("human-jessica"),
        &serde_json::to_vec(&body).unwrap(),
        NOW,
    )
    .await
    .unwrap();
    let row: ObservationRow = observations::table.first(&mut conn).unwrap();
    assert_eq!(row.observed_at, NOW - 60);
}

#[tokio::test]
async fn seq_increments_per_observer() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = manager();

    let j1 = accept_observation(
        &mgr,
        &mut conn,
        Some("human-jessica"),
        &content_viewed(1, 1),
        NOW,
    )
    .await
    .unwrap();
    let j2 = accept_observation(
        &mgr,
        &mut conn,
        Some("human-jessica"),
        &content_viewed(2, 2),
        NOW,
    )
    .await
    .unwrap();
    let m1 = accept_observation(
        &mgr,
        &mut conn,
        Some("human-matthew"),
        &content_viewed(3, 3),
        NOW,
    )
    .await
    .unwrap();
    let j3 = accept_observation(
        &mgr,
        &mut conn,
        Some("human-jessica"),
        &content_viewed(4, 4),
        NOW,
    )
    .await
    .unwrap();

    assert_eq!(
        (j1.seq, j2.seq, j3.seq),
        (1, 2, 3),
        "jessica's own sequence"
    );
    assert_eq!(m1.seq, 1, "matthew's sequence is independent of jessica's");
    assert_eq!(m1.log_offset, 0, "and so is his log");
    assert_eq!(j3.log_offset, 2);
}

// ---------------------------------------------------------------------------
// Bounds (ruling R-A10): observedAt, body size, subjectCid == payload.ref_cid.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observed_at_out_of_bounds_is_400() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    for bad in [json!(-1), json!(NOW + 301), json!(i64::MAX)] {
        let mut body: serde_json::Value =
            serde_json::from_slice(&content_viewed(3000, 50)).unwrap();
        body["observedAt"] = bad.clone();
        let err = accept_observation(
            &manager(),
            &mut conn,
            Some("human-jessica"),
            &serde_json::to_vec(&body).unwrap(),
            NOW,
        )
        .await
        .unwrap_err();
        let message = err.to_string();
        assert_eq!(status_of(err), StatusCode::BAD_REQUEST, "{bad}");
        assert!(message.contains("observedAt"), "{message}");
    }
    let count: i64 = observations::table.count().get_result(&mut conn).unwrap();
    assert_eq!(count, 0, "a refused write leaves no row");

    // The edges are inside the bound: the epoch and five minutes of clock skew.
    for good in [0, NOW + 300] {
        let mut body: serde_json::Value =
            serde_json::from_slice(&content_viewed(3000, 50)).unwrap();
        body["observedAt"] = json!(good);
        accept_observation(
            &manager(),
            &mut conn,
            Some("human-jessica"),
            &serde_json::to_vec(&body).unwrap(),
            NOW,
        )
        .await
        .unwrap_or_else(|e| panic!("observedAt {good} is in bounds: {e}"));
    }
}

#[tokio::test]
async fn post_body_over_16_kib_is_400() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mut body: serde_json::Value = serde_json::from_slice(&content_viewed(3000, 50)).unwrap();
    // Unknown fields would be refused anyway; pad a known one so only the size speaks.
    body["subjectKind"] =
        json!("x".repeat(elohim_storage::api::observations::MAX_OBSERVATION_BODY_BYTES));
    let bytes = serde_json::to_vec(&body).unwrap();
    assert!(bytes.len() > 16 * 1024);
    let err = accept_observation(&manager(), &mut conn, Some("human-jessica"), &bytes, NOW)
        .await
        .unwrap_err();
    let message = err.to_string();
    assert_eq!(status_of(err), StatusCode::BAD_REQUEST);
    assert!(message.contains("16384"), "{message}");
    assert_eq!(
        elohim_storage::api::observations::MAX_OBSERVATION_BODY_BYTES,
        16 * 1024
    );
}

#[tokio::test]
async fn subject_cid_must_equal_payload_ref_cid() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let payload =
        json!({ "ref_cid": "bafy-node-1", "dwell_ms": 10, "scroll_depth_pct": 5 }).to_string();

    for subject in [Some("bafy-node-2"), None] {
        let mut body = json!({
            "observationKind": "lamad:content-viewed",
            "subjectKind": "content",
            "payloadJson": payload,
        });
        if let Some(s) = subject {
            body["subjectCid"] = json!(s);
        }
        let err = accept_observation(
            &manager(),
            &mut conn,
            Some("human-jessica"),
            &serde_json::to_vec(&body).unwrap(),
            NOW,
        )
        .await
        .unwrap_err();
        let message = err.to_string();
        assert_eq!(status_of(err), StatusCode::BAD_REQUEST, "{subject:?}");
        assert!(message.contains("ref_cid"), "{message}");
    }
    let count: i64 = observations::table.count().get_result(&mut conn).unwrap();
    assert_eq!(count, 0);
}
