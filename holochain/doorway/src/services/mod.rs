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

pub mod custodian;
pub mod recording;
pub mod shard_resolver;
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
