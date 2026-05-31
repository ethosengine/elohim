//! # elohim-edge-presence
//!
//! WAN presence registration for Elohim doorway and steward nodes.
//!
//! This crate lets any `doorway-service` or `steward/node` process:
//!
//! 1. Discover its current public WAN IPv4 address.
//! 2. Register a stable DNS hostname by updating an A record via a pluggable
//!    [`DnsProvider`] (Cloudflare first; DuckDNS/No-IP planned).
//! 3. Obtain and auto-renew a TLS certificate via ACME/Let's Encrypt (DNS-01
//!    challenge by default; HTTP-01 optional fallback).
//! 4. Serve HTTPS in-process via a hot-reloadable [`TlsAcceptorHandle`] that
//!    wraps the caller's `tokio::net::TcpStream` — no restart needed on cert
//!    renewal.
//!
//! # k8s-ingress safety
//!
//! The default [`TlsMode`] is [`TlsMode::DeferToIngress`]. Existing deployments
//! behind k8s NGINX ingress + cert-manager are **never affected** unless they
//! explicitly set `TlsMode::TerminateInProcess`. This is a hard invariant of
//! the crate design.
//!
//! # pkarr relationship
//!
//! pkarr (in-tree at `doorway/doorway-service/src/services/pkarr_resolver.rs`)
//! is the **peer-discovery layer**: signed records on a P2P DHT, consumed by
//! protocol peers (elohim nodes, libp2p/iroh transports).
//!
//! This crate is the **browser-facing layer**: traditional DNS A records +
//! ACME-issued TLS, consumed by browsers and standard HTTP clients that cannot
//! resolve pkarr names.
//!
//! They are **complementary**. A household doorway SHOULD register with both:
//! pkarr for peer discovery; DNS A + cert for browser access.
//!
//! # Crate placement
//!
//! `crates/elohim-edge-presence/` — consistent with other `crates/` members
//! (`elohim-sdk`, `elohim-storage-client`). Consumed via:
//! ```toml
//! elohim-edge-presence = { path = "../../crates/elohim-edge-presence" }
//! ```
//! in `doorway/doorway-service/Cargo.toml` and `steward/node/Cargo.toml`.
//!
//! # Build note
//!
//! This is a **native** crate. Always build with `RUSTFLAGS=""` (the repo
//! environment sets `--cfg getrandom_backend="custom"` for Holochain WASM
//! builds, which breaks native Rust compilation).
//!
//! # Phase status
//!
//! This is a **Phase 1 scaffold**. The public API surface is production-ready;
//! most method bodies are stubs (`todo!()` or error-returning). See the design
//! spec for the phase-by-phase implementation checklist.
//!
//! Phases that are NOT yet implemented:
//! - Phase 2: WAN IP cascade (real reqwest calls) + Cloudflare DNS provider
//! - Phase 3: ACME issuance via `instant-acme` + Pebble integration tests
//! - Phase 4: TLS termination via `tokio-rustls` + `arc-swap` hot-reload
//! - Phase 5: doorway-service integration
//! - Phase 6: steward/node integration

pub mod acme;
pub mod config;
pub mod dns;
pub mod error;
pub mod orchestrate;
pub mod tls;
pub mod wan_ip;

// ---------------------------------------------------------------------------
// Re-exports — the crate's public surface
// ---------------------------------------------------------------------------

/// The main orchestrator type. Construct with [`EdgePresence::new`], spawn
/// [`EdgePresence::run`] as a background task.
pub use orchestrate::EdgePresence;

/// Configuration for the orchestrator.
pub use config::{AcmeChallenge, EdgePresenceConfig, TlsMode};

/// Error type returned by all public functions.
pub use error::EdgePresenceError;

/// TLS acceptor handle for in-process TLS termination.
pub use tls::TlsAcceptorHandle;

/// DNS provider trait for DDNS + ACME DNS-01.
pub use dns::DnsProvider;
