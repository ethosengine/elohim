//! M1 tracer-bullet integration test.
//!
//! Builds the bridge schema with a real (in-memory) DbPool, exercises the
//! economicEvent query, and asserts both:
//!   (a) the response contains fixture data
//!   (b) a row landed in translation_observations
//!
//! This is the smallest end-to-end test that proves the M1 wire path.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use valueflows_bridge::{schema, BridgeContext, DbPool};
use valueflows_types::TranslationKind;

/// Re-embed the elohim-storage migrations directory at test build time so
/// the in-memory sqlite has the `translation_observations` table.
const MIGRATIONS: EmbeddedMigrations =
    embed_migrations!("../../../elohim/elohim-storage/migrations");

fn build_test_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder()
        .max_size(1) // single conn so migrations + queries share state
        .build(manager)
        .expect("build pool");
    let mut conn = pool.get().expect("get conn");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("run migrations");
    pool
}

#[tokio::test]
async fn economic_event_query_returns_fixture_and_logs_observation() {
    let pool = build_test_pool();
    let ctx = BridgeContext { pool: pool.clone() };
    let schema = schema::build_schema(ctx);

    let req = async_graphql::Request::new(
        r#"query { economicEvent(id: "tracer-bullet-id") {
              id action provider receiver note
          } }"#
            .to_string(),
    );
    let resp = schema.execute(req).await;
    assert!(resp.errors.is_empty(), "graphql errors: {:?}", resp.errors);

    let data = resp.data.into_json().expect("data is json");
    let ee = &data["economicEvent"];
    assert_eq!(ee["id"], "tracer-bullet-id", "fixture echoes id");
    assert_eq!(ee["action"], "transfer", "fixture action");
    assert_eq!(ee["provider"], "agent-fixture-provider");
    assert_eq!(ee["receiver"], "agent-fixture-receiver");
    assert_eq!(
        ee["note"],
        "M1 tracer-bullet fixture; M3 will return real hREA data",
        "note matches fixture",
    );

    // Verify a translation observation was written.
    let mut conn = pool.get().expect("get conn");
    let count: i64 = diesel::sql_query("SELECT COUNT(*) AS c FROM translation_observations")
        .get_result::<CountRow>(&mut conn)
        .expect("count query")
        .c;
    assert_eq!(count, 1, "exactly one observation written");

    let kind: String = diesel::sql_query(
        "SELECT translation_kind AS c FROM translation_observations LIMIT 1",
    )
    .get_result::<StringRow>(&mut conn)
    .expect("kind query")
    .c;
    assert_eq!(
        kind,
        TranslationKind::IdentityShape.as_ledger_str(),
        "M1 fixture is IdentityShape",
    );
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    c: i64,
}

#[derive(QueryableByName)]
struct StringRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    c: String,
}
