//! # `elohim-consent-grant` — a controller's consent, asked for from a terminal
//!
//! A device cannot recognize itself as someone's device. It asks; one of the
//! person's controllers approves in a portal; the approval comes back as a
//! single-use code. This crate holds the rules of that exchange so the hosted
//! portal, the native portal and the asking terminal apply the same ones.
//!
//! ```text
//! terminal                     portal (controller signed in)
//!   GrantRequest  ───────────▶ admit_request
//!   (keeps the verifier)       consent screen
//!                              ConsentRecord::agree ─ witness beat
//!                              (may pause) ─ controller's cell signs ─ issue
//!   ◀── loopback redirect, or the person pastes `code#state`
//!   Redemption    ───────────▶ admit_redemption ─▶ the record, its signatures
//!                                                  and the signed enrollment
//! ```
//!
//! ## What a consent is, and is not
//!
//! Every approval yields a [`ConsentRecord`]: what was asked, what was agreed,
//! for which device and which person. It is content-addressed, so it can be
//! held locally, shown to anyone, and signed again later without changing what
//! it is. A person may agree to less than was asked, never more.
//!
//! The record is evidence. It confers nothing by itself. What makes a device
//! recognized is the binding the controller's own cell signs and peers verify;
//! the record says that binding was asked for and agreed to.
//!
//! This flow lives on the recognition plane: it establishes whose device this
//! is. It grants no authority over any content. That is a matter of standing,
//! which is decided elsewhere.
//!
//! ## Why a seen code is useless
//!
//! The code is bound to the asking terminal twice: by the PKCE verifier only
//! that terminal holds, and by the device key the request named.
//!
//! ## One ceremony, any host
//!
//! [`ceremony`] holds the steps a portal performs. A single peer runtime runs
//! them alone, with its own node signing and [`MemoryStore`] holding the
//! delivery: only the device is needed to start. A doorway mounts the same
//! steps for people it hosts.
//!
//! Apart from [`MemoryStore`], everything here is a side-effect-free function
//! over plain data. Callers supply the time, the signing and the transport.

pub mod act;
pub mod carrier;
pub mod ceremony;
pub mod consent;
pub mod controller;
pub mod decide;
pub mod declaration;
pub mod delivery;
pub mod dpop;
pub mod enrollment;
pub mod hash_shape;
pub mod pending;
pub mod pkce;
pub mod request;
pub mod return_path;
pub mod signin;
pub mod verify;
pub mod witness;

pub use act::{ActRefusal, RequestedAct};
pub use carrier::{CarryRequest, CarryResponse};
pub use ceremony::{
    attendance, issue, redeem, settle, AgreedView, Attendance, ConsentView, ControllerTally,
    Delivered, Held, IssueRefusal, MemoryStore, Relation, ReturnTargetView, Taken,
};
pub use consent::{
    consent_message, Agreement, ConsentRecord, ConsentRefusal, ConsentSignature, SignedConsent,
    SignerRole, CONSENT_SIGNING_DOMAIN,
};
pub use controller::{ControllerStanding, StandingRefusal, StandingView};
pub use decide::{
    decide, ByAnswer, ByDeclaration, DecidedBy, Decider, Decision, NoElohimAttending,
    OWN_IDENTITY_REASON,
};
pub use declaration::{
    identifier_claim, Declaration, DeclarationConflict, DeclaredAsking, DeclaredDevice,
    DeclaredIdentity, ExistingIdentity,
};
pub use delivery::{
    admit_redemption, code_digest, DeliveryRefusal, PendingDelivery, Redemption, RedemptionRefusal,
};
pub use dpop::{
    bind_session_key, body_hash, check_proof, htu_path, read_proof, session_proof_verdict,
    verifier, AlgVerifier, BoundKey, Proof, ProofFacts, ProofRefusal, ReplaySet, SessionKey,
    IAT_WINDOW_SECS,
};
pub use enrollment::{ControllerProof, Enrollment, EnrollmentIntent, ENROLLMENT_DOMAIN};
pub use pending::{
    Ask, Dropped, Listed, Made, NodeState, PendingAsk, PendingAsks, PendingView, Speaks, SpeaksFor,
    ASK_LIFE_MICROS,
};
pub use request::{admit_request, AdmittedRequest, GrantPolicy, GrantRequest, RequestRefusal};
pub use return_path::{parse_pasted, return_target, ReturnPath, ReturnTarget};
pub use signin::{
    attend_sign_in, channel, may_make_node_sign, sign_in_channel_verdict, sign_in_key_rule,
    AttemptLimiter, Channel, SignInClaims, SignInRefusal, PLAIN_SIGN_IN_ALLOWED,
    SESSION_LIFE_MICROS,
};
pub use verify::{check_delivered, DeliveredRefusal};
pub use witness::{
    attend, AuthorizationClaims, MomentKind, Paused, Unattended, WitnessBeat, WitnessedMoment,
    Witnessing, MAX_PAUSE_REASON,
};

/// Version tag every request names. A portal refuses a request for a version
/// it does not implement, so the two sides never guess at each other's rules.
pub const GRANT_DOMAIN: &str = "elohim:consent-grant:v1";
