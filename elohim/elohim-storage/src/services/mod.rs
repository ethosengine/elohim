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

pub mod agreement_service;
pub mod anomaly_detection;
pub mod behavioral_trust;
pub mod boot_registration;
pub mod content_service;
pub mod disposition_service;
pub mod distribution;
pub mod economic_event_service;
pub mod elohim_gate;
pub mod events;
pub mod exchange_service;
pub mod governance_health;
pub mod hazard;
pub mod holochain_humans_replayer;
pub mod household_backfill;
pub mod household_resilience;
pub mod inference_engine;
pub mod inference_router;
pub mod knowledge_service;
pub mod mastery_depth;
pub mod presence_service;
pub mod provenance_service;
pub mod rea_commitment_service;
pub mod recognition_pipeline_service;
pub mod relationship_density;
pub mod relationship_service;
pub mod resource_nature;
pub mod resource_service;
pub mod response;
pub mod responsibility_demand_service;
pub mod risk_alert;
pub mod routing;
pub mod sidecar_engine;
pub mod sla_service;
pub mod spatial;
pub mod spatial_capacity;
pub mod spatial_dashboard;
pub mod steward_affinity_service;
pub mod steward_standing;
pub mod stewardship_service;
pub mod token_decay_service;
pub mod token_ledger_service;
pub mod token_mint_service;
pub mod vulnerability;
pub mod weather;

// Re-exports
pub use content_service::ContentService;
pub use economic_event_service::EconomicEventService;
pub use events::{EventBus, EventListener, StorageEvent};
pub use exchange_service::ExchangeService;
pub use knowledge_service::KnowledgeService;
pub use presence_service::PresenceService;
pub use relationship_service::RelationshipService;
pub use resource_service::ResourceService;
pub use response::*;
pub use stewardship_service::StewardshipService;

use crate::db::{context::AppContext, DbPool};
use elohim_gate::ElohimGate;
use inference_engine::InferenceEngine;
use inference_router::InferenceRouter;
use sidecar_engine::SidecarEngine;
use std::sync::Arc;

/// Service container for dependency injection
///
/// Holds all services with shared database connection pool.
/// Pass this to HttpServer for handler access.
pub struct Services {
    pub content: Arc<ContentService>,
    pub relationship: Arc<RelationshipService>,
    pub knowledge: Arc<KnowledgeService>,
    pub events: Arc<EventBus>,
    pub gate: Arc<ElohimGate>,
}

impl Services {
    /// Create all services with shared database pool
    pub fn new(pool: DbPool) -> Self {
        let events = Arc::new(EventBus::new());
        let ctx = AppContext::default_lamad();

        // Create inference router with sidecar engine
        let sidecar_url = std::env::var("ELOHIM_AGENT_URL")
            .unwrap_or_else(|_| "http://localhost:8095".to_string());
        let sidecar = Arc::new(SidecarEngine::new(
            sidecar_url,
            "gate-evaluator".to_string(),
        ));
        let router = Arc::new(InferenceRouter::new(vec![
            sidecar as Arc<dyn InferenceEngine>,
        ]));

        Self {
            content: Arc::new(ContentService::new(
                pool.clone(),
                ctx.clone(),
                events.clone(),
            )),
            relationship: Arc::new(RelationshipService::new(
                pool.clone(),
                ctx.clone(),
                events.clone(),
            )),
            knowledge: Arc::new(KnowledgeService::new(
                pool.clone(),
                ctx.clone(),
                events.clone(),
            )),
            events,
            gate: Arc::new(ElohimGate::new(router)),
        }
    }

    /// Create services without event bus (for testing)
    pub fn new_without_events(pool: DbPool) -> Self {
        let events = Arc::new(EventBus::new());
        let ctx = AppContext::default_lamad();

        Self {
            content: Arc::new(ContentService::new(
                pool.clone(),
                ctx.clone(),
                events.clone(),
            )),
            relationship: Arc::new(RelationshipService::new(
                pool.clone(),
                ctx.clone(),
                events.clone(),
            )),
            knowledge: Arc::new(KnowledgeService::new(
                pool.clone(),
                ctx.clone(),
                events.clone(),
            )),
            events,
            gate: Arc::new(ElohimGate::new_skeleton()),
        }
    }
}
