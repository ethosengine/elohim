//! Startup wiring smoke test — verifies seed_if_empty + sweep_expired
//! contract behaviours that main.rs depends on for restart-safety.
//!
//! See: genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md §Testing::Smoke

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;

fn fresh_pool() -> elohim_storage::db::DbPool {
    let url = format!(
        "file:startup_wiring_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(&url))
        .expect("pool");
    elohim_storage::db::run_migrations(&pool).expect("migrations");
    pool
}

#[test]
fn seed_if_empty_seeds_manifests_first_time() {
    let pool = fresh_pool();
    let mut conn = pool.get().unwrap();
    let report =
        elohim_storage::services::bootstrap_manifests::seed_if_empty(&mut conn).expect("seed");
    assert!(
        report.standing_policy_seeded || report.tending_policy_seeded,
        "at least one bootstrap manifest must be seeded on first run, got {:?}",
        report
    );
}

#[test]
fn seed_if_empty_is_idempotent() {
    let pool = fresh_pool();
    let mut conn = pool.get().unwrap();
    let _ = elohim_storage::services::bootstrap_manifests::seed_if_empty(&mut conn).expect("first");
    let report =
        elohim_storage::services::bootstrap_manifests::seed_if_empty(&mut conn).expect("second");
    assert!(
        !report.standing_policy_seeded,
        "standing-policy should not re-seed: {:?}",
        report
    );
    assert!(
        !report.tending_policy_seeded,
        "tending-policy should not re-seed: {:?}",
        report
    );
}

#[test]
fn sweep_expired_clean_db_returns_zero() {
    let pool = fresh_pool();
    let mut conn = pool.get().unwrap();
    let deleted = elohim_storage::services::tending::sweep_expired(&mut conn).expect("sweep");
    assert_eq!(deleted, 0);
}
