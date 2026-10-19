//! DevicePin wire shapes (spec §1.1, §4.4). camelCase out, parsed JSON —
//! snake_case never leaves the Rust boundary.
use crate::shared::JsonVal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Wire view for a single DevicePin row (GET /api/v1/pins, own-node only).
///
/// Category B agent-scoped local store — airplane-mode property holds:
/// every field is sourced from the local `acquisition_pins` SQLite table.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct PinView {
    pub id: i32,
    pub agent_pub_key: String,
    pub head_ref: String,
    pub kind: String,
    /// Parsed closure rule (null when not set).
    pub closure_rule: Option<JsonVal>,
    pub priority: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    // The fields below were added after the first release of this view. A
    // reader of an older node gets none of them, so they are optional on the
    // wire (CONVENTIONS: additive fields never become required).
    /// Retirements in a row with no fetch and no peer inventory naming the head
    /// between them. Each one doubles the hold before the next re-probe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub consecutive_retirements: Option<i32>,
    /// When the pin was last retired (RFC 3339) — the clock its hold runs
    /// from. Absent if it never has been.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub retired_at: Option<String>,
    /// Earliest time a `retired` pin is re-admitted for a re-probe (RFC 3339,
    /// UTC): `retiredAt` plus the hold for its rung (6 h, 12 h, then 24 h), or
    /// `retiredAt` + 6 h if `readSinceRetired`. Every pin past it is
    /// re-admitted by the next reconcile tick, so it is a maximum wait up to
    /// one tick. Absent unless `status` is `retired`. A peer inventory naming
    /// the head, or a read the demand auto-pin admits, re-activates it sooner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub next_readmit_at: Option<String>,
    /// True when a read-miss the demand auto-pin did not admit (throttled,
    /// window full, ceiling) named the pin while it was retired: its hold is
    /// capped at 6 h. Cleared on re-activation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub read_since_retired: Option<bool>,
    /// `exhausted` (the retry budget was spent and no peer asked supplied the
    /// bytes), `fetched`, or absent when no round has concluded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_outcome: Option<String>,
}

/// Input body for POST /api/v1/pins.
///
/// `kind` defaults to `"item"` when omitted.
/// `priority` defaults to `1` when omitted.
/// `provide` makes the pin a serveable peer provider (spec slice 2b); it is
/// honored only on a peer-capable node — a `browser` context provide is refused.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct CreatePinInputView {
    pub head_ref: String,
    pub kind: Option<String>,
    pub closure_rule: Option<JsonVal>,
    pub priority: Option<i32>,
    pub provide: Option<bool>,
    pub context: Option<String>,
}

/// Per-EPR pull progress (spec §4.3 / Slice 2b T13), served on
/// GET /api/v1/pins/{eprId}/pull (own node only).
///
/// Groups all of a person's pins for one `head_ref` and counts each shared
/// content id exactly once. `total`/`caughtUp` are `Option`: `None` on the
/// wire means "cannot compute" = keep waiting (the wait-for-drain tri-state,
/// spec §4.3) — never caught up.
///
/// Category C operational — recomputed per request from the in-memory
/// AcquisitionState; never persisted.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../elohim/sdk/storage-client-ts/src/generated/")]
pub struct EprPullStatusView {
    pub epr_id: String,
    pub total: Option<u64>,
    pub fetched: u64,
    pub pending: u64,
    pub failed: u64,
    pub caught_up: Option<bool>,
}
