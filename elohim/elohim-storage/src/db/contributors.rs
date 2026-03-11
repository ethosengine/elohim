//! Contributor dashboard and impact CRUD operations

use diesel::prelude::*;
use serde::Serialize;

use super::diesel_schema::{contributor_dashboards, economic_events};
use super::models::{ContributorDashboard, EconomicEvent};
use crate::error::StorageError;

/// Get dashboard for a presence
pub fn get_dashboard(
    conn: &mut SqliteConnection,
    presence_id: &str,
) -> Result<Option<ContributorDashboard>, StorageError> {
    contributor_dashboards::table
        .filter(contributor_dashboards::presence_id.eq(presence_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Impact summary for a contributor
#[derive(Debug, Clone, Serialize)]
pub struct ImpactSummary {
    pub presence_id: String,
    pub total_events: i64,
    pub unique_content_ids: Vec<String>,
}

/// Get impact summary from economic events for a presence
pub fn get_impact(
    conn: &mut SqliteConnection,
    presence_id: &str,
) -> Result<ImpactSummary, StorageError> {
    let total_events: i64 = economic_events::table
        .filter(economic_events::contributor_presence_id.eq(presence_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let unique_content_ids: Vec<String> = economic_events::table
        .filter(economic_events::contributor_presence_id.eq(presence_id))
        .filter(economic_events::content_id.is_not_null())
        .select(economic_events::content_id)
        .distinct()
        .load::<Option<String>>(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .into_iter()
        .flatten()
        .collect();

    Ok(ImpactSummary {
        presence_id: presence_id.to_string(),
        total_events,
        unique_content_ids,
    })
}

/// Get recognition history from economic events for a presence
pub fn get_recognition_history(
    conn: &mut SqliteConnection,
    presence_id: &str,
) -> Result<Vec<EconomicEvent>, StorageError> {
    economic_events::table
        .filter(economic_events::contributor_presence_id.eq(presence_id))
        .order(economic_events::has_point_in_time.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
