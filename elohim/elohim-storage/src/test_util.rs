//! Test utilities shared across integration tests.
//!
//! Exposed as a public module so `tests/*.rs` integration test binaries can
//! reference `elohim_storage::test_util::test_pool`.  Not intended for
//! production use.

use crate::db::{init_pool, run_migrations, DbPool};

/// Create an in-memory SQLite pool with all migrations applied.
///
/// Each call returns a fresh, isolated database — safe to use concurrently
/// across test threads because SQLite in-memory databases are not shared.
pub fn test_pool() -> DbPool {
    // Use a unique URI so concurrent tests don't share the same in-memory DB.
    // The `?mode=memory&cache=private` prevents connection sharing across pool
    // connections (each checkout is a fresh empty DB), so we use a named DB
    // instead and allow the pool to keep one connection alive.
    let url = format!(
        "file:testdb_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = init_pool(&url).expect("test pool init failed");
    run_migrations(&pool).expect("test migrations failed");
    pool
}
