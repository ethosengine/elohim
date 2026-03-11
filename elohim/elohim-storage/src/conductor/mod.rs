//! Holochain Conductor Connection Module
//!
//! This module provides a well-structured connection to the Holochain conductor.
//!
//! # Architecture
//!
//! The module is organized by concern, with each submodule having a single responsibility:
//!
//! | Module      | Responsibility                                    |
//! |-------------|---------------------------------------------------|
//! | `transport` | WebSocket connect/send/receive                    |
//! | `protocol`  | Msgpack encoding, Holochain wire format           |
//! | `auth`      | Token acquisition and connection authentication   |
//! | `session`   | An authenticated, ready-to-use connection         |
//! | `client`    | High-level client with automatic reconnection     |
//!
//! # Key Design Principles
//!
//! ## 1. Make Invalid States Unrepresentable
//!
//! - `Session` can only be created via `Session::establish()`
//! - `establish()` blocks until the session is fully ready
//! - If you have a `Session`, you can make zome calls
//!
//! ## 2. No Hidden Background Tasks
//!
//! - `ConductorClient::connect()` blocks until connected
//! - Reconnection happens lazily on next call
//! - No polling loops or race conditions
//!
//! ## 3. Clear Ownership
//!
//! - Each component owns its resources
//! - No shared mutable state between tasks
//! - Channels are used for communication, not shared flags
//!
//! # Usage
//!
//! ```ignore
//! use elohim_storage::conductor::{ConductorClient, ConductorClientConfig};
//!
//! // Create a client - blocks until connected
//! let client = ConductorClient::connect(ConductorClientConfig {
//!     admin_url: "ws://localhost:4444".to_string(),
//!     app_url: "ws://localhost:4445".to_string(),
//!     app_id: "my-app".to_string(),
//!     ..Default::default()
//! }).await?;
//!
//! // Make zome calls
//! let result = client.call_zome(
//!     &cell_id,
//!     "my_zome",
//!     "my_function",
//!     &payload,
//! ).await?;
//! ```
//!
//! # Why This Structure?
//!
//! A previous implementation had a race condition where:
//! 1. `connect()` spawned a background task and polled a flag
//! 2. The background task connected and set the flag
//! 3. If polling timed out before the flag was seen, the client was dropped
//! 4. The background task saw the channel close and entered a reconnect loop
//!
//! This structure prevents that by:
//! - Making `Session::establish()` synchronous (in async terms)
//! - Not returning until the session is fully ready
//! - Using oneshot channels for definitive signaling, not polling
//!
//! The lesson: **If "connected" is a boolean flag you poll, you have the wrong abstraction.
//! "Connected" should be a type you either have or don't have.**

// Internal modules - not exposed publicly
mod auth;
mod protocol;
mod transport;

// Public modules
mod client;
mod session;

// Re-export the public API
pub use client::{ConductorClient, ConductorClientConfig};
pub use session::{Session, SessionConfig};
