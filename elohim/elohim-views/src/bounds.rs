//! bounds view types — BoundsValidationResultView for POST /api/v1/diagnostics/validate-bounds.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BoundsValidationResultView {
    pub pass: bool,
    pub commitment_cid: String,
    pub violation: Option<BoundsViolationView>,
    pub checks: BoundsChecksView,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BoundsViolationView {
    pub kind: ViolationKind,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ViolationKind {
    CommitmentInactive,
    ScopeNotIncluded,
    ReachCeilingExceeded,
    RateLimitExceeded,
    KeyRotationStale,
    CommitmentRevoked,
    CommitmentNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BoundsChecksView {
    pub commitment_found: bool,
    pub active: bool,
    pub scope_includes_event: bool,
    pub reach_ceiling_ok: bool,
    pub rate_within_limit: bool,
    pub key_rotation_current: bool,
    pub not_revoked: bool,
}
