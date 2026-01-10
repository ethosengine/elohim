//! NATS messaging layer for Doorway
//!
//! Provides inter-service communication for multi-host routing.
//! Pattern adapted from holo-host/rust/util_libs/nats

pub mod client;
pub mod gateway;
pub mod messages;
pub mod routing;

pub use client::NatsClient;
pub use gateway::{GatewayConfig, GatewayPublisher, SessionGateway};
pub use messages::{HcWsRequest, HcWsResponse};
pub use routing::HostRouter;
