//! Device policies CRUD operations
//!
//! Per-subject/per-device policy rules set by stewards.
//! Policies form inheritance chains via `inherits_from`.

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::diesel_schema::device_policies;
use super::models::{DevicePolicy, NewDevicePolicy};
use super::PooledConn;
use crate::error::StorageError;

/// Input for creating/upserting a device policy
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDevicePolicyInput {
    pub subject_id: String,
    pub device_id: Option<String>,
    pub author_id: String,
    pub author_tier: String,
    pub inherits_from: Option<String>,
    pub blocked_categories_json: String,
    pub blocked_hashes_json: String,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    pub time_windows_json: String,
    pub cooldown_minutes: Option<i32>,
    pub disabled_features_json: String,
    pub disabled_routes_json: String,
    pub require_approval_json: String,
    pub log_sessions: bool,
    pub log_categories: bool,
    pub log_policy_events: bool,
    pub retention_days: i32,
    pub subject_can_view: bool,
}

/// Upsert a device policy (insert or update by subject_id + author_id)
pub fn upsert_policy(
    conn: &mut PooledConn,
    input: &CreateDevicePolicyInput,
) -> Result<DevicePolicy, StorageError> {
    // Check if a policy already exists for this author + subject
    let existing = device_policies::table
        .filter(device_policies::subject_id.eq(&input.subject_id))
        .filter(device_policies::author_id.eq(&input.author_id))
        .first::<DevicePolicy>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let now = chrono::Utc::now().to_rfc3339();

    if let Some(existing) = existing {
        // Update existing policy, bump version
        diesel::update(device_policies::table.filter(device_policies::id.eq(&existing.id)))
            .set((
                device_policies::inherits_from.eq(&input.inherits_from),
                device_policies::blocked_categories_json.eq(&input.blocked_categories_json),
                device_policies::blocked_hashes_json.eq(&input.blocked_hashes_json),
                device_policies::age_rating_max.eq(&input.age_rating_max),
                device_policies::reach_level_max.eq(&input.reach_level_max),
                device_policies::session_max_minutes.eq(&input.session_max_minutes),
                device_policies::daily_max_minutes.eq(&input.daily_max_minutes),
                device_policies::time_windows_json.eq(&input.time_windows_json),
                device_policies::cooldown_minutes.eq(&input.cooldown_minutes),
                device_policies::disabled_features_json.eq(&input.disabled_features_json),
                device_policies::disabled_routes_json.eq(&input.disabled_routes_json),
                device_policies::require_approval_json.eq(&input.require_approval_json),
                device_policies::log_sessions.eq(input.log_sessions as i32),
                device_policies::log_categories.eq(input.log_categories as i32),
                device_policies::log_policy_events.eq(input.log_policy_events as i32),
                device_policies::retention_days.eq(input.retention_days),
                device_policies::subject_can_view.eq(input.subject_can_view as i32),
                device_policies::version.eq(existing.version + 1),
                device_policies::updated_at.eq(&now),
            ))
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

        device_policies::table
            .filter(device_policies::id.eq(&existing.id))
            .first::<DevicePolicy>(conn)
            .map_err(|e| StorageError::Internal(format!("Reload failed: {}", e)))
    } else {
        // Insert new policy
        let new_policy = NewDevicePolicy {
            id: Uuid::new_v4().to_string(),
            subject_id: input.subject_id.clone(),
            device_id: input.device_id.clone(),
            author_id: input.author_id.clone(),
            author_tier: input.author_tier.clone(),
            inherits_from: input.inherits_from.clone(),
            blocked_categories_json: input.blocked_categories_json.clone(),
            blocked_hashes_json: input.blocked_hashes_json.clone(),
            age_rating_max: input.age_rating_max.clone(),
            reach_level_max: input.reach_level_max,
            session_max_minutes: input.session_max_minutes,
            daily_max_minutes: input.daily_max_minutes,
            time_windows_json: input.time_windows_json.clone(),
            cooldown_minutes: input.cooldown_minutes,
            disabled_features_json: input.disabled_features_json.clone(),
            disabled_routes_json: input.disabled_routes_json.clone(),
            require_approval_json: input.require_approval_json.clone(),
            log_sessions: input.log_sessions as i32,
            log_categories: input.log_categories as i32,
            log_policy_events: input.log_policy_events as i32,
            retention_days: input.retention_days,
            subject_can_view: input.subject_can_view as i32,
            effective_from: now.clone(),
            effective_until: None,
            version: 1,
            created_at: now.clone(),
            updated_at: now,
        };

        diesel::insert_into(device_policies::table)
            .values(&new_policy)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

        device_policies::table
            .filter(device_policies::id.eq(&new_policy.id))
            .first::<DevicePolicy>(conn)
            .map_err(|e| StorageError::Internal(format!("Reload failed: {}", e)))
    }
}

/// Get all policies for a subject, ordered by author_tier
pub fn get_policies_for_subject(
    conn: &mut PooledConn,
    subject_id: &str,
) -> Result<Vec<DevicePolicy>, StorageError> {
    device_policies::table
        .filter(device_policies::subject_id.eq(subject_id))
        .order(device_policies::author_tier.asc())
        .load::<DevicePolicy>(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get a policy by ID
pub fn get_policy_by_id(
    conn: &mut PooledConn,
    id: &str,
) -> Result<DevicePolicy, StorageError> {
    device_policies::table
        .filter(device_policies::id.eq(id))
        .first::<DevicePolicy>(conn)
        .map_err(|e| StorageError::NotFound(format!("Policy {} not found: {}", id, e)))
}

/// Delete a policy by ID
pub fn delete_policy(conn: &mut PooledConn, id: &str) -> Result<(), StorageError> {
    let deleted = diesel::delete(device_policies::table.filter(device_policies::id.eq(id)))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    if deleted == 0 {
        return Err(StorageError::NotFound(format!("Policy {} not found", id)));
    }
    Ok(())
}
