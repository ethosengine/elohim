//! Live session logic for passing state between a person's machines.
//!
//! Pure by construction: no I/O, no async, no clock. Every function that depends on time
//! takes `now` (milliseconds on the caller's monotonic clock). Storage wraps this in an iroh
//! protocol handler; the hub's node process will use it unchanged.
//!
//! Passing state is held in memory only and is never decided from (concern C5). Nothing here
//! can write a row or a chain action.

//! Adapter obligations:
//! - Call `Register::connection_closed(sender)` when an authenticated connection from a sender
//!   ends. Dedup is scoped to the life of a connection; without this call a reconnecting
//!   sender (whose `Outbox` restarts at count 1) is dropped as stale and its slot is held.
//!   Never call it on silence or expiry: forgetting counts there would allow stale replay.
//! - Drive `Outbox::poll` on a timer and after each `present`, so held-back latest values and
//!   renewals go out.
//! - Pass a monotonic millisecond clock; a clock that runs backwards is outside the contract.

pub mod frame;
pub mod register;
pub mod session_id;

pub use frame::{Body, Frame, FrameError, FORMAT_VERSION};
pub use register::{
    Applied, Dropped, Limits, Outbox, OutboxError, PeerState, Register, EXPIRY_MS,
    RENEW_INTERVAL_MS,
};
pub use session_id::SessionId;
