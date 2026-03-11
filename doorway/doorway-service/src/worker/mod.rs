//! Worker module - Request processing for Holochain
//!
//! Provides two modes:
//! - **Pool mode**: In-process worker pool (single-node, no external deps)
//! - **NATS mode**: JetStream-based distributed workers (multi-node)
//!
//! Pool mode is used automatically when NATS isn't available.

pub mod conductor;
pub mod pool;
pub mod processor;
pub mod zome_call;

pub use conductor::ConductorConnection;
pub use pool::{PoolConfig, PoolMetrics, WorkerPool};
pub use processor::{
    Worker, WorkerConfig, WorkerRequest, WorkerResponse, CONSUMER_NAME_PREFIX, STREAM_NAME,
    SUBJECT_PREFIX,
};
pub use zome_call::{
    DoorwayBatchInput, DoorwayGetInput, DoorwayWriteInput, RequesterIdentity, ZomeCallBuilder,
    ZomeCallConfig,
};
