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
//!   (keeps the verifier)       consent screen ─ controller's cell signs
//!                              ConsentRecord::agree  +  PendingDelivery::issue
//!   ◀── loopback redirect, or the person pastes `code#state`
//!   Redemption    ───────────▶ admit_redemption ─▶ the record and its signatures
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
//! Everything here is a side-effect-free function over plain data. Callers
//! supply the time, the store and the transport.

pub mod act;
pub mod consent;
pub mod delivery;
pub mod hash_shape;
pub mod pkce;
pub mod request;
pub mod return_path;

pub use act::{ActRefusal, RequestedAct};
pub use consent::{Agreement, ConsentRecord, ConsentRefusal, ConsentSignature, SignedConsent};
pub use delivery::{
    admit_redemption, code_digest, DeliveryRefusal, PendingDelivery, Redemption, RedemptionRefusal,
};
pub use request::{admit_request, AdmittedRequest, GrantPolicy, GrantRequest, RequestRefusal};
pub use return_path::{parse_pasted, return_target, ReturnPath, ReturnTarget};

/// Version tag every request names. A portal refuses a request for a version
/// it does not implement, so the two sides never guess at each other's rules.
pub const GRANT_DOMAIN: &str = "elohim:consent-grant:v1";
