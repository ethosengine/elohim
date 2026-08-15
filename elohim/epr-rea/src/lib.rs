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
//! **Stocks and flows** ([`stock`]) are the dynamics layer over the same events: a level with
//! what fills and drains it, plus the derived quantities that separate dynamic equilibrium from
//! silting (turnover time) and name overshoot before it is visible (the emission/absorption and
//! harvest/regeneration indices, and the respite/response controllability ratio). Where the
//! `Bound` above is a ceiling on a promise, a stock is the physics the promise sits inside —
//! Beer's regulator and Meadows' plant, in one crate. Derived like everything else here: a
//! stock is never stored, so two peers folding the same events mint the same stock with no
//! shared clock.
//!
//! **Who is acting** ([`actor`]) is a fourth, orthogonal record: an [`actor::ActorClaim`] is an
//! honor-system identity an agent registers *for itself, in flight*, session-scoped, superseded
//! by appending rather than by mutation. It is deliberately not a field on the records above —
//! attribution written about an actor after the fact cannot be disputed by the party it names,
//! and cannot be revised mid-run. Nothing here proves the claim; what it buys is that the claim
//! now EXISTS as an addressable record, so a later attestation has something precise to agree or
//! disagree with. Its store is a separate sidecar for a load-bearing reason: a reader that
//! consults identity must be able to fail to read it and still decide.
//!
//! Resource is NOT an entity: anything content-addressed is a resource; resource *state*
//! is a pure fold over event history ([`fold`]). Granularity is scale-free here — what
//! *should* be metered is a governance decision bound via `.epr-meta`, never a constant
//! in this crate.
//!
//! The action vocabulary is [`elohim_epr::witness::ReaVerb`] — deliberately reused, not
//! a fourth REA action enum (reconciliation with the protocol schema enum is tracked in
//! the spec §8).

pub mod actor;
pub mod epistemic;
pub mod error;
pub mod fold;
pub mod model;
pub mod scope;
pub mod stock;
pub mod store;
pub mod walk;

pub use actor::{
    parse_agent_ref, ActorClaim, ActorRecord, ActorStore, MemoryActorStore, SidecarActorStore,
};
pub use epistemic::{
    cite_gate, classify, fold_standing, CanonizationRef, EpistemicStanding, EpistemicStatus,
    EpistemicThresholds, ReviewEvent,
};
pub use error::{FabricError, Result};
pub use fold::{fulfillment, resource_state, FulfillmentStatus, ResourceState};
pub use model::{
    atom_cid, edge_fp, AgentRef, Bound, Commitment, CommitmentState, Composition, DepEdge,
    EdgeSpec, EdgeStatus, FlowEvent, Governor, Intent, LimitSource, PinnedRef, Process,
    ProcessSpec, ResourceSpec, Sense, StageSpec, ValidatorRef,
};
pub use scope::{Containers, Scopes};
pub use stock::{
    respite_response, stock_over_window, stock_over_window_within, Stock, StockError, Window,
    Within,
};
pub use store::{FlowRecord, FlowStore, MemoryFlowStore, SidecarFlowStore};
pub use walk::{FlowWalk, Frontier, Lineage};

// Re-export the shared vocabulary so consumers need no direct elohim-epr dep for it.
pub use elohim_epr::algedonic::{AlgedonicEvidence, AlgedonicKind};
pub use elohim_epr::witness::{Magnitude, ReaVerb};
