//! ContentEngagementStats projection — M-AGGR-3.
//!
//! Source of truth: derived projection of EconomicEvent stream filtered by
//! content_id AND lamadEventType IN ('content-view', 'content-complete').
//! Category C operational — reconstructable at any time from the underlying
//! EconomicEvent entries in the elohim DNA content_store zome.
//!
//! No dht_anchor_hash: this is a GROUP BY aggregate over the economic_events
//! projection table (which carries per-event dht_anchor_hash values). Provenance
//! can be verified by querying economic_events directly per content_id and
//! lamad_event_type.
//!
//! The projection is (re)computed on one trigger:
//! 1. `Signal::EconomicEventCreated` — when lamadEventType is 'content-view'
//!    OR 'content-complete', the row for that content_id is upserted.
//!
//! Projection writer: `project_content_engagement_stats`
//! Fetch: `fetch_content_engagement_stats`

use diesel::dsl::{count_star, sql};
use diesel::prelude::*;

use crate::db::diesel_schema::content_engagement_stats;
use crate::db::diesel_schema::economic_events;
use crate::error::StorageError;
use elohim_views::ContentEngagementStatsView;

// ---------------------------------------------------------------------------
// Event type constants
// ---------------------------------------------------------------------------

const EVENT_CONTENT_VIEW: &str = "content-view";
const EVENT_CONTENT_COMPLETE: &str = "content-complete";

// ---------------------------------------------------------------------------
// Diesel row types
// ---------------------------------------------------------------------------

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = content_engagement_stats)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ContentEngagementStatsRow {
    pub content_id: String,
    pub h_app_id: String,
    pub views: i32,
    pub completions: i32,
    pub unique_viewers: i32,
    pub completion_rate: f64,
    pub computed_at: String,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = content_engagement_stats)]
pub struct UpsertContentEngagementStats<'a> {
    pub content_id: &'a str,
    pub h_app_id: &'a str,
    pub views: i32,
    pub completions: i32,
    pub unique_viewers: i32,
    pub completion_rate: f64,
    pub computed_at: &'a str,
}

// ---------------------------------------------------------------------------
// Core projection logic
// ---------------------------------------------------------------------------

/// Recompute and upsert the engagement stats row for a single content_id.
///
/// Reads the economic_events projection table, filters to the two relevant
/// lamadEventType values, and derives views / completions / unique_viewers /
/// completion_rate. Writes the upserted row back to content_engagement_stats.
///
/// This is the canonical write path — called by the signal handler when
/// Signal::EconomicEventCreated arrives with a qualifying lamadEventType.
pub fn project_content_engagement_stats(
    conn: &mut SqliteConnection,
    content_id: &str,
    h_app_id: &str,
) -> Result<ContentEngagementStatsView, StorageError> {
    let now = chrono::Utc::now().to_rfc3339();

    // 1. Count content-view events for this content_id.
    let views: i64 = economic_events::table
        .filter(economic_events::content_id.eq(content_id))
        .filter(economic_events::h_app_id.eq(h_app_id))
        .filter(economic_events::lamad_event_type.eq(EVENT_CONTENT_VIEW))
        .select(count_star())
        .first::<i64>(conn)
        .map_err(|e| {
            StorageError::Database(format!("content_engagement_stats views count: {e}"))
        })?;

    // 2. Count content-complete events for this content_id.
    let completions: i64 = economic_events::table
        .filter(economic_events::content_id.eq(content_id))
        .filter(economic_events::h_app_id.eq(h_app_id))
        .filter(economic_events::lamad_event_type.eq(EVENT_CONTENT_COMPLETE))
        .select(count_star())
        .first::<i64>(conn)
        .map_err(|e| {
            StorageError::Database(format!("content_engagement_stats completions count: {e}"))
        })?;

    // 3. Count distinct providers (agents) among content-view events — unique viewers.
    //    Diesel's SQLite backend does not expose COUNT(DISTINCT col) as a typed DSL
    //    method, so we use a raw sql() expression on the select clause only.
    let unique_viewers: i64 = economic_events::table
        .filter(economic_events::content_id.eq(content_id))
        .filter(economic_events::h_app_id.eq(h_app_id))
        .filter(economic_events::lamad_event_type.eq(EVENT_CONTENT_VIEW))
        .select(sql::<diesel::sql_types::BigInt>("COUNT(DISTINCT provider)"))
        .first::<i64>(conn)
        .map_err(|e| {
            StorageError::Database(format!(
                "content_engagement_stats unique_viewers count: {e}"
            ))
        })?;

    // 4. Derive completion_rate.
    let completion_rate = if views > 0 {
        completions as f64 / views as f64
    } else {
        0.0
    };

    // 5. Upsert the projection row.
    let row = UpsertContentEngagementStats {
        content_id,
        h_app_id,
        views: views as i32,
        completions: completions as i32,
        unique_viewers: unique_viewers as i32,
        completion_rate,
        computed_at: &now,
    };

    diesel::insert_into(content_engagement_stats::table)
        .values(&row)
        .on_conflict((
            content_engagement_stats::content_id,
            content_engagement_stats::h_app_id,
        ))
        .do_update()
        .set(&row)
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("content_engagement_stats upsert: {e}")))?;

    Ok(ContentEngagementStatsView {
        content_id: content_id.to_string(),
        views,
        completions,
        unique_viewers,
        completion_rate,
        computed_at: now,
    })
}

/// Fetch an already-projected engagement stats row by content_id.
///
/// Returns `None` if no EconomicEvents have been projected for this content yet.
/// The caller should trigger `project_content_engagement_stats` if projection is
/// needed before reading.
pub fn fetch_content_engagement_stats(
    conn: &mut SqliteConnection,
    content_id: &str,
    h_app_id: &str,
) -> Result<Option<ContentEngagementStatsView>, StorageError> {
    let row = content_engagement_stats::table
        .filter(content_engagement_stats::content_id.eq(content_id))
        .filter(content_engagement_stats::h_app_id.eq(h_app_id))
        .first::<ContentEngagementStatsRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("content_engagement_stats fetch: {e}")))?;

    Ok(row.map(ContentEngagementStatsView::from))
}

// ---------------------------------------------------------------------------
// From impl (DB row → View)
// ---------------------------------------------------------------------------

impl From<ContentEngagementStatsRow> for ContentEngagementStatsView {
    fn from(r: ContentEngagementStatsRow) -> Self {
        let completion_rate = if r.views > 0 {
            r.completions as f64 / r.views as f64
        } else {
            0.0
        };
        ContentEngagementStatsView {
            content_id: r.content_id,
            views: r.views as i64,
            completions: r.completions as i64,
            unique_viewers: r.unique_viewers as i64,
            completion_rate,
            computed_at: r.computed_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_rate_zero_when_no_views() {
        let row = ContentEngagementStatsRow {
            content_id: "c1".into(),
            h_app_id: "app".into(),
            views: 0,
            completions: 0,
            unique_viewers: 0,
            completion_rate: 0.0,
            computed_at: "2026-05-28T00:00:00Z".into(),
        };
        let view = ContentEngagementStatsView::from(row);
        assert_eq!(view.completion_rate, 0.0);
    }

    #[test]
    fn completion_rate_derived_from_row_counts() {
        let row = ContentEngagementStatsRow {
            content_id: "c2".into(),
            h_app_id: "app".into(),
            views: 10,
            completions: 4,
            unique_viewers: 7,
            completion_rate: 0.4,
            computed_at: "2026-05-28T00:00:00Z".into(),
        };
        let view = ContentEngagementStatsView::from(row);
        // From impl recomputes; stored completion_rate in row is ignored in favour
        // of recomputed value to avoid drift.
        assert!((view.completion_rate - 0.4).abs() < 1e-9);
        assert_eq!(view.views, 10);
        assert_eq!(view.completions, 4);
        assert_eq!(view.unique_viewers, 7);
    }
}
