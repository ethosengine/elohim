//! M1 HTTP-level smoke test.
//!
//! Exercises the body-bytes → schema → response wire via the
//! `handle_request_for_test` helper. Complements `m1_tracer_bullet.rs`
//! (schema-level test) by going one layer deeper.

use bytes::Bytes;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use http_body_util::BodyExt;
use valueflows_bridge::{handle_request_for_test, BridgeContext, DbPool};

const MIGRATIONS: EmbeddedMigrations =
    embed_migrations!("../../../elohim/elohim-storage/migrations");

fn build_test_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder().max_size(1).build(manager).expect("build pool");
    let mut conn = pool.get().expect("get conn");
    conn.run_pending_migrations(MIGRATIONS).expect("migrations");
    pool
}

#[tokio::test]
async fn vf_graphql_returns_fixture_economic_event_via_handler_for_test() {
    let pool = build_test_pool();
    let ctx = BridgeContext { pool };

    let body = serde_json::json!({
        "query": "query { economicEvent(id: \"smoke\") { id action provider } }",
    });
    let body_bytes = Bytes::from(serde_json::to_vec(&body).unwrap());

    let resp = handle_request_for_test(body_bytes, ctx)
        .await
        .expect("handler returns OK response");

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("Content-Type").unwrap().to_str().unwrap(),
        "application/json"
    );

    let resp_body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let resp_json: serde_json::Value =
        serde_json::from_slice(&resp_body).expect("parse response json");

    assert!(
        resp_json["errors"].is_null() || resp_json["errors"].as_array().unwrap().is_empty(),
        "no graphql errors: {:?}",
        resp_json["errors"]
    );
    let ee = &resp_json["data"]["economicEvent"];
    assert_eq!(ee["id"], "smoke");
    assert_eq!(ee["action"], "transfer");
    assert_eq!(ee["provider"], "agent-fixture-provider");
}

#[tokio::test]
async fn invalid_graphql_body_returns_400() {
    let pool = build_test_pool();
    let ctx = BridgeContext { pool };

    // Not valid GraphQL request shape (no "query" field).
    let body_bytes = Bytes::from_static(b"this is not a graphql request");

    let resp = handle_request_for_test(body_bytes, ctx)
        .await
        .expect("handler returns response");

    assert_eq!(resp.status(), 400, "malformed body → 400");
    let resp_body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let resp_json: serde_json::Value =
        serde_json::from_slice(&resp_body).expect("parse response json");
    let code = &resp_json["errors"][0]["extensions"]["code"];
    assert_eq!(code, "invalid_graphql_request");
}
