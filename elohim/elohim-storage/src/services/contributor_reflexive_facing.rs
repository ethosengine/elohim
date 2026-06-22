//! Contributor-reflexive facing — the impure adapter that assembles a
//! [`ContributorReflexiveView`] ("how the network sees a contributor") from a presence's
//! `economic_events` (network-routed recognition), its active `stewardship_allocations`
//! (steward role), and its accumulated scalars (engagement). A thin adapter over the pure
//! folds in `elohim_facings::folds::contributor_reflexive` — it loads, maps diesel rows to
//! the DB-free mirror rows, calls the folds, and assembles the view. NEVER folds in here.
//!
//! Child of the §11 select→fold→aggregate framework
//! (`genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md`).
//! Spec: `genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md` (Wave 2).

use diesel::SqliteConnection;
use elohim_facings::folds::contributor_reflexive::{
    commons_flow_value, distinct_content_recognized, recognition_value_by_action,
    steward_allocation_summary, total_recognition_value, AllocationRow, RecognitionEventRow,
};
use elohim_views::{ContributorReflexiveView, RecognitionByAction};

use crate::db::{contributor_presences, economic_events, stewardship_allocations, AppContext};

/// Build the contributor-reflexive view for `presence_id`. Returns `None` when no presence
/// exists (the route surfaces a 404). Query failures on the event/allocation relations degrade
/// to empty (a presence with no routed recognition reads as zeros, never an error).
pub fn build_contributor_reflexive_view(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    presence_id: &str,
) -> Option<ContributorReflexiveView> {
    // Engagement-accrued scalars — assembled straight off the presence row (NOT re-folded).
    let presence = contributor_presences::get_contributor_presence(conn, ctx, presence_id)
        .ok()
        .flatten()?;

    // Network-routed recognition events — the indexed `contributor_presence_id` join.
    let events: Vec<RecognitionEventRow> =
        economic_events::get_events_for_presence(conn, ctx, presence_id)
            .unwrap_or_default()
            .into_iter()
            .map(|e| RecognitionEventRow {
                action: e.action,
                receiver: e.receiver,
                resource_quantity_value: e.resource_quantity_value.map(|v| v as f64),
                content_id: e.content_id,
            })
            .collect();

    // Steward role — active allocations only (the query filters governance_state).
    let allocations: Vec<AllocationRow> =
        stewardship_allocations::get_allocations_for_steward(conn, ctx, presence_id)
            .unwrap_or_default()
            .into_iter()
            .map(|a| AllocationRow {
                content_id: a.content_id,
                allocation_ratio: a.allocation_ratio as f64,
                governance_state: a.governance_state,
                recognition_accumulated: a.recognition_accumulated as f64,
            })
            .collect();

    let (steward_allocation_count, steward_recognition_accumulated) =
        steward_allocation_summary(&allocations);

    let recognition_by_action = recognition_value_by_action(&events)
        .into_iter()
        .map(|(action, value)| RecognitionByAction { action, value })
        .collect();

    Some(ContributorReflexiveView {
        presence_id: presence.id,
        display_name: presence.display_name,
        presence_state: presence.presence_state,
        recognition_score: presence.recognition_score as f64,
        citation_count: presence.citation_count,
        affinity_total: presence.affinity_total as f64,
        unique_engagers: presence.unique_engagers,
        total_recognition_value: total_recognition_value(&events),
        recognition_by_action,
        distinct_content_recognized: distinct_content_recognized(&events),
        commons_flow_value: commons_flow_value(&events),
        steward_allocation_count,
        steward_recognition_accumulated,
    })
}
