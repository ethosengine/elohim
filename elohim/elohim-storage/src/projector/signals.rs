//! Projector signal emission.
//!
//! For every successful UPSERT into a pillar read-model table the projector
//! emits a [`ProjectorSignal`] on an internal [`tokio::sync::mpsc`] channel.
//!
//! Subscribers (today and future):
//! - `/api/v1/status/projector` lag metric (B.8)
//! - elohim-agent defenders (Phase 2C)
//! - Phase 4 GraphQL subscriptions
//!
//! ## Backpressure policy
//!
//! The projector uses `try_send` and silently logs on full-channel. Subscribers
//! that fall behind do not block the projector loop.

use serde::Serialize;

/// One signal emitted per successful UPSERT into a pillar read-model table.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectorSignal {
    /// Pillar that owns the projection, e.g. `"shefa"`, `"lamad"`.
    pub pillar: String,
    /// EPR kind that was projected, e.g. `"EconomicEvent"`.
    pub kind: String,
    /// Content-address of the source EPR atom (CIDv1 base32 string).
    pub epr_cid: String,
    /// Target SQLite table, e.g. `"economic_events"`.
    pub target_table: String,
    /// Primary key value of the projected row. For CID-keyed tables this equals
    /// `epr_cid`; for other tables it may differ once non-CID projections land.
    pub row_key: String,
    /// ISO 8601 UTC timestamp of the projection write (not the atom's issued_at).
    pub timestamp: String,
}
