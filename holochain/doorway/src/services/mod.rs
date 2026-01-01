//! Services layer for Doorway
//!
//! This module contains business logic services that coordinate between
//! the cache layer, projection layer, and external APIs.
//!
//! ## Services
//!
//! - **Custodian**: P2P blob distribution and custodian selection
//! - **Verification**: SHA256 blob integrity verification
//! - **Recording**: WebRTC to blob recording pipeline
//! - **ShardResolver**: Native Holochain blob resolution via elohim-storage
//! - **ImportOrchestrator**: Batch import processing (elohim-store → zome)
//! - **ImportConfig**: Zome-declared import capability discovery
//! - **Discovery**: Runtime discovery of zome capabilities from conductor
//! - **RouteRegistry**: Dynamic route management from DNAs and external agents
//! - **DIDResolver**: W3C DID resolution for doorway federation

pub mod custodian;
pub mod did_resolver;
pub mod discovery;
pub mod import_client;
pub mod import_config;
pub mod import_orchestrator;
pub mod recording;
pub mod route_registry;
pub mod shard_resolver;
pub mod storage_registration;
pub mod verification;

pub use custodian::{
    CommitmentStatus, CustodianBlobCommitment, CustodianCapability, CustodianSelectionCriteria,
    CustodianService, CustodianServiceConfig, CustodianStats, HealthProbeResult, ReachLevel,
    spawn_health_probe_task,
};
pub use recording::{
    AudioCodec, ContainerFormat, RecordingCmd, RecordingConfig, RecordingError,
    RecordingService, RecordingServiceConfig, RecordingSession, RecordingStatus,
    RecordingStatusResponse, VideoCodec, spawn_recording_cleanup_task,
};
pub use shard_resolver::{
    BlobResolution, ResolvedBlob, ResolverStats, ShardLocation, ShardManifest,
    ShardResolver, ShardResolverConfig, ShardResolverError,
};
pub use verification::{
    compute_sha256, StreamingHasher, VerificationConfig, VerificationService, VerifyBlobRequest,
    VerifyBlobResponse,
};
pub use import_config::{
    DnaImportConfig, ImportConfigDiscovery, ImportConfigStore,
    ImportConfig, ImportBatchType, IMPORT_CONFIG_FN,
};
pub use import_orchestrator::{
    BlobStore, ChunkResult, ImportError, ImportOrchestrator, ImportOrchestratorConfig,
    ImportProgress, ImportStatus, InMemoryBlobStore, StartImportInput, StartImportOutput,
    ZomeClient,
};
pub use discovery::{
    CellInfo, DiscoveryConfig, DiscoveryResult, DiscoveryService,
    spawn_discovery_task, spawn_discovery_task_with_routes,
};
pub use route_registry::{
    AgentRouteEntry, CompiledRoute, RouteRegistry, RouteRegistryConfig, RouteRegistryStats,
    RouteSource, RouteTarget, spawn_cleanup_task as spawn_route_cleanup_task,
};
pub use import_client::{ImportClient, ImportClientConfig};
pub use storage_registration::{
    StorageRegistrationConfig, StorageRegistrationResult, register_local_storage,
};
pub use did_resolver::{
    DIDDocument, DIDResolver, DIDResolverConfig, DIDResolverError, DIDResolverStats,
    Service as DIDService, VerificationMethod, create_resolver, create_resolver_with_config,
};
