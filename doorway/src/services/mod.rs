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
//! - **ElohimVerifier**: AI-assisted identity verification for disaster recovery

pub mod custodian;
pub mod did_resolver;
pub mod discovery;
pub mod elohim_verifier;
pub mod federation;
pub mod import_client;
pub mod import_config;
pub mod import_orchestrator;
pub mod recording;
pub mod route_registry;
pub mod shard_resolver;
pub mod storage_registration;
pub mod verification;
pub mod zome_caller;

pub use custodian::{
    spawn_health_probe_task, CommitmentStatus, CustodianBlobCommitment, CustodianCapability,
    CustodianSelectionCriteria, CustodianService, CustodianServiceConfig, CustodianStats,
    HealthProbeResult, ReachLevel,
};
pub use did_resolver::{
    create_resolver, create_resolver_with_config, DIDDocument, DIDResolver, DIDResolverConfig,
    DIDResolverError, DIDResolverStats, Service as DIDService, VerificationMethod,
};
pub use discovery::{
    spawn_discovery_task, spawn_discovery_task_with_routes, CellInfo, DiscoveryConfig,
    DiscoveryResult, DiscoveryService,
};
pub use elohim_verifier::{
    AnswerScore, ClientQuestion, ElohimVerifier, LearningPreferences, PathCompletion,
    QuestionAnswer, QuestionCategory, QuizScore, UserProfileData, VerificationQuestion,
    VerificationResult, MAX_ELOHIM_CONFIDENCE, MIN_ACCURACY_THRESHOLD, QUESTION_COUNT,
};
pub use federation::FederationConfig;
pub use import_client::{ImportClient, ImportClientConfig};
pub use import_config::{
    DnaImportConfig, ImportBatchType, ImportConfig, ImportConfigDiscovery, ImportConfigStore,
    IMPORT_CONFIG_FN,
};
pub use import_orchestrator::{
    BlobStore, ChunkResult, ImportError, ImportOrchestrator, ImportOrchestratorConfig,
    ImportProgress, ImportStatus, InMemoryBlobStore, StartImportInput, StartImportOutput,
    ZomeClient,
};
pub use recording::{
    spawn_recording_cleanup_task, AudioCodec, ContainerFormat, RecordingCmd, RecordingConfig,
    RecordingError, RecordingService, RecordingServiceConfig, RecordingSession, RecordingStatus,
    RecordingStatusResponse, VideoCodec,
};
pub use route_registry::{
    spawn_cleanup_task as spawn_route_cleanup_task, AgentRouteEntry, CompiledRoute, RouteRegistry,
    RouteRegistryConfig, RouteRegistryStats, RouteSource, RouteTarget,
};
pub use shard_resolver::{
    BlobResolution, ResolvedBlob, ResolverStats, ShardLocation, ShardManifest, ShardResolver,
    ShardResolverConfig, ShardResolverError,
};
pub use storage_registration::{
    register_local_storage, StorageRegistrationConfig, StorageRegistrationResult,
};
pub use verification::{
    compute_sha256, StreamingHasher, VerificationConfig, VerificationService, VerifyBlobRequest,
    VerifyBlobResponse,
};
pub use zome_caller::ZomeCaller;
