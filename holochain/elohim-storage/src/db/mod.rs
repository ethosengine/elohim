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
//!
//! ## Tables
//!
//! - `content` - Content metadata (id, title, type, tags JSON, blob_hash)
//! - `paths` - Learning paths
//! - `steps` - Path steps referencing content
//! - `content_tags` - Tag index for fast lookup

pub mod schema;
pub mod content;
pub mod paths;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use tracing::{info, debug};

use crate::error::StorageError;

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

// Re-exports
pub use content::{ContentRow, CreateContentInput, ContentQuery};
pub use paths::{PathRow, StepRow, CreatePathInput, CreateStepInput};
