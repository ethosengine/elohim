//! Service layer for elohim-storage
//!
//! Services encapsulate business logic between HTTP handlers and repositories.
//! Each service wraps database operations with:
//! - Input validation
//! - Cross-entity orchestration
//! - Event emission for audit/notifications
//! - Transaction boundaries
//!
//! ## Architecture
//!
//! ```text
//! HTTP Handlers (thin)
//!     ↓
//! Service Layer (business logic)
//!     ↓
//! Repository Layer (db/*.rs)
//!     ↓
//! SQLite Database
//! ```

pub mod response;
pub mod events;
pub mod content_service;
pub mod path_service;
pub mod relationship_service;
pub mod knowledge_service;

// Re-exports
pub use response::*;
pub use events::{EventBus, StorageEvent, EventListener};
pub use content_service::ContentService;
pub use path_service::PathService;
pub use relationship_service::RelationshipService;
pub use knowledge_service::KnowledgeService;

use crate::db::ContentDb;
use std::sync::Arc;

/// Service container for dependency injection
///
/// Holds all services with shared database connection.
/// Pass this to HttpServer for handler access.
pub struct Services {
    pub content: Arc<ContentService>,
    pub path: Arc<PathService>,
    pub relationship: Arc<RelationshipService>,
    pub knowledge: Arc<KnowledgeService>,
    pub events: Arc<EventBus>,
}

impl Services {
    /// Create all services with shared database
    pub fn new(content_db: Arc<ContentDb>) -> Self {
        let events = Arc::new(EventBus::new());

        Self {
            content: Arc::new(ContentService::new(content_db.clone(), events.clone())),
            path: Arc::new(PathService::new(content_db.clone(), events.clone())),
            relationship: Arc::new(RelationshipService::new(content_db.clone(), events.clone())),
            knowledge: Arc::new(KnowledgeService::new(content_db.clone(), events.clone())),
            events,
        }
    }

    /// Create services without event bus (for testing)
    pub fn new_without_events(content_db: Arc<ContentDb>) -> Self {
        let events = Arc::new(EventBus::new());

        Self {
            content: Arc::new(ContentService::new(content_db.clone(), events.clone())),
            path: Arc::new(PathService::new(content_db.clone(), events.clone())),
            relationship: Arc::new(RelationshipService::new(content_db.clone(), events.clone())),
            knowledge: Arc::new(KnowledgeService::new(content_db.clone(), events.clone())),
            events,
        }
    }
}
