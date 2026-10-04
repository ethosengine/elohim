//! # `elohim-consent-grant` — a controller's consent, asked for from a terminal
//!
//! A device that wants to act for a person cannot grant itself that standing.
//! It asks; one of the person's controllers approves in a portal; the approval
//! comes back as a single-use code. This crate holds the rules of that exchange
//! so the hosted portal, the native portal and the asking terminal apply the
//! same ones.
//!
//! ```text
//! terminal                     portal (controller signed in)
//!   GrantRequest  ───────────▶ admit_request
//!   (keeps the verifier)       consent screen ─ controller's cell signs
//!                              issue: StoredGrant (claims) + code
//!   ◀── loopback redirect, or the person pastes `code#state`
//!   Redemption    ───────────▶ admit_redemption ─▶ the signed act
//! ```
//!
//! Every approval yields a grant, and the grant's claims say what was granted:
//! which acts, whether the participant key is bound, and until when. A person
//! may grant less than was asked. Whether the device ends up an enrolled peer
//! or holds one permission for an hour is read from the claims, not from which
//! flow was taken; there is one flow.
//!
//! The grant carries no authority of its own. What it returns is an act the
//! controller's own cell signed; peers verify that signature on-chain. The
//! portal is where consent is given and nothing more.
//!
//! The code is bound to the asking terminal twice: by the PKCE verifier only
//! that terminal holds, and by the device key the request named. A code seen
//! over a shoulder or lifted from a redirect redeems nothing.
//!
//! Everything here is a side-effect-free function over plain data. Callers
//! supply the time, the store and the transport.

pub mod act;
pub mod agent_key;
pub mod pkce;
pub mod redeem;
pub mod request;
pub mod return_path;

pub use act::{ActRefusal, RequestedAct};
pub use redeem::{
    admit_redemption, code_digest, Approval, GrantedClaims, IssueRefusal, Redemption,
    RedemptionRefusal, StoredGrant,
};
pub use request::{
    admit_request, AdmittedRequest, GrantPolicy, GrantRequest, RequestRefusal, Standing,
};
pub use return_path::{parse_pasted, return_target, ReturnPath, ReturnTarget};

/// Version tag every request names. A portal refuses a request for a version
/// it does not implement, so the two sides never guess at each other's rules.
pub const GRANT_DOMAIN: &str = "elohim:consent-grant:v1";
