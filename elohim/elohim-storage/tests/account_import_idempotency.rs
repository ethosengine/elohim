//! Integration tests: account-import relationship idempotency
//!
//! Covers the 2026-04-17 seed-pipeline regression where importing Adam's
//! package (declaring a bidirectional spouse with Eve) followed by Eve's
//! package (declaring the same symmetric edge) produced a UNIQUE constraint
//! error instead of converging to exactly one `human_relationships` row.
//!
//! ## Approach
//!
//! `do_account_import` is a private method on `StorageServer` and there is no
//! axum Router test fixture in this crate.  All existing integration tests use
//! direct Rust calls against in-memory SQLite (see `provenance_gate_integration`,
//! `peer_status_e2e`, `sync_integration`).  We follow the same pattern here:
//!
//! - Spin up an in-memory SQLite with the exact `human_relationships` DDL used
//!   by production (same schema the unit tests in `human_relationships.rs` use).
//! - Run the Phase-2 relationship loop that `do_account_import` contains —
//!   calling `find_existing_relationship` then `create_human_relationship` —
//!   directly and track `relationships_created` / `relationships_skipped` with
//!   the same logic the handler uses.
//!
//! This approach exercises the truth layer (the DB functions added in Task 1
//! and the loop rewritten in Tasks 2+3) without needing a private entry-point
//! or a running HTTP server.
//!
//! ## Tests
//!
//! 1. `adam_eve_bidirectional_spouse_converges` — two parties independently
//!    author the same symmetric spouse edge; exactly one row is written.
//! 2. `rerunning_import_is_idempotent` — importing the same package twice
//!    produces one created row on the first run, zero on the second.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use elohim_storage::db::context::AppContext;
use elohim_storage::db::human_relationships::{
    create_human_relationship, find_existing_relationship, CreateHumanRelationshipInput,
};
use elohim_storage::error::StorageError;

// ============================================================================
// Test harness helpers
// ============================================================================

/// Spin up an in-memory SQLite and create the `human_relationships` table with
/// the exact DDL that production uses (mirrors the unit-test helper in
/// `src/db/human_relationships.rs` and the production migration).
fn setup_db() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:")
        .expect("failed to open in-memory SQLite");

    conn.batch_execute(
        r#"
        CREATE TABLE human_relationships (
            id                       TEXT    PRIMARY KEY,
            h_app_id                 TEXT    NOT NULL,
            party_a_id               TEXT    NOT NULL,
            party_b_id               TEXT    NOT NULL,
            relationship_type        TEXT    NOT NULL,
            intimacy_level           TEXT    NOT NULL,
            is_bidirectional         INTEGER NOT NULL DEFAULT 0,
            consent_given_by_a       INTEGER NOT NULL DEFAULT 0,
            consent_given_by_b       INTEGER NOT NULL DEFAULT 0,
            custody_enabled_by_a     INTEGER NOT NULL DEFAULT 0,
            custody_enabled_by_b     INTEGER NOT NULL DEFAULT 0,
            auto_custody_enabled     INTEGER NOT NULL DEFAULT 0,
            emergency_access_enabled INTEGER NOT NULL DEFAULT 0,
            initiated_by             TEXT    NOT NULL,
            verified_at              TEXT,
            governance_layer         TEXT,
            reach                    TEXT    NOT NULL DEFAULT 'private',
            context_json             TEXT,
            created_at               TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at               TEXT    NOT NULL DEFAULT (datetime('now')),
            expires_at               TEXT,
            dht_anchor_hash          TEXT,
            UNIQUE (h_app_id, party_a_id, party_b_id, relationship_type)
        );
        "#,
    )
    .expect("failed to create test schema");

    conn
}

/// A minimal description of one relationship seed as it would appear in an
/// `AccountPackageInputView.relationships` entry.
struct RelSeed {
    target_id: String,
    relationship_type: String,
    intimacy_level: String,
    is_bidirectional: bool,
    reach: String,
}

/// Mirrors the Phase-2 loop inside `do_account_import` (http.rs).  Returns
/// `(relationships_created, relationships_skipped, errors)`.
///
/// Keeping this in the test file (not in lib.rs) avoids polluting the public
/// API surface.  If the loop is ever extracted into a service function, replace
/// this helper with a direct call to that function.
fn run_relationship_phase(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    seeds: &[RelSeed],
) -> (usize, usize, Vec<String>) {
    let mut created = 0usize;
    let mut skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for seed in seeds {
        let existing = find_existing_relationship(
            conn,
            ctx,
            human_id,
            &seed.target_id,
            &seed.relationship_type,
            seed.is_bidirectional,
        )
        .expect("find_existing_relationship must not error");

        if existing.is_some() {
            skipped += 1;
            continue;
        }

        let input = CreateHumanRelationshipInput {
            id: None,
            party_a_id: human_id.to_string(),
            party_b_id: seed.target_id.clone(),
            relationship_type: seed.relationship_type.clone(),
            intimacy_level: seed.intimacy_level.clone(),
            is_bidirectional: seed.is_bidirectional,
            consent_given_by_a: true,
            consent_given_by_b: false,
            initiated_by: human_id.to_string(),
            governance_layer: None,
            reach: seed.reach.clone(),
            context_json: None,
            expires_at: None,
        };

        match create_human_relationship(conn, ctx, input) {
            Ok(_) => created += 1,
            Err(StorageError::Internal(msg)) if msg.contains("UNIQUE constraint") => {
                // Race: a concurrent import beat us between pre-check and insert.
                // Same shape row — treat as skipped, not an error.
                skipped += 1;
            }
            Err(e) => {
                errors.push(format!(
                    "failed to create relationship {} -> {}: {}",
                    human_id, seed.target_id, e
                ));
            }
        }
    }

    (created, skipped, errors)
}

/// Count rows in `human_relationships` for a given app context.
fn count_rel_rows(conn: &mut SqliteConnection, ctx: &AppContext) -> i64 {
    use diesel::dsl::count_star;
    use elohim_storage::db::diesel_schema::human_relationships;

    human_relationships::table
        .filter(human_relationships::h_app_id.eq(&ctx.h_app_id))
        .select(count_star())
        .first(conn)
        .expect("count query failed")
}

// ============================================================================
// Test 1: bidirectional convergence
// ============================================================================

/// Adam imports first, declaring a bidirectional spouse with Eve.
/// Eve imports second, declaring the mirror edge.
/// Expected result: exactly one `human_relationships` row; second import
/// reports `relationships_skipped == 1, relationships_created == 0`, no errors.
#[test]
fn adam_eve_bidirectional_spouse_converges() {
    let mut conn = setup_db();
    let ctx = AppContext::default_lamad();

    // Adam's account package declares spouse → Eve (bidirectional).
    let adam_seeds = vec![RelSeed {
        target_id: "human-eve-firstwoman".into(),
        relationship_type: "spouse".into(),
        intimacy_level: "intimate".into(),
        is_bidirectional: true,
        reach: "private".into(),
    }];

    let (adam_created, adam_skipped, adam_errors) =
        run_relationship_phase(&mut conn, &ctx, "human-adam-firstman", &adam_seeds);

    assert_eq!(adam_errors, Vec::<String>::new(), "Adam import must have no errors");
    assert_eq!(adam_created, 1, "Adam import must create 1 relationship");
    assert_eq!(adam_skipped, 0, "Adam import must skip nothing");

    // Exactly one row after Adam's import.
    assert_eq!(
        count_rel_rows(&mut conn, &ctx),
        1,
        "exactly one human_relationships row after Adam's import"
    );

    // Eve's account package declares the symmetric spouse → Adam (also bidirectional).
    // The pre-check must detect the existing (adam, eve, spouse) row via the
    // bidirectional OR clause and skip rather than attempting an insert.
    let eve_seeds = vec![RelSeed {
        target_id: "human-adam-firstman".into(),
        relationship_type: "spouse".into(),
        intimacy_level: "intimate".into(),
        is_bidirectional: true,
        reach: "private".into(),
    }];

    let (eve_created, eve_skipped, eve_errors) =
        run_relationship_phase(&mut conn, &ctx, "human-eve-firstwoman", &eve_seeds);

    assert_eq!(eve_errors, Vec::<String>::new(), "Eve import must have no errors");
    assert_eq!(
        eve_created, 0,
        "Eve import must create 0 relationships (already exists)"
    );
    assert_eq!(
        eve_skipped, 1,
        "Eve import must report relationships_skipped == 1"
    );

    // Still exactly one row — bidirectional convergence achieved.
    assert_eq!(
        count_rel_rows(&mut conn, &ctx),
        1,
        "human_relationships must still have exactly one row after Eve's import"
    );
}

// ============================================================================
// Test 2: rerun idempotency
// ============================================================================

/// Importing the same package twice is safe: first run creates, second skips.
/// Covers the case where the seeder reruns a package without clearing state.
#[test]
fn rerunning_import_is_idempotent() {
    let mut conn = setup_db();
    let ctx = AppContext::default_lamad();

    // Package with two relationships (mentor + friend).
    let make_seeds = || {
        vec![
            RelSeed {
                target_id: "human-ruth-moabite".into(),
                relationship_type: "mentor".into(),
                intimacy_level: "trusted".into(),
                is_bidirectional: false,
                reach: "private".into(),
            },
            RelSeed {
                target_id: "human-naomi-elder".into(),
                relationship_type: "friend".into(),
                intimacy_level: "recognition".into(),
                is_bidirectional: true,
                reach: "private".into(),
            },
        ]
    };

    // First run — both relationships created.
    let (created1, skipped1, errors1) =
        run_relationship_phase(&mut conn, &ctx, "human-boaz-kinsman", &make_seeds());

    assert_eq!(errors1, Vec::<String>::new(), "first run must have no errors");
    assert_eq!(created1, 2, "first run must create 2 relationships");
    assert_eq!(skipped1, 0, "first run must skip nothing");
    assert_eq!(count_rel_rows(&mut conn, &ctx), 2, "2 rows after first run");

    // Second run — both already exist, both skipped.
    let (created2, skipped2, errors2) =
        run_relationship_phase(&mut conn, &ctx, "human-boaz-kinsman", &make_seeds());

    assert_eq!(errors2, Vec::<String>::new(), "second run must have no errors");
    assert_eq!(
        created2, 0,
        "second run must create 0 relationships (already exist)"
    );
    assert_eq!(
        skipped2, 2,
        "second run must report relationships_skipped == 2"
    );
    assert_eq!(
        count_rel_rows(&mut conn, &ctx),
        2,
        "row count must remain 2 after idempotent re-run"
    );
}
