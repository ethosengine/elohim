//! Portable app lifecycle contracts. Source of truth:
//! `elohim/sdk/schemas/v1/views/app-release.schema.json and app-*.schema.json`.
//! These are declarations, not claims that a runtime adapter is implemented.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// An immutable release selected by the existing channel authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppReleaseView {
    pub app_id: String,
    pub release_cid: String,
    /// Platform/ABI target declared by the artifact producer, not renderer.platform.
    pub target: String,
    /// Complete executable dependency set; possession does not grant execution.
    pub artifact_cids: Vec<String>,
    pub required_capabilities: Vec<String>,
}

/// Idempotent request. grantRef is a reference to authority the runtime MUST verify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppOpenInput {
    pub release: AppReleaseView,
    pub operation_id: String,
    pub grant_ref: String,
}

/// Operational handle, not a notarized app entity or a public presence announcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppInstanceView {
    pub instance_id: String,
    pub device_id: String,
    pub release: AppReleaseView,
}

/// Independent custody facts. Unknown verification is never true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppResidencyView {
    pub release_cid: String,
    pub pinned: bool,
    pub running: bool,
    pub artifacts_verified: Option<bool>,
    pub data_verified: Option<bool>,
}

/// Reference to an explicit private checkpoint in the existing working-version store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppContinuationView {
    pub source_instance_id: String,
    pub release_cid: String,
    pub checkpoint_cid: String,
    pub format_version: u32,
}

/// Browser exposure request; authority and origin are checked independently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppAttachmentInput {
    pub instance_id: String,
    pub grant_ref: String,
    pub origin: String,
}

/// A private expiring route. The URL contains no bearer credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppRouteView {
    pub url: String,
    pub expires_at: String,
}

/// A fixed per-device target in the existing stream's epoch/position space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppSyncTargetView {
    pub device_id: String,
    pub stream_id: String,
    pub epoch: String,
    /// Decimal u64 string, preserving positions beyond JavaScript's exact integers.
    /// Null means the remote end is unknown, not zero.
    pub end_position: Option<String>,
}

/// Source of truth: ../enums/app-sync-urgency.schema.json. Priority does not confer authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub enum AppSyncUrgency {
    Background,
    Foreground,
}

/// Transient request over an already-authorized scope, not a second sync protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppSyncDemandView {
    pub scope: String,
    pub urgency: AppSyncUrgency,
    pub targets: Vec<AppSyncTargetView>,
}

/// Monomorphic wire specialization of the existing Answer envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    tag = "state",
    rename_all = "camelCase",
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub enum AppReleaseAnswer {
    Present { value: AppReleaseView },
    Absent { reason: AppAbsentReason },
    Unreachable { reason: AppUnreachableReason },
}

/// Runtime lookup preserves observed absence versus unreachable, as the native Answer does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    tag = "state",
    rename_all = "camelCase",
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub enum AppInstanceAnswer {
    Present { value: AppInstanceView },
    Absent { reason: AppAbsentReason },
    Unreachable { reason: AppUnreachableReason },
}

/// Absent subset of the existing answer-reason schema; not a new vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub enum AppAbsentReason {
    ObservedAbsent,
}

/// Unreachable subset of the existing answer-reason schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub enum AppUnreachableReason {
    Timeout,
    TransportError,
    Refused,
    Unverifiable,
    NotYetDelivered,
}

/// Per-device progress uses existing sync-state semantics: null is unknown,
/// false is behind, true is caught up. Reachability is a separate observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(
    export,
    export_to = "../../../elohim/sdk/storage-client-ts/src/generated/"
)]
pub struct AppSyncProgressView {
    pub target: AppSyncTargetView,
    pub reachable: Option<bool>,
    pub local_epoch: Option<String>,
    pub local_position: Option<String>,
    pub caught_up: Option<bool>,
}
