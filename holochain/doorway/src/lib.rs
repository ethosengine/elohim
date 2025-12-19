//! Doorway - WebSocket gateway for Elohim Holochain
//!
//! "Knock and it shall be opened" - Matthew 7:7-8
//!
//! Doorway provides authenticated WebSocket access to Holochain conductors
//! with support for multiple operators via NATS messaging and MongoDB storage.

pub mod auth;
pub mod config;
pub mod db;
pub mod hosts;
pub mod logging;
pub mod nats;
pub mod proxy;
pub mod routes;
pub mod server;
pub mod types;
pub mod worker;

pub use config::Args;
pub use server::{run, AppState};
pub use types::{DoorwayError, Result};
