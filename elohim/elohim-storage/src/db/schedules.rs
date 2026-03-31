//! Schedule CRUD operations (Kairos temporal dimension)
//!
//! Polymorphic temporal attachment for any CID-addressed entity.
//! Supports scheduling (future activation), expiry (duration),
//! and recurrence (RFC 5545 RRULE).

use chrono::TimeZone;
use diesel::prelude::*;
use tracing::debug;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::schedules;
use super::models::{current_timestamp, NewSchedule, Schedule};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a schedule
pub struct CreateScheduleInput {
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
}

/// Input for updating a schedule (PATCH semantics).
/// `Option<Option<T>>`: None = no change, Some(None) = clear, Some(Some(v)) = set.
#[derive(Debug, Default)]
pub struct UpdateScheduleInput {
    pub scheduled_at: Option<Option<String>>,
    pub expires_at: Option<Option<String>>,
    pub rrule: Option<Option<String>>,
}

/// Query params for listing schedules
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub due_before: Option<String>,
    pub limit: Option<i64>,
}

// ============================================================================
// RRULE Computation
// ============================================================================

/// Compute next occurrence from an RRULE string after a given datetime.
///
/// The `rrule_str` should be a bare RRULE property value (e.g. "FREQ=WEEKLY;BYDAY=MO").
/// We prepend a DTSTART line so the rrule crate can parse it as a complete RRuleSet.
///
/// Returns `None` if the RRULE has no more occurrences after the given time.
fn compute_next_occurrence(
    rrule_str: &str,
    after: Option<&str>,
) -> Result<Option<String>, StorageError> {
    use rrule::RRuleSet;

    // Build an RRULE string with DTSTART.
    // Use a far-past start so we don't miss any occurrences.
    let full_rrule = format!("DTSTART:20000101T000000Z\nRRULE:{}", rrule_str);

    let rrule_set: RRuleSet = full_rrule
        .parse()
        .map_err(|e| StorageError::Internal(format!("Invalid RRULE '{}': {}", rrule_str, e)))?;

    // Determine the "after" threshold
    let after_str = match after {
        Some(s) => s.to_string(),
        None => current_timestamp(),
    };

    // Parse the after datetime. Support both RFC 3339 and the simpler ISO format
    // used by current_timestamp().
    let after_dt = parse_datetime_to_tz(&after_str)?;

    // Get occurrences after the given datetime
    let result = rrule_set.after(after_dt).all(100);

    Ok(result
        .dates
        .first()
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
}

/// Parse a datetime string into `DateTime<rrule::Tz>` for RRULE computation.
fn parse_datetime_to_tz(s: &str) -> Result<chrono::DateTime<rrule::Tz>, StorageError> {
    use rrule::Tz;

    // Try RFC 3339 first (e.g. "2026-03-16T12:00:00Z" or "2026-03-16T12:00:00+00:00")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Tz::UTC));
    }

    // Try the format used by current_timestamp(): "2026-03-16T12:00:00Z"
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ") {
        let utc_dt = Tz::UTC.from_utc_datetime(&ndt);
        return Ok(utc_dt);
    }

    // Try bare ISO without timezone
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        let utc_dt = Tz::UTC.from_utc_datetime(&ndt);
        return Ok(utc_dt);
    }

    Err(StorageError::Internal(format!(
        "Invalid datetime for RRULE computation: {}",
        s
    )))
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new schedule
pub fn create_schedule(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateScheduleInput,
) -> Result<Schedule, StorageError> {
    let now = current_timestamp();
    let id = Uuid::new_v4().to_string();

    // Compute next_occurrence_at from RRULE if provided
    let next_occurrence = if let Some(ref rrule_str) = input.rrule {
        compute_next_occurrence(rrule_str, None)?
    } else {
        // No recurrence — next occurrence is the scheduled_at time (if any)
        input.scheduled_at.clone()
    };

    let new = NewSchedule {
        id: id.clone(),
        h_app_id: ctx.h_app_id().to_string(),
        entity_type: input.entity_type,
        entity_id: input.entity_id,
        scheduled_at: input.scheduled_at,
        expires_at: input.expires_at,
        rrule: input.rrule,
        last_occurred_at: None,
        next_occurrence_at: next_occurrence,
        occurrence_count: 0,
        created_at: now.clone(),
        updated_at: now,
    };

    debug!(
        id = %new.id,
        entity_type = %new.entity_type,
        entity_id = %new.entity_id,
        "Creating schedule"
    );

    diesel::insert_into(schedules::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create schedule: {}", e)))?;

    schedules::table
        .filter(schedules::id.eq(&id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to fetch created schedule: {}", e)))
}

/// Get schedule by entity type and entity ID
pub fn get_schedule(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    entity_type: &str,
    entity_id: &str,
) -> Result<Schedule, StorageError> {
    schedules::table
        .filter(schedules::h_app_id.eq(ctx.h_app_id()))
        .filter(schedules::entity_type.eq(entity_type))
        .filter(schedules::entity_id.eq(entity_id))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StorageError::NotFound(format!(
                "Schedule not found for {}:{}",
                entity_type, entity_id
            )),
            _ => StorageError::Internal(format!("Failed to fetch schedule: {}", e)),
        })
}

/// Get schedule by ID
pub fn get_schedule_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Schedule, StorageError> {
    schedules::table
        .filter(schedules::id.eq(id))
        .filter(schedules::h_app_id.eq(ctx.h_app_id()))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("Schedule not found: {}", id))
            }
            _ => StorageError::Internal(format!("Failed to fetch schedule: {}", e)),
        })
}

/// List schedules with optional filters
pub fn list_schedules(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ScheduleQuery,
) -> Result<Vec<Schedule>, StorageError> {
    let mut q = schedules::table
        .filter(schedules::h_app_id.eq(ctx.h_app_id()))
        .into_boxed();

    if let Some(ref entity_type) = query.entity_type {
        q = q.filter(schedules::entity_type.eq(entity_type));
    }
    if let Some(ref entity_id) = query.entity_id {
        q = q.filter(schedules::entity_id.eq(entity_id));
    }
    if let Some(ref before) = query.due_before {
        q = q.filter(schedules::next_occurrence_at.le(before));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    q.order(schedules::next_occurrence_at.asc())
        .load::<Schedule>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list schedules: {}", e)))
}

/// Update a schedule (PATCH semantics)
pub fn update_schedule(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    input: UpdateScheduleInput,
) -> Result<Schedule, StorageError> {
    let existing = get_schedule_by_id(conn, ctx, id)?;
    let now = current_timestamp();

    // Apply PATCH: None = keep existing, Some(v) = use new value
    let new_scheduled_at = input.scheduled_at.unwrap_or(existing.scheduled_at.clone());
    let new_expires_at = input.expires_at.unwrap_or(existing.expires_at.clone());
    let new_rrule = input.rrule.unwrap_or(existing.rrule.clone());

    // Recompute next_occurrence if rrule changed
    let next_occurrence = if new_rrule != existing.rrule {
        if let Some(ref rrule_str) = new_rrule {
            compute_next_occurrence(rrule_str, existing.last_occurred_at.as_deref())?
        } else {
            new_scheduled_at.clone()
        }
    } else {
        existing.next_occurrence_at
    };

    diesel::update(schedules::table.filter(schedules::id.eq(id)))
        .set((
            schedules::scheduled_at.eq(&new_scheduled_at),
            schedules::expires_at.eq(&new_expires_at),
            schedules::rrule.eq(&new_rrule),
            schedules::next_occurrence_at.eq(&next_occurrence),
            schedules::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update schedule: {}", e)))?;

    get_schedule_by_id(conn, ctx, id)
}

/// Advance a recurring schedule — bump last_occurred_at, increment count, recompute next
pub fn advance_occurrence(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Schedule, StorageError> {
    let existing = get_schedule_by_id(conn, ctx, id)?;
    let now = current_timestamp();

    let next_occurrence = if let Some(ref rrule_str) = existing.rrule {
        compute_next_occurrence(rrule_str, Some(&now))?
    } else {
        None // One-time schedule — no more occurrences after advance
    };

    diesel::update(schedules::table.filter(schedules::id.eq(id)))
        .set((
            schedules::last_occurred_at.eq(&now),
            schedules::next_occurrence_at.eq(&next_occurrence),
            schedules::occurrence_count.eq(existing.occurrence_count + 1),
            schedules::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to advance schedule: {}", e)))?;

    get_schedule_by_id(conn, ctx, id)
}
