//! Reconcile controller — Principle P1 implementation.
//!
//! The reconcile controller is elohim-storage's "desired state" engine.
//! It subscribes to the DNA signal stream (DHT post-commit events from the
//! imagodei conductor) and eagerly projects them into the local SQLite tables,
//! maintaining integrity state without polling.
//!
//! ## Architecture
//!
//! ```text
//! imagodei DNA (post-commit)
//!     └─► RecoveryV2Signal (DNA-side enum)
//!             │
//!             │  HolochainAppSignalStream (Task A.11 — translates)
//!             ▼
//!         DnaSignal (storage-internal canonical shape)
//!             │
//!             │  ReconcileController (Task A.4 — drives)
//!             ▼
//!         SQLite projections (key_rotations, key_revocations,
//!                             agent_peer_bindings, revocation_votes)
//! ```
//!
//! ## Modules
//!
//! - [`signal_stream`] — `DnaSignal` enum + payload structs, `DnaSignalStream`
//!   trait, and stub implementations for testing.
//! - [`controller`] — `ReconcileController` k8s-style loop (Task A.4).

pub mod controller;
pub mod custody;
pub mod custody_sweep;
pub mod holochain_app_signal;
pub mod portal_host_handlers;
pub mod pubkey_timeline;
pub mod signal_stream;
pub mod sweep;

pub use controller::{ReconcileController, ReconcileError};
pub use holochain_app_signal::{AppSignalStreamError, HolochainAppSignalStream};
pub use pubkey_timeline::{PubkeyTimeline, PubkeyTimelineCache, PubkeyValidity};
pub use sweep::{sweep_on_revocation, SweepReport, REVOKED_STALE_FINGERPRINT};
