use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PeerCapacityView {
    pub peer_cid: String,
    pub computed_at: String,
    pub total_raw_bytes: u64,
    pub pledges: PledgesView,
    pub actually_held: ActuallyHeldView,
    pub ratio_compliance: RatioComplianceView,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PledgesView {
    pub dwelling_bytes: u64,
    pub collective_bytes: u64,
    pub commons_bytes: u64,
    pub total_pledged_bytes: u64,
    pub pledges_by_recipient: Vec<PledgeByRecipientView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PledgeByRecipientView {
    pub tier: Tier,
    pub recipient_hub_id: String,
    pub commitment_cid: String,
    pub capacity_bytes: u64,
    pub provider_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ActuallyHeldView {
    pub unique_shard_bytes: u64,
    pub free_bytes_remaining: i64,
    pub fragmentation_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RatioComplianceView {
    pub effective_ratios: EffectiveRatiosView,
    pub current_ratios: CurrentRatiosView,
    pub compliant_with_donut: bool,
    pub violations: Vec<RatioViolationView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EffectiveRatiosView {
    pub commons_pct: i32,
    pub dwelling_pct: i32,
    pub collective_pct: i32,
    pub free_pct: i32,
    pub manifest_cid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CurrentRatiosView {
    pub commons_pct: i32,
    pub dwelling_pct: i32,
    pub collective_pct: i32,
    pub free_pct: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RatioViolationView {
    pub tier: Tier,
    pub violation_kind: ViolationKind,
    pub current_pct: i32,
    pub bound_pct: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum Tier {
    Dwelling,
    Collective,
    Commons,
    Free,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ViolationKind {
    BelowFloor,
    AboveCeiling,
    BelowManifestTarget,
    AboveManifestTarget,
}
