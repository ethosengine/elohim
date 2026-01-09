//! Event system for storage operations
//!
//! Provides an event bus for notifying listeners about storage operations.
//! Useful for:
//! - Audit logging
//! - Cache invalidation
//! - Real-time notifications
//! - Sync triggers

use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, trace};

/// Storage events emitted by services
#[derive(Debug, Clone)]
pub enum StorageEvent {
    // Content events
    ContentCreated {
        id: String,
        title: String,
        content_type: Option<String>,
    },
    ContentUpdated {
        id: String,
    },
    ContentDeleted {
        id: String,
    },
    ContentBulkCreated {
        count: usize,
        ids: Vec<String>,
    },

    // Path events
    PathCreated {
        id: String,
        title: String,
    },
    PathUpdated {
        id: String,
    },
    PathDeleted {
        id: String,
    },
    PathBulkCreated {
        count: usize,
        ids: Vec<String>,
    },

    // Relationship events
    RelationshipCreated {
        id: String,
        source_id: String,
        target_id: String,
        relationship_type: String,
    },
    RelationshipDeleted {
        id: String,
    },
    RelationshipBulkCreated {
        count: usize,
    },

    // Knowledge map events
    KnowledgeMapCreated {
        id: String,
        map_type: String,
        owner_id: String,
    },
    KnowledgeMapUpdated {
        id: String,
    },
    KnowledgeMapDeleted {
        id: String,
    },

    // Path extension events
    PathExtensionCreated {
        id: String,
        base_path_id: String,
        extended_by: String,
    },
    PathExtensionUpdated {
        id: String,
    },
    PathExtensionDeleted {
        id: String,
    },
}

/// Trait for event listeners
pub trait EventListener: Send + Sync {
    /// Handle an event
    fn on_event(&self, event: &StorageEvent);
}

/// Event bus for broadcasting storage events
pub struct EventBus {
    sender: broadcast::Sender<StorageEvent>,
}

impl EventBus {
    /// Create a new event bus with default capacity
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a new event bus with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: StorageEvent) {
        trace!(event = ?event, "Emitting storage event");
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(event);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<StorageEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Logging event listener for audit trails
pub struct LoggingEventListener;

impl EventListener for LoggingEventListener {
    fn on_event(&self, event: &StorageEvent) {
        match event {
            StorageEvent::ContentCreated { id, title, .. } => {
                debug!(id = %id, title = %title, "Content created");
            }
            StorageEvent::ContentDeleted { id } => {
                debug!(id = %id, "Content deleted");
            }
            StorageEvent::PathCreated { id, title } => {
                debug!(id = %id, title = %title, "Path created");
            }
            StorageEvent::RelationshipCreated {
                source_id,
                target_id,
                relationship_type,
                ..
            } => {
                debug!(
                    source = %source_id,
                    target = %target_id,
                    rel_type = %relationship_type,
                    "Relationship created"
                );
            }
            _ => {
                trace!(event = ?event, "Storage event");
            }
        }
    }
}

/// Spawn a background task that logs all events
pub fn spawn_logging_listener(event_bus: Arc<EventBus>) -> tokio::task::JoinHandle<()> {
    let mut receiver = event_bus.subscribe();
    let listener = LoggingEventListener;

    tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => listener.on_event(&event),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    debug!(skipped = n, "Event listener lagged, skipped events");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("Event bus closed, stopping listener");
                    break;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_event_bus_emit_receive() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe();

        bus.emit(StorageEvent::ContentCreated {
            id: "test-1".into(),
            title: "Test Content".into(),
            content_type: Some("article".into()),
        });

        let event = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("timeout")
            .expect("receive error");

        match event {
            StorageEvent::ContentCreated { id, title, .. } => {
                assert_eq!(id, "test-1");
                assert_eq!(title, "Test Content");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_bus_no_subscribers() {
        let bus = EventBus::new();
        // Should not panic even with no subscribers
        bus.emit(StorageEvent::ContentDeleted {
            id: "test".into(),
        });
    }
}
