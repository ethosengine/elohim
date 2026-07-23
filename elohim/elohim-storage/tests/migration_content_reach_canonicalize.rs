//! Migration test for `2026-07-23-140000_content_reach_canonicalize` (reach-vocab
//! slice 3, task 5): a scratch in-memory SQLite `content` table is seeded with
//! one row per pre-canonical legacy value plus one already-canonical row, the
//! migration's `up.sql` is executed verbatim via `include_str!`, and the
//! resulting `reach` value multiset is asserted exactly.
//!
//! Order is load-bearing: the old top-rung `'public'` must land on `'commons'`
//! BEFORE `'district'`/`'federated'` remap INTO `'public'`, or those rows would
//! be double-migrated to `'commons'` too. This test seeds all values in one
//! batch and lets `up.sql`'s own statement order settle the ambiguity, so it
//! would catch a regression that reordered the UPDATE statements.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::collections::HashMap;

/// Statements from the actual migration file — read at test-compile time so
/// drift between this test and the shipped migration is impossible.
const UP_SQL: &str =
    include_str!("../migrations/2026-07-23-140000_content_reach_canonicalize/up.sql");

fn setup_db() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

    conn.batch_execute(
        r#"
        CREATE TABLE content (
            id    TEXT PRIMARY KEY NOT NULL,
            reach TEXT NOT NULL
        );
        "#,
    )
    .expect("create scratch content table");

    conn
}

#[test]
fn up_sql_canonicalizes_every_legacy_value() {
    let mut conn = setup_db();

    // One row per pre-canonical / retired value, plus one row already on the
    // canonical schema-8 vocabulary (`commons`) to prove idempotence for rows
    // that need no change.
    let seed_rows: &[(&str, &str)] = &[
        ("row-public", "public"),
        ("row-district", "district"),
        ("row-federated", "federated"),
        ("row-personal", "personal"),
        ("row-household", "household"),
        ("row-local", "local"),
        ("row-neighborhood", "neighborhood"),
        ("row-collective", "collective"),
        ("row-invited", "invited"),
        ("row-commons", "commons"),
    ];

    for (id, reach) in seed_rows {
        diesel::sql_query("INSERT INTO content (id, reach) VALUES (?, ?)")
            .bind::<diesel::sql_types::Text, _>(*id)
            .bind::<diesel::sql_types::Text, _>(*reach)
            .execute(&mut conn)
            .unwrap_or_else(|e| panic!("seed row {id} failed: {e}"));
    }

    conn.batch_execute(UP_SQL)
        .expect("up.sql migration should apply cleanly");

    #[derive(QueryableByName, Debug)]
    struct ReachRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        id: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        reach: String,
    }

    let rows: Vec<ReachRow> = diesel::sql_query("SELECT id, reach FROM content ORDER BY id")
        .load(&mut conn)
        .expect("select migrated rows");

    let got: HashMap<String, String> = rows.into_iter().map(|r| (r.id, r.reach)).collect();

    let expected: &[(&str, &str)] = &[
        ("row-public", "commons"),
        ("row-district", "public"),
        ("row-federated", "public"),
        ("row-personal", "self"),
        ("row-household", "trusted"),
        ("row-local", "trusted"),
        ("row-neighborhood", "familiar"),
        ("row-collective", "community"),
        ("row-invited", "intimate"),
        ("row-commons", "commons"),
    ];

    for (id, expected_reach) in expected {
        assert_eq!(
            got.get(*id).map(String::as_str),
            Some(*expected_reach),
            "row {id} expected reach {expected_reach:?}, got {:?}",
            got.get(*id)
        );
    }

    // No stray/legacy values should survive the migration.
    let canonical: std::collections::HashSet<&str> = [
        "private",
        "self",
        "intimate",
        "trusted",
        "familiar",
        "community",
        "public",
        "commons",
    ]
    .into_iter()
    .collect();
    for (id, reach) in &got {
        assert!(
            canonical.contains(reach.as_str()),
            "row {id} left with non-canonical reach {reach:?}"
        );
    }
}
