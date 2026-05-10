//! Dual-publish gossip fan-out — Plan 4 cutover gate #4.
//!
//! Provides [`DualGossipPublisher`], a [`GossipPublisher`] implementation that
//! fans out to both the libp2p-gossipsub and iroh-gossip transports using
//! byte-identical payloads. Per-topic routing is governed by
//! [`classify_topic`]'s verdict table.
//!
//! # Modules
//!
//! - [`verdicts`] — per-topic [`TopicTransports`] enum + lookup table.
//! - [`iroh_publisher`] — iroh-backed [`GossipPublisher`] implementation.
//! - [`dual_publisher`] — fan-out wrapper owning both inner publishers.
//!
//! # Usage
//!
//! ```ignore
//! use std::sync::Arc;
//! use elohim_storage::p2p::adapters::LibP2PGossipPublisher;
//! use elohim_storage::p2p_iroh::dual_publish::{DualGossipPublisher, IrohGossipPublisher};
//! use elohim_storage::services::gossip_flood::GossipPublisher;
//!
//! let libp2p_pub = Arc::new(LibP2PGossipPublisher::new(p2p_tx));
//! let iroh_pub   = Arc::new(IrohGossipPublisher::spawn(iroh_node.gossip().clone()));
//! let publisher: Arc<dyn GossipPublisher> = Arc::new(
//!     DualGossipPublisher::new(Some(libp2p_pub), Some(iroh_pub))
//! );
//! ```

pub mod dual_publisher;
pub mod iroh_publisher;
pub mod verdicts;

pub use dual_publisher::DualGossipPublisher;
pub use iroh_publisher::IrohGossipPublisher;
pub use verdicts::{classify_topic, TopicTransports};

#[cfg(test)]
mod tests;
