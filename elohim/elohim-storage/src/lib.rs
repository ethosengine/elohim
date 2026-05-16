//! Elohim Storage - Blob storage sidecar for Elohim nodes
//!
//! Runs alongside Holochain conductor to provide scalable blob storage.
//!
//! ## Architecture
//!
//! - **Holochain DNA**: Stores metadata/provenance (BlobEntry)
//! - **elohim-storage**: Stores actual blob data locally
//! - **P2P Network**: Request/transfer blobs between nodes via rust-libp2p
//!
//! ## Why Separate from Holochain DHT?
//!
//! | Problem with DHT | Solution with Sidecar |
//! |------------------|----------------------|
//! | 16MB entry limit | Unlimited local storage |
//! | 10-50x gossip amplification | Direct P2P transfer |
//! | Memory pressure on conductor | Separate process |
//! | Slow for large files | Optimized streaming |
//!
//! ## Storage Layout
//!
//! ```text
//! ~/.elohim-storage/
//! ├── blobs/                 # Content-addressed blob storage
//! │   ├── sha256-abc123...   # First 2 chars of hash as subdirs
//! │   └── sha256-def456...
//! ├── meta.sled/             # Metadata database
//! ├── identity.key           # libp2p keypair (if p2p feature enabled)
//! └── config.toml            # Configuration
//! ```
//!
//! ## Features
//!
//! - `p2p` - Enable rust-libp2p for P2P shard transfer

// Custom getrandom v0.3 backend for holochain dependencies
// (must be declared before any modules that use randomness)
mod getrandom_custom;

// Protocol enum constants — AUTO-GENERATED from JSON schemas
pub mod generated_enums;

// Enriched API endpoints (controller layer for /api/v1/*)
pub mod api;

// Core modules (always available)
pub mod blob_store;
pub mod conductor; // Conductor process manager — spawns/monitors holochain binary
pub mod conductor_client; // Legacy: kept for backward compatibility during migration
pub mod config;
pub mod dag_store;
pub mod db; // SQLite content storage
pub mod epr_atom_service;
pub mod epr_codec;
pub mod epr_head;
pub mod epr_service;
pub mod error;
pub mod graduation; // Graduation evaluator — observations to attestations/events

// Graph-native projection substrate (CozoDB embedded; default-on)
#[cfg(feature = "graph-native")]
pub mod graph;

// Graph-native view builders — lamad + shefa domains (CozoDB projection; default-on)
#[cfg(feature = "graph-native")]
pub mod graph_views;

// GraphQL surface — Apollo Federation v2 subgraph over the graph-native projection.
// Hand-rolled hyper handler; no axum dependency.
#[cfg(feature = "graph-native")]
pub mod graphql;

pub mod happ_manager;
pub mod hc_client;
pub mod hc_client_registry;
pub mod http;
pub mod identity_handshake_service;
pub mod import_handler;
pub mod metadata;
pub mod observation; // Observation/Event Layer — peer-witnessed evidence (Track 2 substrate)
pub mod rea_projection; // REA projection signal handler (DHT → SQLite sync)
pub mod shard_service;
pub mod sharding;
pub mod signals; // Official holochain_client wrapper with signing support
pub mod signing; // ConductorSigningClient — EPR Phase 2B Task C.1 (imagodei sign_for_agent wrapper)
pub mod trust_service;
pub mod view_fed_service;
pub mod write_through; // Write-through flag state — EPR Phase 2B Task C.4 (4-layer override stack)
pub use hc_client::{ConductorHealth, HcClient, HcClientConfig, NetworkHealth, StorageHealth};
pub mod cell_discovery;
pub mod debug_stream;
pub mod import_api;
pub mod node_registry_api;
pub mod progress_hub;
pub mod progress_ws;
pub mod services;
pub mod sse;

// Cache stream for projection warm-up (SSE)
pub mod cache_stream;

// Reconcile controller — Principle P1 (DHT as manifest, eager projection)
pub mod reconcile;

// EPR projector — consumes epr_atoms, projects into pillar read-models (EPR 2B Decision #3)
pub mod projector;

// Tally strategies for multi-mechanism voting
pub mod tally;

// Sensemaking: opinion clustering for governance deliberation
pub mod sensemaking;

// Recovery — optional cryptographic share assembly (G.1/G.2 attestation-consolidation sprint)
pub mod recovery;

// View types for HTTP API responses (camelCase serialization for TypeScript)
pub mod views;

// Three-pillar trust verification via conductor (DHT credential checks)
pub mod trust_verification;

// P2P identity and discovery (always available, but some types require p2p feature)
pub mod content_server;
pub mod identity;

// P2P network modules (require p2p feature)
#[cfg(feature = "p2p")]
pub mod p2p;

// SSR endpoint handlers (require ssr feature — opt-in, V8 is expensive)
#[cfg(feature = "ssr")]
pub mod ssr;

// Parallel iroh-based P2P stack (staged cutover; see plan
// genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md).
#[cfg(feature = "p2p-iroh")]
pub mod p2p_iroh;

// Blob backend router — selects iroh vs libp2p per GET /blob/{hash} request.
// Pure selection function; no I/O. Gated on p2p-iroh since it uses peer_map types.
#[cfg(feature = "p2p-iroh")]
pub mod http_blob_router;

// CRDT sync module (always available, P2P transport requires p2p feature)
pub mod sync;

// Sovereignty and cluster modules
#[cfg(feature = "p2p")]
pub mod cluster;
pub mod sovereignty;

// Peer policy engine (operator-declared availability & capability flags)
pub mod policy;

// Heartbeat task — periodic policy eval + PeerStatus publication
pub mod heartbeat;

// TCP forwarder — external-facing listener piped to localhost conductor
pub mod forwarder;

// Test utilities (public for integration test binaries in tests/)
pub mod test_util;

// Re-exports
pub use blob_store::BlobStore;
pub use config::Config;
pub use debug_stream::{DebugBroadcaster, DebugEvent};
pub use error::StorageError;
pub use http::HttpServer;
pub use import_handler::{ImportHandler, ImportHandlerConfig, ImportProgress};
pub use metadata::MetadataDb;
pub use node_registry_api::{NodeRegistryApi, ShardAssignment, ShardStatus, ShardingStrategy};
pub use progress_hub::{ProgressHub, ProgressHubConfig, ProgressMessage};
pub use sharding::{ShardConfig, ShardEncoder, ShardManifest};

// P2P re-exports
pub use content_server::{ContentServerBridge, ContentServerConfig, PublisherInfo};
pub use identity::{NodeCapabilities, NodeIdentityInfo};

#[cfg(feature = "p2p")]
pub use identity::NodeIdentity;
#[cfg(feature = "p2p")]
pub use p2p::{
    FederationError, P2PConfig, P2PHandle, P2PNode, P2PStatusInfo, PeerInfoView, PeerListView,
    RelayMode,
};

// Sync re-exports
pub use sync::{
    DocStore, DocStoreConfig, StoredDocument, StreamPosition, StreamTracker, SyncManager,
};

// SQLite re-exports
pub use db::DbStats;

// Policy cache re-exports
pub use db::policy_cache::{
    CachedPolicy, ContentMetadata, PolicyCache, PolicyDecision, PolicyEnforcement, PolicyEvent,
    PolicyEventType, TimeAccessDecision, TimeWindow,
};

// Service re-exports
pub use services::{
    ContentService, EventBus, KnowledgeService, RelationshipService, Services, StorageEvent,
};

// View re-exports (used by schema contract tests and external callers)
pub use views::ChallengeOutcomeView;
pub use views::ElohimReputationProfileView;
pub use views::GateDecisionAttestationView;
pub use views::GateDecisionChallengeView;
pub use views::PutBlobResponseView;
pub use views::{
    build_peer_status_view, load_elohim_capability_from_env, load_render_capability_from_url,
    load_render_capability_from_url_blocking, BundleEntry, CapabilityExtensionEntry,
    CapabilityExtensions, CommittedResources, DeviceEntryView, ElohimCapabilityProfile,
    HouseholdDevicesView, HouseholdResilienceDetails, HouseholdResilienceView, NetworkPostureView,
    NodeShapeView, PeerStatusView, RenderCapabilityProfile, RendererKind,
};
pub use views::{AttestationView, GovernanceActionTallyView, GovernanceActionView};
pub use views::{
    EprCouplingView, EprEnvelopeView, EprListView, EprProvidersView, EprPublishInput,
    EprSignatureView, EprVerifyErrorView, EprVerifyView, EprView,
};
pub use views::{IrohTransportProfileView, Libp2pTransportProfileView, PeerTransportManifestView};
pub use views::{
    PlacementGapView, RegionalDistributionView, ResilienceSnapshotDetailsView,
    ResilienceSnapshotView, StewardingCollectiveEntry,
};
