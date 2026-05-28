use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Re-export the canonical HubKind (defined in `infrastructure`) so downstream
// consumers can import it as `elohim_views::hub_capacity::HubKind`. We reuse the
// existing enum rather than redeclaring — a second definition would clobber the
// generated `HubKind.ts` and strip its doc annotations. Variants + snake_case
// serde already match the hub-capacity-view schema enum.
pub use crate::infrastructure::HubKind;
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
