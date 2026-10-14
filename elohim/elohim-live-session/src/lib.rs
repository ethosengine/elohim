//! Live session logic for passing state between a person's machines.
//!
//! Pure by construction: no I/O, no async, no clock. Every function that depends on time
//! takes `now` (milliseconds on the caller's monotonic clock). Storage wraps this in an iroh
//! protocol handler; the hub's node process will use it unchanged.
//!
//! Passing state is held in memory only and is never decided from (concern C5). Nothing here
//! can write a row or a chain action.

pub mod frame;
pub mod register;
pub mod session_id;

pub use frame::{Body, Frame, FrameError, FORMAT_VERSION};
pub use register::{
    Applied, Dropped, Limits, Outbox, PeerState, Register, EXPIRY_MS, RENEW_INTERVAL_MS,
};
pub use session_id::SessionId;
