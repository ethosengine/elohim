//! Integration tests for the valueflows bridge.
//!
//! Shared test helpers (in this file). Real tests live in `tests/`.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use valueflows_bridge::DbPool;

/// Migrations from elohim-storage are embedded so the in-memory SQLite pool
/// has the `translation_observations` table available to the bridge's
/// ledger writes.
pub const MIGRATIONS: EmbeddedMigrations =
    embed_migrations!("../../../elohim/elohim-storage/migrations");

/// Build an in-memory SQLite pool with migrations applied. `max_size(1)` so
/// migrations + queries share the same `:memory:` instance.
pub fn build_test_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .expect("build pool");
    let mut conn = pool.get().expect("get conn");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("run migrations");
    pool
}
