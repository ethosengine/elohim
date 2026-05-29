//! SQLite database module for structured content storage
//!
//! This module provides fast local storage for content,
//! replacing DHT-based content storage for better performance.
//!
//! ## Architecture
//!
//! - Content bodies stored in blob_store (content-addressed)
//! - Content metadata and tags stored in SQLite
//! - DHT used only for attestations and agent-centric data
//! - All operations are app-scoped for multi-tenant isolation
//!
//! ## Tables
//!
//! - `apps` - Registered apps for multi-tenancy
//! - `content` - Content metadata (id, h_app_id, title, type, blob_hash)
//! - `content_tags` - Tag index for fast lookup

// Diesel modules with app scoping
pub mod cache_queries;
pub mod content_diesel;
pub mod context;
pub mod diesel_schema;
pub mod models;

// Diesel modules for graph relationships and domain models
pub mod accumulation_status;
pub mod content_engagement_stats;
pub mod mechanism_selection;
// M-AGGR-1: session + upgrade-prompt projections (Category C, EconomicEvent × HumanProgress × onboarding Manifest)
pub mod agreements;
pub mod collectives;
pub mod comments;
pub mod content_mastery;
pub mod contributor_presences;
pub mod device_policies;
pub mod economic_events;
pub mod enum_registry;
pub mod hazards;
pub mod human_relationships;
pub mod humans;
pub mod imagodei_observations;
pub mod knowledge_maps_diesel;
pub mod local_sessions;
pub mod manifests;
pub mod places;
pub mod rea_commitments;
pub mod relationships_diesel;
pub mod risk_alerts;
pub mod schedules;
pub mod session_human_view;
pub mod spatial_contexts;
pub mod steward_affinity;
pub mod stewardship_allocations;
pub mod upgrade_prompt_view;

// Governance tables (v7)
pub mod governance;

// Attestation, steward, and contributor tables (v7)
pub mod content_attestations;
pub mod contributors;
pub mod steward_operations;

// Custodian node metrics
pub mod custodian_metrics;

// Token economy (shefa — elohim-token sprint 1)
pub mod token_balances;
pub mod token_mint_events;
pub mod token_transfers;

// Responsibility demand curve config (shefa — elohim-token sprint 2)
pub mod responsibility_demand_configs;

// Token decay events (shefa — elohim-token sprint 3)
pub mod token_decay_events;

// Shard protocol tables (P2P Resilience — Sprint B)
pub mod shard_locations;
pub mod shard_manifests;

// Stewarded node topology (node registry + node-human stewardship)
pub mod stewarded_nodes;

// Policy cache for stewardship enforcement
pub mod policy_cache;

// Observation session diagnostic system (Category C — operational)
pub mod observation_sessions;

// Peer status projection (Peer-Stewarded Availability — Phase 1)
pub mod peer_statuses;

// Gate decision attestation projection (mishpat DNA — Phase 4)
pub mod gate_decision_attestations;

// Gate decision challenge + outcome projections (mishpat DNA — Phase 11 Task 11.2)
pub mod challenge_outcomes;
pub mod gate_decision_challenges;

// Elohim reputation aggregation query (mishpat outcome graph — Phase 11 Task 11.3)
pub mod elohim_reputation;

// Placement gap CRUD — shefa signal surface (self-healing Plan 1 Task 4)
pub mod placement_gaps;

// EPR storage layer — Phase 2a (notarized atoms, coupling, claims, supersedence)
pub mod epr_atoms;

// Recovery Protocol Phase 2 — DHT projection CRUD (imagodei RecoveryRequest + KeyRotation)
pub mod recovery_requests;

// Recovery Protocol Phase 2 — M3 witness projection (IntimateWitnessSubmitted signal)
pub mod recovery_witnesses;

// EPR Phase 2B — peer identity bindings projection (AgentPeerBinding DHT signal)
pub mod peer_identity_bindings;

// T12 — peer_blob_inventory projection (libp2p gossipsub inventory, Category C operational)
pub mod peer_blob_inventory;

// Recovery Protocol Phase 2 — M4 revocation projection (key_revocations + revocation_votes)
// + recovery_flows state-machine projection (T6 schema, T7 CRUD)
pub mod key_revocations;
pub mod recovery_flows;
pub mod revocation_votes;

// Recovery Protocol Phase 2 — M5 portal host projection (imagodei PortalHost entry, Category A)
pub mod portal_hosts;

// EPR Phase 3.5 — predecessor records (trust-compute gradient back-prop, Category C operational)
pub mod predecessor_records;

// EPR Phase 3.5 — attention_tending (tending lifecycle cache, Category C operational)
pub mod tending;

// EPR Phase 3.5 — standing_view (per-evaluator StandingScore projection, Category C operational)
pub mod standing_view;

// Phase 4 — projection_events append-only log (Category C, rebuildable from rea_projection stream)
pub mod projection_events;

// Attestation Consolidation Sprint — unified attestation + governance projection tables
// Category A (DHT projection): attestations, governance_actions
// Category C (derived operational): governance_action_tally
pub mod attestations;
pub mod governance_action_tally;
pub mod governance_actions;

// Recovery M4 T20 — authorization gate for Shamir share responders
// Category C operational (queries the attestations projection; no new table)
pub mod recovery_approval_gate;

// Recovery M4 T21 — per-custodian Shamir share store
// Category C operational — write target of T22 extern; read by the responder
pub mod custodian_shares;

// Wave 3 M1 — valueflows bridge learning ledger (Category C operational)
// Each row is one TranslationPoint observation; aggregated at M5.
pub mod translation_observations;

// Sprint 3 — Mutual Storage Replication §6.2: sweep telemetry (Category C operational)
pub mod mutuality_audit_log;

use std::path::Path;
use std::time::Duration;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::info;

use crate::error::StorageError;
pub use context::AppContext;
// Re-export so api/* modules can use `crate::db::SqliteConnection` directly.
pub use diesel::sqlite::SqliteConnection;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Database statistics
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStats {
    pub content_count: u64,
    pub unique_tags: u64,
}

// ============================================================================
// Diesel Connection Pool
// ============================================================================

/// Type alias for Diesel connection pool
pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Type alias for pooled connection
pub type PooledConn = r2d2::PooledConnection<ConnectionManager<SqliteConnection>>;

/// Sets SQLite PRAGMAs on each new connection from the pool.
#[derive(Debug)]
struct SqlitePragmas;

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for SqlitePragmas {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        // High-concurrency SQLite setup. Multiple background writers share this
        // pool (heartbeat, InfrastructureSignal subscriber, reconcile controller,
        // import-handler drain) alongside HTTP request handlers and bulk content
        // seeding, so we need WAL + a timeout long enough to absorb the worst
        // overlap window without surfacing SQLITE_BUSY to clients.
        //
        // ORDER IS LOAD-BEARING: busy_timeout MUST be set first.
        // r2d2 establishes connections concurrently during pool warm-up. Setting
        // journal_mode = WAL acquires a write lock on the database file. If
        // busy_timeout is still at SQLite's default of 0 when that lock is
        // contested, the second concurrent on_acquire returns SQLITE_BUSY
        // immediately instead of waiting, causing pool initialisation to fail and
        // the process to exit (crashloop). Setting busy_timeout first means every
        // subsequent lock-needing pragma will wait up to 30 s rather than error.
        for pragma in [
            "PRAGMA busy_timeout = 30000",
            "PRAGMA journal_mode = WAL",
            "PRAGMA synchronous = NORMAL",
        ] {
            diesel::sql_query(pragma)
                .execute(conn)
                .map_err(diesel::r2d2::Error::QueryError)?;
        }
        Ok(())
    }
}

/// Initialize a Diesel connection pool
pub fn init_pool(database_url: &str) -> Result<DbPool, StorageError> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);

    Pool::builder()
        .max_size(10)
        .connection_timeout(Duration::from_secs(30))
        .connection_customizer(Box::new(SqlitePragmas))
        .build(manager)
        .map_err(|e| StorageError::Internal(format!("Failed to create connection pool: {}", e)))
}

/// Initialize a Diesel connection pool from storage directory
pub fn init_pool_from_dir(storage_dir: &Path) -> Result<DbPool, StorageError> {
    let db_path = storage_dir.join("content.db");
    let database_url = db_path.to_string_lossy().to_string();

    info!("Initializing Diesel connection pool at {:?}", db_path);
    let pool = init_pool(&database_url)?;

    // Run pending migrations on startup
    run_migrations(&pool)?;

    Ok(pool)
}

/// Run pending database migrations
pub fn run_migrations(pool: &DbPool) -> Result<(), StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

    info!("Running database migrations...");
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| StorageError::Internal(format!("Failed to run migrations: {}", e)))?;
    info!("Database migrations complete");

    Ok(())
}

/// App-scoped database handle using Diesel connection pool
pub struct AppScopedDb {
    pool: DbPool,
    ctx: AppContext,
}

impl AppScopedDb {
    /// Create a new app-scoped database handle
    pub fn new(pool: DbPool, h_app_id: impl Into<String>) -> Self {
        Self {
            pool,
            ctx: AppContext::new(h_app_id),
        }
    }

    /// Get a connection from the pool
    pub fn conn(&self) -> Result<PooledConn, StorageError> {
        self.pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
    }

    /// Get the app context
    pub fn context(&self) -> &AppContext {
        &self.ctx
    }

    /// Get app-scoped stats
    pub fn stats(&self) -> Result<DbStats, StorageError> {
        let mut conn = self.conn()?;

        let content_count = content_diesel::content_count(&mut conn, &self.ctx)?;
        let tag_count = content_diesel::tag_count(&mut conn, &self.ctx)?;

        Ok(DbStats {
            content_count: content_count as u64,
            unique_tags: tag_count as u64,
        })
    }
}

// Re-export Diesel types
pub mod diesel_types {
    pub use super::content_diesel::{BulkResult, ContentQuery, CreateContentInput};
}

// ============================================================================
// Regression tests — SqlitePragmas ordering
// ============================================================================
//
// These tests live here (not in tests/) so they compile against the lib
// binary, which already supplies the getrandom custom backend symbol.
// Integration test binaries would fail to link because tempfile pulls in
// getrandom as a transitive dep without that symbol defined.

#[cfg(test)]
mod pragma_order_tests {
    use super::{CustomizeConnection, SqliteConnection, SqlitePragmas};
    use diesel::prelude::*;

    /// Read-back helper: returns the current `busy_timeout` PRAGMA value for
    /// a connection using the `pragma_busy_timeout` virtual table.
    fn read_busy_timeout(conn: &mut SqliteConnection) -> i32 {
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            timeout: i32,
        }
        diesel::sql_query("SELECT timeout AS timeout FROM pragma_busy_timeout")
            .get_result::<Row>(conn)
            .expect("reading PRAGMA busy_timeout via virtual table")
            .timeout
    }

    /// After `SqlitePragmas::on_acquire` the connection must report
    /// `busy_timeout = 30000`. Single-threaded; cannot be flaky.
    #[test]
    fn busy_timeout_is_30000_after_on_acquire() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_str().unwrap();

        let mut conn = SqliteConnection::establish(path).expect("open connection");
        SqlitePragmas
            .on_acquire(&mut conn)
            .expect("on_acquire should succeed on an uncontested DB");

        let timeout = read_busy_timeout(&mut conn);
        assert_eq!(
            timeout, 30000,
            "busy_timeout must be 30000 after on_acquire; got {}",
            timeout
        );
    }

    /// Second `on_acquire` on an already-WAL DB must succeed without any
    /// lock contention. In production, the FIRST pool connection promotes
    /// the DB to WAL; every subsequent connection calls `journal_mode = WAL`
    /// and gets back "wal" (a no-op) without needing a write lock. This test
    /// verifies that the full `on_acquire` sequence succeeds for both the
    /// first (WAL promotion) and second (no-op check) connections, and that
    /// both report `busy_timeout = 30000` afterward.
    #[test]
    fn on_acquire_succeeds_for_both_first_and_second_pool_connection() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_str().unwrap();

        // First connection: promotes the DB to WAL mode.
        let mut conn_a = SqliteConnection::establish(path).expect("open first connection");
        SqlitePragmas
            .on_acquire(&mut conn_a)
            .expect("on_acquire must succeed for the first (WAL-promoting) connection");

        // Second connection: DB is already in WAL mode; `journal_mode = WAL`
        // returns the current mode without acquiring a write lock.
        let mut conn_b = SqliteConnection::establish(path).expect("open second connection");
        SqlitePragmas
            .on_acquire(&mut conn_b)
            .expect("on_acquire must succeed for the second (already-WAL) connection");

        // Both connections must report busy_timeout = 30000.
        assert_eq!(
            read_busy_timeout(&mut conn_a),
            30000,
            "first connection: busy_timeout must be 30000 after on_acquire"
        );
        assert_eq!(
            read_busy_timeout(&mut conn_b),
            30000,
            "second connection: busy_timeout must be 30000 after on_acquire"
        );
    }

    /// `journal_mode = WAL` FIRST (old broken order): with `busy_timeout`
    /// still at 0, a connection that loses the write-lock race returns
    /// `SQLITE_BUSY` immediately.
    ///
    /// This test is a deterministic proof of the original defect.
    /// It MUST observe an error — passing would mean the defect isn't
    /// reproducible in this environment.
    #[test]
    fn old_order_wal_fails_immediately_under_write_lock() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_str().unwrap();

        // Prime the file.
        {
            let mut init = SqliteConnection::establish(path).expect("open init connection");
            diesel::sql_query("CREATE TABLE IF NOT EXISTS _init (x INTEGER)")
                .execute(&mut init)
                .expect("create init table");
        }

        // Victim: busy_timeout = 0 (old order — not yet set before WAL attempt).
        let mut victim = SqliteConnection::establish(path).expect("open victim connection");
        diesel::sql_query("PRAGMA busy_timeout = 0")
            .execute(&mut victim)
            .expect("set busy_timeout = 0 on victim");

        // Blocker: hold the write lock.
        let mut blocker = SqliteConnection::establish(path).expect("open blocker connection");
        diesel::sql_query("PRAGMA busy_timeout = 0")
            .execute(&mut blocker)
            .expect("set busy_timeout = 0 on blocker");
        diesel::sql_query("BEGIN IMMEDIATE")
            .execute(&mut blocker)
            .expect("BEGIN IMMEDIATE on blocker");

        // WAL promotion with busy_timeout = 0 and the write lock held.
        // Must fail immediately (SQLITE_BUSY).
        let result = diesel::sql_query("PRAGMA journal_mode = WAL").execute(&mut victim);

        // `_blocker` still alive — lock still held — so this must be an error.
        assert!(
            result.is_err(),
            "OLD ORDER proof: journal_mode = WAL must fail with SQLITE_BUSY \
             when busy_timeout = 0 and a write lock is held. If this passes \
             the test environment does not reproduce WAL write-lock contention. \
             Got Ok({})",
            result.unwrap()
        );

        // Explicit drop for clarity — lock released here.
        drop(blocker);
    }
}
