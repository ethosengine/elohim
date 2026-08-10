//! # elohim-epr-rea
//!
//! The REA/ValueFlows domain layer over EPR atoms — the value-chain fabric.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md`.
//!
//! Three planes, one mechanism (the ValueFlows levels, materialized):
//! - **Knowledge**: [`model::ProcessSpec`] — a recipe: stages, edges, meaningful joints.
//! - **Plan**: [`model::Intent`] / [`model::Commitment`] — desired and promised flows.
//! - **Observation**: [`model::FlowEvent`] — what actually happened, with `fulfills` /
//!   `satisfies` edges inside the hashed bytes.
//!
//! **Limits are bounds on promises.** A [`model::Commitment`] may declare a [`model::Bound`] —
//! a ceiling on itself — and the flows that discharge the promise (`fulfills`, the edge the DHT
//! spells `bounded_by`) are the same flows that accumulate against that ceiling. Crossing the
//! band edge or the limit is therefore an event against the *commitment*, derived by
//! [`fold::fulfillment`] as `elohim_epr::algedonic` evidence whose `bound_ref` is the
//! commitment's own CID, and projected per-promise by [`store::FlowStore::open_pain`]. A promise
//! that declared no bound is never in pain — honest absence, not a zero.
//!
//! Resource is NOT an entity: anything content-addressed is a resource; resource *state*
//! is a pure fold over event history ([`fold`]). Granularity is scale-free here — what
//! *should* be metered is a governance decision bound via `.epr-meta`, never a constant
//! in this crate.
//!
//! The action vocabulary is [`elohim_epr::witness::ReaVerb`] — deliberately reused, not
//! a fourth REA action enum (reconciliation with the protocol schema enum is tracked in
//! the spec §8).

pub mod epistemic;
pub mod error;
pub mod fold;
pub mod model;
pub mod store;
pub mod walk;

pub use epistemic::{
    cite_gate, classify, fold_standing, CanonizationRef, EpistemicStanding, EpistemicStatus,
    EpistemicThresholds, ReviewEvent,
};
pub use error::{FabricError, Result};
pub use fold::{fulfillment, resource_state, FulfillmentStatus, ResourceState};
pub use model::{
    atom_cid, edge_fp, AgentRef, Bound, Commitment, CommitmentState, DepEdge, EdgeSpec, EdgeStatus,
    FlowEvent, Governor, Intent, PinnedRef, Process, ProcessSpec, ResourceSpec, StageSpec,
    ValidatorRef,
};
pub use store::{FlowRecord, FlowStore, MemoryFlowStore, SidecarFlowStore};
pub use walk::{FlowWalk, Frontier, Lineage};

// Re-export the shared vocabulary so consumers need no direct elohim-epr dep for it.
pub use elohim_epr::algedonic::{AlgedonicEvidence, AlgedonicKind};
pub use elohim_epr::witness::{Magnitude, ReaVerb};
