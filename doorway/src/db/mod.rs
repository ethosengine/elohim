//! Database layer for Doorway
//!
//! Provides MongoDB storage for users, API keys, and host registry.
//! Pattern adapted from holo-host/rust/util_libs/db

pub mod mongo;
pub mod schemas;

pub use mongo::{MongoClient, MongoCollection};
pub use schemas::{ApiKeyDoc, HostDoc, Metadata, UserDoc};
