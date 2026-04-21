//! elohim-epr — canonical codec for the Elohim EPR atom.
//!
//! See `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`.

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
