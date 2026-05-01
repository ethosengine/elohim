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
pub mod spatial_contexts;
pub mod steward_affinity;
pub mod stewardship_allocations;

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

// Recovery Protocol Phase 2 — M4 revocation projection (KeyRevocation + RevocationVote)
pub mod key_revocations;
pub mod revocation_votes;

// Recovery Protocol Phase 2 — M5 portal host projection (imagodei PortalHost entry, Category A)
pub mod portal_hosts;

// EPR Phase 3.5 — predecessor records (trust-compute gradient back-prop, Category C operational)
pub mod predecessor_records;

use std::path::Path;
use std::time::Duration;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::info;

use crate::error::StorageError;
pub use context::AppContext;

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
        for pragma in [
            "PRAGMA journal_mode = WAL",
            "PRAGMA synchronous = NORMAL",
            "PRAGMA busy_timeout = 30000",
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
