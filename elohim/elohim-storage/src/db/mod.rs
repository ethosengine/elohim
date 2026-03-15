//! SQLite database module for structured content storage
//!
//! This module provides fast local storage for content and paths,
//! replacing DHT-based content storage for better performance.
//!
//! ## Architecture
//!
//! - Content bodies stored in blob_store (content-addressed)
//! - Content metadata, paths, steps, tags stored in SQLite
//! - DHT used only for attestations and agent-centric data
//! - All operations are app-scoped for multi-tenant isolation
//!
//! ## Tables
//!
//! - `apps` - Registered apps for multi-tenancy
//! - `content` - Content metadata (id, app_id, title, type, blob_hash)
//! - `paths` - Learning paths
//! - `steps` - Path steps referencing content
//! - `chapters` - Optional grouping within paths
//! - `content_tags` - Tag index for fast lookup
//! - `path_tags` - Path tag index
//! - `path_attestations` - Attestations granted upon path completion

// Diesel modules with app scoping
pub mod content_diesel;
pub mod context;
pub mod diesel_schema;
pub mod models;
pub mod paths_diesel;

// Diesel modules for graph relationships and domain models
pub mod agreements;
pub mod collectives;
pub mod comments;
pub mod content_mastery;
pub mod contributor_presences;
pub mod device_policies;
pub mod economic_events;
pub mod human_relationships;
pub mod humans;
pub mod imagodei_observations;
pub mod knowledge_maps_diesel;
pub mod local_sessions;
pub mod path_extensions_diesel;
pub mod rea_commitments;
pub mod relationships_diesel;
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

// Stewarded node topology (node registry + node-human stewardship)
pub mod stewarded_nodes;

// Policy cache for stewardship enforcement
pub mod policy_cache;

use std::path::Path;
use std::time::Duration;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool};
use tracing::info;

use crate::error::StorageError;
pub use context::AppContext;

/// Database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DbStats {
    pub content_count: u64,
    pub path_count: u64,
    pub step_count: u64,
    pub unique_tags: u64,
}

// ============================================================================
// Diesel Connection Pool
// ============================================================================

/// Type alias for Diesel connection pool
pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Type alias for pooled connection
pub type PooledConn = r2d2::PooledConnection<ConnectionManager<SqliteConnection>>;

/// Initialize a Diesel connection pool
pub fn init_pool(database_url: &str) -> Result<DbPool, StorageError> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);

    Pool::builder()
        .max_size(10)
        .connection_timeout(Duration::from_secs(30))
        .build(manager)
        .map_err(|e| StorageError::Internal(format!("Failed to create connection pool: {}", e)))
}

/// Initialize a Diesel connection pool from storage directory
pub fn init_pool_from_dir(storage_dir: &Path) -> Result<DbPool, StorageError> {
    let db_path = storage_dir.join("content.db");
    let database_url = db_path.to_string_lossy().to_string();

    info!("Initializing Diesel connection pool at {:?}", db_path);
    init_pool(&database_url)
}

/// App-scoped database handle using Diesel connection pool
pub struct AppScopedDb {
    pool: DbPool,
    ctx: AppContext,
}

impl AppScopedDb {
    /// Create a new app-scoped database handle
    pub fn new(pool: DbPool, app_id: impl Into<String>) -> Self {
        Self {
            pool,
            ctx: AppContext::new(app_id),
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
        let path_count = paths_diesel::path_count(&mut conn, &self.ctx)?;
        let step_count = paths_diesel::total_step_count(&mut conn, &self.ctx)?;
        let tag_count = content_diesel::tag_count(&mut conn, &self.ctx)?;

        Ok(DbStats {
            content_count: content_count as u64,
            path_count: path_count as u64,
            step_count: step_count as u64,
            unique_tags: tag_count as u64,
        })
    }
}

// Re-export Diesel types
pub mod diesel_types {
    pub use super::content_diesel::{BulkResult, ContentQuery, CreateContentInput};
    pub use super::paths_diesel::{
        BulkPathResult, CreateAttestationInput, CreateChapterInput, CreatePathInput,
        CreateStepInput,
    };
}
