//! elohim-epr — canonical codec for the Elohim EPR atom.
//!
//! See `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`.
//!
//! # Example
//!
//! ```no_run
//! use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
//! use chrono::Utc;
//!
//! let kp = AgentKeypair::from_secret(&[42u8; 32]).unwrap();
//! let agent_cid = compute_cid(&[100]);
//!
//! let epr = Epr::builder()
//!     .kind(EprKind::Manifest)
//!     .schema_ref(compute_cid(&[1]))
//!     .schema_key("app-manifest")
//!     .reach(Reach::Commons)
//!     .coupling(Coupling { governance: Some(compute_cid(&[4])), ..Default::default() })
//!     .issued_at(Utc::now())
//!     .payload(b"{}".to_vec())
//!     .sign(&kp, agent_cid)
//!     .unwrap();
//!
//! assert!(epr.verify_with_key(&kp.public_key_bytes()).is_ok());
//! ```

pub mod cbor;
pub mod cid;
pub mod coupling;
pub mod envelope;
pub mod epr;
pub mod error;
pub mod kind;
pub mod proof;
pub mod reach;
pub mod signature;
pub mod validation;

pub use coupling::Coupling;
pub use envelope::Envelope;
pub use epr::{Epr, EprBuilder};
pub use error::{EprError, Result};
pub use kind::{CouplingLeg, EprKind};
pub use proof::{sign, verify, AgentKeypair};
pub use reach::Reach;
pub use signature::Signature;
pub use validation::validate_coupling;
