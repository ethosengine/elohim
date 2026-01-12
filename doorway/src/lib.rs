//! Doorway - WebSocket gateway for Elohim Holochain
//!
//! "Knock and it shall be opened" - Matthew 7:7-8
//!
//! Doorway provides authenticated WebSocket access to Holochain conductors
//! with support for multiple operators via NATS messaging and MongoDB storage.
//!
//! ## Services
//!
//! - **Gateway**: WebSocket proxy to Holochain admin and app interfaces
//! - **Bootstrap**: Agent discovery service for DHT networks
//! - **Signal**: WebRTC signaling relay (SBD protocol)
//! - **Cache**: In-memory content caching for REST API
//! - **Projection**: DHT → MongoDB projection engine for fast reads
//! - **Orchestrator**: Plug-n-play node management with mDNS discovery

pub mod auth;
pub mod bootstrap;
pub mod cache;
pub mod config;
pub mod custodial_keys;
pub mod db;
pub mod hosts;
pub mod logging;
pub mod nats;
pub mod orchestrator;
pub mod projection;
pub mod proxy;
pub mod routes;
pub mod server;
pub mod services;
pub mod signal;
pub mod signing;
pub mod types;
pub mod worker;

pub use config::Args;
pub use server::{run, AppState};
pub use types::{DoorwayError, Result};
