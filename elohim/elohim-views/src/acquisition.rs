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
