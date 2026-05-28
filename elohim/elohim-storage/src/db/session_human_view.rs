//! SessionHumanView projection — M-AGGR-1.
//!
//! Source of truth: derived projection of EconomicEvent stream × HumanProgress
//! for a specific agent (provider), filtered by qualifying lamadEventType values.
//! Category C operational — reconstructable at any time from the underlying
//! EconomicEvent entries in the elohim DNA content_store zome.
//!
//! No dht_anchor_hash: this is a COUNT aggregate over the economic_events
//! projection table (which carries per-event dht_anchor_hash values). Provenance
//! can be verified by querying economic_events directly for the agent's events.
//!
//! The projection is (re)computed on one trigger:
//! 1. `Signal::EconomicEventCreated` — when lamadEventType is one of the
//!    qualifying values listed below, the row for that agent is upserted.
//!
//! Projection writer: `project_session_human_view`
//! Fetch: `fetch_session_human_view`

use diesel::dsl::count_star;
use diesel::prelude::*;

use crate::db::diesel_schema::economic_events;
use crate::db::diesel_schema::session_human_view;
use crate::error::StorageError;
use elohim_views::SessionHumanView;

// ---------------------------------------------------------------------------
// lamadEventType constants relevant to this projection
// ---------------------------------------------------------------------------

const EVENT_CONTENT_VIEW: &str = "content-view";
const EVENT_AFFINITY: &str = "contributor-presence";
const EVENT_PATH_STARTED: &str = "path-started";
const EVENT_PATH_COMPLETED: &str = "path-completed";
const EVENT_STEP_COMPLETED: &str = "step-completed";

// ---------------------------------------------------------------------------
// Diesel row types
// ---------------------------------------------------------------------------

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = session_human_view)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SessionHumanViewRow {
    pub agent_id: String,
    pub h_app_id: String,
    pub nodes_viewed: i64,
    pub nodes_with_affinity: i64,
    pub paths_started: i64,
    pub paths_completed: i64,
    pub steps_completed: i64,
    pub journey_started_at: Option<String>,
    pub computed_at: String,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = session_human_view)]
pub struct UpsertSessionHumanView<'a> {
    pub agent_id: &'a str,
    pub h_app_id: &'a str,
    pub nodes_viewed: i64,
    pub nodes_with_affinity: i64,
    pub paths_started: i64,
    pub paths_completed: i64,
    pub steps_completed: i64,
    pub journey_started_at: Option<&'a str>,
    pub computed_at: &'a str,
}

// ---------------------------------------------------------------------------
// Core projection logic
// ---------------------------------------------------------------------------

/// Count events of a single lamadEventType for the given agent.
/// lamad_event_type is Nullable<Text> in the schema, so we compare against Some(event_type).
fn count_events(
    conn: &mut SqliteConnection,
    agent_id: &str,
    h_app_id: &str,
    event_type: &str,
) -> Result<i64, StorageError> {
    economic_events::table
        .filter(economic_events::provider.eq(agent_id))
        .filter(economic_events::h_app_id.eq(h_app_id))
        .filter(economic_events::lamad_event_type.eq(Some(event_type)))
        .select(count_star())
        .first::<i64>(conn)
        .map_err(|e| StorageError::Database(format!("session_human_view count({event_type}): {e}")))
}

/// Find the earliest has_point_in_time among all qualifying events for the agent.
fn earliest_event_at(
    conn: &mut SqliteConnection,
    agent_id: &str,
    h_app_id: &str,
) -> Result<Option<String>, StorageError> {
    // Use a raw SQL WHERE clause to filter nullable lamad_event_type across multiple values.
    use diesel::dsl::sql;
    use diesel::sql_types::Bool;

    economic_events::table
        .filter(economic_events::provider.eq(agent_id))
        .filter(economic_events::h_app_id.eq(h_app_id))
        .filter(sql::<Bool>(
            "lamad_event_type IN ('content-view','contributor-presence','path-started','path-completed','step-completed')"
        ))
        .order(economic_events::has_point_in_time.asc())
        .select(economic_events::has_point_in_time)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("session_human_view earliest_event_at: {e}")))
}

/// Recompute and upsert the SessionHumanView row for a single agent.
///
/// Reads the economic_events projection table, counts qualifying event types,
/// and derives journey_started_at from the earliest event timestamp.
///
/// This is the canonical write path — called by the signal handler when
/// Signal::EconomicEventCreated arrives with a qualifying lamadEventType.
pub fn project_session_human_view(
    conn: &mut SqliteConnection,
    agent_id: &str,
    h_app_id: &str,
) -> Result<SessionHumanView, StorageError> {
    let now = chrono::Utc::now().to_rfc3339();

    let nodes_viewed = count_events(conn, agent_id, h_app_id, EVENT_CONTENT_VIEW)?;
    let nodes_with_affinity = count_events(conn, agent_id, h_app_id, EVENT_AFFINITY)?;
    let paths_started = count_events(conn, agent_id, h_app_id, EVENT_PATH_STARTED)?;
    let paths_completed = count_events(conn, agent_id, h_app_id, EVENT_PATH_COMPLETED)?;
    let steps_completed = count_events(conn, agent_id, h_app_id, EVENT_STEP_COMPLETED)?;
    let journey_started_at = earliest_event_at(conn, agent_id, h_app_id)?;

    let row = UpsertSessionHumanView {
        agent_id,
        h_app_id,
        nodes_viewed,
        nodes_with_affinity,
        paths_started,
        paths_completed,
        steps_completed,
        journey_started_at: journey_started_at.as_deref(),
        computed_at: &now,
    };

    diesel::insert_into(session_human_view::table)
        .values(&row)
        .on_conflict((session_human_view::agent_id, session_human_view::h_app_id))
        .do_update()
        .set(&row)
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("session_human_view upsert: {e}")))?;

    Ok(SessionHumanView {
        agent_id: agent_id.to_string(),
        h_app_id: h_app_id.to_string(),
        nodes_viewed,
        nodes_with_affinity,
        paths_started,
        paths_completed,
        steps_completed,
        journey_started_at,
        computed_at: now,
    })
}

/// Fetch an already-projected SessionHumanView row by agent_id.
///
/// Returns `None` if no EconomicEvents have been projected for this agent yet.
pub fn fetch_session_human_view(
    conn: &mut SqliteConnection,
    agent_id: &str,
    h_app_id: &str,
) -> Result<Option<SessionHumanView>, StorageError> {
    let row = session_human_view::table
        .filter(session_human_view::agent_id.eq(agent_id))
        .filter(session_human_view::h_app_id.eq(h_app_id))
        .select(SessionHumanViewRow::as_select())
        .first::<SessionHumanViewRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("fetch_session_human_view: {e}")))?;

    Ok(row.map(|r| SessionHumanView {
        agent_id: r.agent_id,
        h_app_id: r.h_app_id,
        nodes_viewed: r.nodes_viewed,
        nodes_with_affinity: r.nodes_with_affinity,
        paths_started: r.paths_started,
        paths_completed: r.paths_completed,
        steps_completed: r.steps_completed,
        journey_started_at: r.journey_started_at,
        computed_at: r.computed_at,
    }))
}
