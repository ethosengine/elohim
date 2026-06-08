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
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
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
}

/// Input body for POST /api/v1/pins.
///
/// `kind` defaults to `"item"` when omitted.
/// `priority` defaults to `1` when omitted.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreatePinInputView {
    pub head_ref: String,
    pub kind: Option<String>,
    pub closure_rule: Option<JsonVal>,
    pub priority: Option<i32>,
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
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprPullStatusView {
    pub epr_id: String,
    pub total: Option<u64>,
    pub fetched: u64,
    pub pending: u64,
    pub failed: u64,
    pub caught_up: Option<bool>,
}
