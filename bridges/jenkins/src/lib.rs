//! # jenkins-bridge — a consuming app served by the network, never in control of it
//!
//! Jenkins is a non-native CONSUMER. The pattern this crate sets for every legacy app:
//!
//! - **Gifts flow out** as a governed offer from `collective:ethosengine/elohim` — the recipe,
//!   drift ([`drift`]) and a governance card ([`card`]). An offer is active only while a Steward
//!   who did not author it has approved it; refusing any gift is free.
//! - **Observations flow in** only as `Process` + `FlowEvent` ([`translate::Observation`]),
//!   wrapped in the general external-claim envelope ([`ExternalClaim`], shared in
//!   `elohim_epr_rea::external`): source `service:jenkins`
//!   (provenance, never an actor or payee), signature status, the lowest reach, and `in_scope_of`
//!   the offer's CID.
//! - **No dependency on the consumer.** Nothing here is called by Jenkins or waits on it; an
//!   operator runs the CLI over Jenkins' own archive.
//!
//! The translation is a library; the host that runs it is the consumer's operator.

pub mod card;
pub mod drift;
pub mod observe;
pub mod offer;
pub mod translate;

pub use drift::{drift, DeclaredStage, DriftReport, Finding};
pub use elohim_epr_rea::{ExternalClaim, SignatureStatus};
pub use observe::{
    mint_offer, observe, BridgeError, BuildInputs, Context, ObserveOutcome, Standing,
};
pub use offer::{OfferDeclaration, OfferDocument};
pub use translate::{translate, Observation, Refusal, Translation};
