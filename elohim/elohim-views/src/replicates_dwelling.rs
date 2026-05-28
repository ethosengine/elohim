use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReplicatesDwellingPayload {
    pub action: String,
    pub provider_dwelling_hub_id: String,
    pub recipient_dwelling_hub_id: String,
    pub provider_role: ProviderRole,
    pub via_collective_hub_id: Option<String>,
    pub capacity_bytes: u64,
    pub scope_filter: ScopeFilter,
    pub valid_from: String,
    pub valid_until: String,
    pub grace_period_days: u32,
    pub rotation_ttl_days: u32,
    pub ratio_attestation: RatioAttestation,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ProviderRole {
    StewardMutual,
    CollectiveSteward,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ScopeFilter {
    pub epr_kinds: Option<Vec<String>>,
    pub bytes_per_blob_max: Option<u64>,
    pub requires_attestations: Option<Vec<String>>,
    pub kinds_excluded: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RatioAttestation {
    pub commons_pct: u8,
    pub dwelling_pct: u8,
    pub collective_pct: u8,
    pub free_pct: u8,
    pub effective_ratio_cid: String,
}
