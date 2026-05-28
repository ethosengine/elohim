use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::infrastructure::HubKind;
use crate::peer_capacity::{ActuallyHeldView, PledgesView};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HubCapacityView {
    pub hub_id: String,
    pub hub_kind: HubKind,
    pub display_label: Option<String>,
    pub member_device_count: i32,
    pub capacity: Option<HubCapacityAggregate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HubCapacityAggregate {
    pub total_raw_bytes: u64,
    pub pledges: PledgesView,
    pub actually_held: ActuallyHeldView,
}
