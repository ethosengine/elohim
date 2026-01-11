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

// Legacy rusqlite modules (will be deprecated)
pub mod schema;
pub mod content;
pub mod paths;

// Graph/relationship modules (rusqlite, new in v3)
pub mod relationships;
pub mod knowledge_maps;
pub mod path_extensions;

// New Diesel modules with app scoping
pub mod context;
pub mod diesel_schema;
pub mod models;
pub mod content_diesel;
pub mod paths_diesel;

// Diesel modules for graph relationships and domain models
pub mod relationships_diesel;
pub mod human_relationships;
pub mod contributor_presences;
pub mod economic_events;
pub mod content_mastery;
pub mod stewardship_allocations;
pub mod local_sessions;

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool};
use rusqlite::Connection;
use tracing::{info, debug};

use crate::error::StorageError;
pub use context::AppContext;

/// SQLite database for content and paths
pub struct ContentDb {
    conn: Mutex<Connection>,
}

impl ContentDb {
    /// Open or create the content database
    pub fn open(storage_dir: &Path) -> Result<Self, StorageError> {
        let db_path = storage_dir.join("content.db");
        info!("Opening SQLite database at {:?}", db_path);

        let conn = Connection::open(&db_path)
            .map_err(|e| StorageError::Internal(format!("Failed to open SQLite: {}", e)))?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| StorageError::Internal(format!("Failed to set PRAGMA: {}", e)))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        // Initialize schema
        db.init_schema()?;

        Ok(db)
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self, StorageError> {
        debug!("Opening in-memory SQLite database");

        let conn = Connection::open_in_memory()
            .map_err(|e| StorageError::Internal(format!("Failed to open in-memory SQLite: {}", e)))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.init_schema()?;

        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock()
            .map_err(|e| StorageError::Internal(format!("Lock poisoned: {}", e)))?;

        schema::init_schema(&conn)?;

        Ok(())
    }

    /// Get a reference to the connection (for transactions)
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&Connection) -> Result<T, StorageError>,
    {
        let conn = self.conn.lock()
            .map_err(|e| StorageError::Internal(format!("Lock poisoned: {}", e)))?;
        f(&conn)
    }

    /// Execute a write operation with exclusive access
    pub fn with_conn_mut<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&mut Connection) -> Result<T, StorageError>,
    {
        let mut conn = self.conn.lock()
            .map_err(|e| StorageError::Internal(format!("Lock poisoned: {}", e)))?;
        f(&mut conn)
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<DbStats, StorageError> {
        self.with_conn(|conn| {
            let content_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM content", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

            let path_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM paths", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

            let step_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM steps", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

            let tag_count: i64 = conn
                .query_row("SELECT COUNT(DISTINCT tag) FROM content_tags", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

            Ok(DbStats {
                content_count: content_count as u64,
                path_count: path_count as u64,
                step_count: step_count as u64,
                unique_tags: tag_count as u64,
            })
        })
    }
}

/// Database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DbStats {
    pub content_count: u64,
    pub path_count: u64,
    pub step_count: u64,
    pub unique_tags: u64,
}

// Legacy re-exports (for backwards compatibility with http.rs)
// These will be deprecated once http.rs is updated to use Diesel
pub use content::{ContentRow, CreateContentInput as LegacyCreateContentInput, ContentQuery};
pub use paths::{PathRow, StepRow, CreatePathInput as LegacyCreatePathInput, CreateStepInput};

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
        self.pool.get()
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

// Re-export Diesel types (namespaced to avoid conflicts with legacy types)
pub mod diesel_types {
    pub use super::content_diesel::{CreateContentInput, ContentQuery, BulkResult};
    pub use super::paths_diesel::{CreatePathInput, CreateChapterInput, CreateStepInput, CreateAttestationInput, BulkPathResult};
}
