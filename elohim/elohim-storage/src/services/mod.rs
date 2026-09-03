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

pub mod advertiser_health; // requester-side advertiser diversity — which peers actually SERVE head-record bytes (bounded, decaying, process-local)
pub mod aggregator;
pub mod agreement_service;
pub mod anomaly_detection;
pub mod arc_actuator; // conductor authority-arc actuation core — {0,1} coverage gate + config render (spec §5)
pub mod arc_policy; // conductor authority-arc Auto policy — pure derive() (2026-06-13-conductor-authority-arc-auto-policy.md)
pub mod back_prop;
pub mod behavioral_trust;
pub mod boot_registration;
pub mod bootstrap_manifests;
pub mod bounds_validator;
pub mod capacity_pledge_author; // Explicit Class-A commons-capacity pledge via Mishpat conductor + eager projections
pub mod capacity_reporter; // Periodic local capacity reporter — measures + upserts custodian_metrics, sets the cluster-capacity gauges
pub mod cluster_view;
pub mod commitment_fetcher;
pub mod concentration_service;
pub mod conductor_commitment_author; // Slice-2b — production CommitmentAuthor (conductor-backed provide loop)
pub mod conductor_writes; // 2026-05-26-substrate-rea-replication-fix — facade for HTTP→conductor zome calls
pub mod connectivity; // Phase 4 T6 — libp2p connected-peers snapshot helper
pub mod constitutional_ratio_registry;
pub mod content_service;
pub mod contest_backoff; // F-B throughput lever — hold back contest attempts that are PREDICTABLE repeat failures (bounded, always-expiring)
pub mod contributor_reflexive_facing; // Wave 2 — "how the network sees a contributor" facing (folds in elohim-facings)
pub mod custody_facing; // custody-observation loader for the typed custody folds (elohim-facings) + the class gauges
pub mod custody_rotation; // author a successor custody pledge when the content's blob rotates under it
pub mod demand_autopin; // self-healing opportunity map row 15 — auto-pin on a local content read-miss
pub mod device_capacity; // Phase 4 T7 — available bytes helper
pub mod did_identity_store; // did:elohim assembly store — implements the did-bridge ElohimIdentityStore contract (spec §3.4)
pub mod disposition_service;
pub mod distribution;
pub mod distribution_view;
pub mod economic_event_emit_service; // Slice-2a T3 — bounds-validated conductor-path emit
pub mod economic_event_service;
pub mod elohim_gate;
pub mod epr_compose;
pub mod epr_kind;
pub mod epr_nav_context_view; // Category C read-only projection for nav-context endpoint
pub mod epr_raw_view; // Category C read-only projection for the /raw inspector (EPR-content facing Slice 2)
pub mod epr_service;
pub mod epr_store;
pub mod events;
pub mod exchange_service;
pub mod federator;
pub mod floor_protections;
pub mod gate_challenge_namespace_backfill; // level-triggered heal: re-file dark gate-challenge/outcome projections into the canonical namespace
pub mod genesis_self_heal; // OPERATOR-AUTHORIZED genesis bootstrap — self-heal own NULL agent_pub_key from own cell key (gated, NULL-only)
pub mod gossip_flood;
pub mod governance_health;
pub mod hazard;
pub mod head_adoption; // adopt-before-author — consult the substrate for an existing canonical head BEFORE minting a local root
pub mod head_batch_resolver; // L1 — batched head-plane reads (round-trip collapse at flat read-permit cost; caps and concurrency never rise)
pub mod heal_backoff; // drain lever 2 — replay a known conductor-missing answer instead of re-paying for it (bounded, always-expiring)
pub mod holochain_humans_replayer;
pub mod household_backfill;
pub mod household_resilience;
pub mod hub_capacity_service;
pub mod hub_resolver; // Wave 2 T2 — agent→hub resolver (CID-canonical, slug-alias, seed-compatible)
pub mod identity_fill; // Periodic NULL agent_pub_key fill + create-missing from DHT membership truth (fills-never-moves)
pub mod imagodei_lookup; // Phase 4 T5 — display name resolver
pub mod inference_engine;
pub mod inference_router;
pub mod knowledge_service;
pub mod lens_facing; // lens-market service layer — DB → affinity fold → LensBindingView (S5)
pub mod limit_gradient_registry;
pub mod manifest_registry;
pub mod mastery_depth;
pub mod measure;
pub mod membership_identity_reconcile; // membership-truth agent-key supersede + rekey cascade (NON-self rows)
pub mod mishpat_commitment_facing;
pub mod mishpat_mirror_backfill; // level-triggered heal: re-file/insert replicates-* mirrors into the canonical projection namespace
pub mod mutuality_audit_service;
pub mod operation_authorization; // Che op-gate Slice 1 §14 — core op-gate gate over mishpat_commitments
pub mod operational_weave_facing;
pub mod peer_capacity_service;
pub mod peer_diversity; // Phase 4 T8 — archetype-mix diversity helper
pub mod peer_selection;
pub mod peer_status_fanout; // Cross-agent PeerStatus fan-in — breaks the self-only peer_statuses ceiling
pub mod peer_topology_view;
pub mod presence_service;
pub mod private_replica; // Wave 5.1 Slice-0 — encrypt-then-erasure-code + DEK envelope PROOF
pub mod provenance_service;
pub mod provide_loop_status; // Workstream D — observability for the provide-loop + re-anchor backfill
pub mod provide_reconcile;
pub mod rate_history;
pub mod rea_commitment_service;
pub mod rea_observed_compute; // Wave 4.3 — OBSERVED-compute reciprocity adapter (consumes the 4.1 mutual_compute fold)
pub mod reach_earning;
pub mod reanchor_backfill; // Workstream D — re-author NULL-anchor content a cold-conductor seed left provenance-only
pub mod reciprocity_view;
pub mod recognition_pipeline_service;
pub mod relationship_density;
pub mod relationship_service;
pub mod release_adoption; // rung-5 §6 — adoption controller (OBSERVE mode): watch/fetch/verify followed release channels; no apply
pub mod release_attestation; // rung-5 §5 — release soak/build attestations + the promotion-threshold reader
pub mod replicates_commons_validator;
pub mod replicates_dwelling_service;
pub mod replicates_dwelling_validator;
pub mod replication_prioritizer;
pub mod republish_epr_validator;
pub mod resilience; // C2 — hub projection for /api/v1/resilience/{id}/hub
pub mod resource_nature;
pub mod resource_service;
pub mod response;
pub mod responsibility_demand_service;
pub mod risk_alert;
pub mod routing;
pub mod salvage_commitment_author; // P3-5 — production CommitmentAuthor (conductor-backed salvage)
pub mod schemaref_resolver;
pub mod sealed_against_self;
pub mod seed_shard_manifest;
pub mod self_stewardship; // honest self-held shard_locations recording (verify-before-record)
pub mod session_exchange;
pub mod session_proof; // peer-native "did this agent prove key control?" — the local-first session predicate
pub mod shard_manifest_backfill; // GAP-2b — steward-side portal session exchange (doorway handoff redemption)
pub mod sidecar_engine;
pub mod signal_weight_registry;
pub mod sla_service;
pub mod spatial;
pub mod spatial_capacity;
pub mod spatial_dashboard;
pub mod spool_custody_author; // Station 2 — custody-spool expands to per-hash custody-blob on the custodian's own conductor
pub mod spool_ingest;
pub mod standing;
pub mod standing_projector;
pub mod standing_query;
pub mod steward_affinity_service;
pub mod steward_standing;
pub mod stewardship_service;
pub mod system_metrics;
pub mod tending;
pub mod token_decay_service;
pub mod token_ledger_service;
pub mod token_mint_service;
pub mod transport_resolve; // agent_cid → libp2p PeerId resolution for the shard PUSH path
pub mod viewer_lens_facing; // imagodei viewer-relative disclosure lens (pure fold in elohim-facings)
pub mod vulnerability;
pub mod weather;

// Multi-collective collaboration EPR — M1 Phase C service layer
pub mod qahal_service;

// Multi-collective collaboration EPR — M1 Phase D share-routing evaluator (pure function)
pub mod share_routing;

// Wave-3 doorway-membrane serve-routing — capability-aware peer selection at race_fetch sites
pub mod serve_routing;

// Attestation Consolidation Sprint — projection signal handler + tally projector
pub mod attestation_projector;
pub mod tally_projector;

// Recovery Protocol Phase 2 (M4) — recovery_flows / key_revocations projection
pub mod recovery_flow_projector;

// Recovery Protocol Phase 2 (M4) — central ElohimContentSignal dispatcher that
// fans signals to AttestationProjector + RecoveryFlowProjector.
pub mod elohim_content_dispatcher;

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
