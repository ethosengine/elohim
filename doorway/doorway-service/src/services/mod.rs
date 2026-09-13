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
//! - **SelfUptime**: doorway-local hourly availability heartbeat (Class C)
//! - **DIDResolver**: W3C DID resolution for doorway federation
//! - **NameRouting**: registry fold + one-hop federated relay for a name this
//!   doorway does not serve (Category C, Operational)
//! - **OwedResponse**: the redress term a refusal advertises and a challenge is
//!   witnessed under — read from the contract, never chosen by the doorway
//! - **ServeReceipt**: the fair-trade receipt — a READING of the REA
//!   commitments that already account for a serve (Category C, nothing minted)
//! - **ServeEligibility**: the reach + standing terms of the serving fold,
//!   re-asked at serve time on every cached path (Category C, Operational —
//!   nothing is stored, the fold is resolved from the live projection)
//! - **ElohimVerifier**: AI-assisted identity verification for disaster recovery

pub mod custodian;
pub mod dashboard_topology;
pub mod did_resolver;
pub mod discovery;
pub mod elohim_verifier;
pub mod federation;
pub mod import_client;
pub mod import_config;
pub mod import_orchestrator;
pub mod name_routing;
pub mod owed_response;
pub mod pkarr_resolver;
pub mod recording;
pub mod route_registry;
pub mod self_uptime;
pub mod serve_eligibility;
pub mod serve_receipt;
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
    spawn_discovery_task, spawn_discovery_task_with_routes, spawn_discovery_task_with_signal,
    CellInfo, DiscoveryConfig, DiscoveryResult, DiscoveryService,
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
pub use name_routing::{NameRouteTable, FEDERATION_HOP_HEADER, SERVED_BY_HEADER};
pub use owed_response::{
    challenge_commitment_id, due_from_responsive_reach, owed_from_contract, OwedResponse,
    CHALLENGE_ROUTE, RESPOND_TO_CHALLENGE_ACTION,
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
pub use serve_eligibility::{
    admitted_standing_value, audience_from_projection, contract_from_projection,
    fold_for_projection, serve_eligibility, stamp_admitted_standing, standing_from_request,
    AudienceTerm, CollectiveRef, ContractTerms, ReachClass, Refusal, RequesterStanding,
    ServeEligibility, ServeRequest, STANDING_HEADER, WHERE_TO_BE_HEARD,
};
pub use serve_receipt::{
    build_receipt, receipt_header_value, stamp_receipt, ChallengeOutcome, Credit, CreditRole,
    ExchangeClause, HeldClause, ProjectedClause, Receipt, RECEIPT_HEADER, RECEIPT_ROUTE_PREFIX,
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
