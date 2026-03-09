//! Stewardship service - business logic for stewardship policy and governance
//!
//! Encapsulates governance state transitions, dispute filing, ratio normalization,
//! and policy computation.
//!
//! ## Architecture
//!
//! Controller (api/stewardship.rs) → **Service (this file)** → Model (db/stewardship_allocations.rs, db/policy_cache.rs)

use chrono::Utc;
use serde_json::Value;

use crate::db::models::governance_states;
use crate::db::models::{ContentStewardship, StewardshipAllocation};
use crate::db::stewardship_allocations::{
    self, AllocationQuery, CreateAllocationInput, UpdateAllocationInput,
};
use crate::db::{AppContext, PooledConn};
use crate::error::StorageError;

/// Stewardship service for governance and policy management
pub struct StewardshipService;

impl StewardshipService {
    // =========================================================================
    // Allocation CRUD
    // =========================================================================

    /// List allocations with optional filters
    pub fn list_allocations(
        conn: &mut PooledConn,
        ctx: &AppContext,
        query: &AllocationQuery,
    ) -> Result<Vec<StewardshipAllocation>, StorageError> {
        stewardship_allocations::list_allocations(conn, ctx, query)
    }

    /// Get a single allocation by ID
    pub fn get_allocation(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
    ) -> Result<StewardshipAllocation, StorageError> {
        stewardship_allocations::get_allocation_by_id(conn, ctx, id)
    }

    /// Create a new stewardship allocation with validation
    pub fn create_allocation(
        conn: &mut PooledConn,
        ctx: &AppContext,
        input: CreateAllocationInput,
    ) -> Result<StewardshipAllocation, StorageError> {
        if input.content_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "contentId must not be empty".into(),
            ));
        }
        if input.steward_presence_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "stewardPresenceId must not be empty".into(),
            ));
        }
        stewardship_allocations::create_allocation(conn, ctx, &input)
    }

    /// Update an existing allocation
    pub fn update_allocation(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
        input: UpdateAllocationInput,
    ) -> Result<StewardshipAllocation, StorageError> {
        stewardship_allocations::update_allocation(conn, ctx, id, &input)
    }

    /// Delete an allocation (hard delete)
    pub fn delete_allocation(
        conn: &mut PooledConn,
        ctx: &AppContext,
        id: &str,
    ) -> Result<(), StorageError> {
        stewardship_allocations::delete_allocation(conn, ctx, id)
    }

    // =========================================================================
    // Lookup helpers
    // =========================================================================

    /// Get all allocations for a content piece
    pub fn get_allocations_for_content(
        conn: &mut PooledConn,
        ctx: &AppContext,
        content_id: &str,
    ) -> Result<Vec<StewardshipAllocation>, StorageError> {
        stewardship_allocations::get_allocations_for_content(conn, ctx, content_id)
    }

    /// Get all allocations for a steward
    pub fn get_allocations_for_steward(
        conn: &mut PooledConn,
        ctx: &AppContext,
        steward_presence_id: &str,
    ) -> Result<Vec<StewardshipAllocation>, StorageError> {
        stewardship_allocations::get_allocations_for_steward(conn, ctx, steward_presence_id)
    }

    /// Get full content stewardship aggregate (allocations + presences)
    pub fn get_content_stewardship(
        conn: &mut PooledConn,
        ctx: &AppContext,
        content_id: &str,
    ) -> Result<ContentStewardship, StorageError> {
        stewardship_allocations::get_content_stewardship(conn, ctx, content_id)
    }

    // =========================================================================
    // Governance transitions
    // =========================================================================

    /// File a dispute on an allocation.
    ///
    /// Transitions state: Active → Disputed.
    /// Generates a dispute ID as `dispute-{timestamp_ms}-{random_hex}`.
    pub fn file_dispute(
        conn: &mut PooledConn,
        ctx: &AppContext,
        allocation_id: &str,
        disputed_by: &str,
        reason: &str,
    ) -> Result<StewardshipAllocation, StorageError> {
        if disputed_by.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "disputedBy must not be empty".into(),
            ));
        }
        if reason.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "reason must not be empty".into(),
            ));
        }

        // Verify current state allows transitioning to DISPUTED
        let allocation = stewardship_allocations::get_allocation_by_id(conn, ctx, allocation_id)?;
        if allocation.governance_state == governance_states::DISPUTED {
            return Err(StorageError::InvalidInput(
                "Allocation already has an active dispute".into(),
            ));
        }
        if allocation.governance_state == governance_states::SUPERSEDED {
            return Err(StorageError::InvalidInput(
                "Cannot dispute a superseded allocation".into(),
            ));
        }

        let now_ms = Utc::now().timestamp_millis();
        let random_suffix = format!("{:x}", now_ms & 0xFFFF);
        let dispute_id = format!("dispute-{}-{}", now_ms, random_suffix);

        stewardship_allocations::file_dispute(
            conn,
            ctx,
            allocation_id,
            &dispute_id,
            disputed_by,
            reason,
        )
    }

    /// Resolve a dispute (Elohim ratification).
    ///
    /// Valid target states: active, pending_review, superseded.
    pub fn resolve_dispute(
        conn: &mut PooledConn,
        ctx: &AppContext,
        allocation_id: &str,
        ratifier_id: &str,
        new_state: &str,
    ) -> Result<StewardshipAllocation, StorageError> {
        if ratifier_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "ratifierId must not be empty".into(),
            ));
        }

        let allowed_target_states = [
            governance_states::ACTIVE,
            governance_states::PENDING_REVIEW,
            governance_states::SUPERSEDED,
        ];
        if !allowed_target_states.contains(&new_state) {
            return Err(StorageError::InvalidInput(format!(
                "Invalid target state '{}'. Must be one of: active, pending_review, superseded",
                new_state
            )));
        }

        stewardship_allocations::resolve_dispute(conn, ctx, allocation_id, ratifier_id, new_state)
    }

    // =========================================================================
    // Policy computation (passthrough from policy cache layer)
    // =========================================================================

    /// Build a computed policy response from a cached policy JSON blob.
    ///
    /// Parses JSON string sub-fields and transforms Rust enum format to flat TypeScript format.
    /// Input `policy_json` is the raw JSON string from the cached_policies table.
    /// Returns a `serde_json::Value` ready to serialize to the client.
    pub fn transform_cached_policy(agent_id: &str, policy_json: &str) -> Value {
        let raw: Value = match serde_json::from_str(policy_json) {
            Ok(v) => v,
            Err(_) => {
                return serde_json::json!({ "subjectId": agent_id, "error": "invalid policy" })
            }
        };

        let obj = match raw.as_object() {
            Some(o) => o,
            None => return raw,
        };

        // Parse array sub-fields stored as JSON strings
        let blocked_categories = parse_json_string_field(obj, "blockedCategories");
        let blocked_hashes = parse_json_string_field(obj, "blockedHashes");
        let disabled_features = parse_json_string_field(obj, "disabledFeatures");
        let disabled_routes = parse_json_string_field(obj, "disabledRoutes");
        let require_approval = parse_json_string_field(obj, "requireApproval");
        let time_windows = parse_json_string_field(obj, "timeWindowsJson");

        // Optional scalar fields (omit if null)
        let age_rating_max =
            obj.get("ageRatingMax")
                .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });
        let reach_level_max =
            obj.get("reachLevelMax")
                .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });
        let session_max_minutes =
            obj.get("sessionMaxMinutes")
                .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });
        let daily_max_minutes =
            obj.get("dailyMaxMinutes")
                .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });
        let cooldown_minutes =
            obj.get("cooldownMinutes")
                .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });

        // Build nested contentRules
        let mut content_rules = serde_json::json!({
            "blockedCategories": blocked_categories,
            "blockedHashes": blocked_hashes,
        });
        if let Some(v) = &age_rating_max {
            content_rules["ageRatingMax"] = v.clone();
        }
        if let Some(v) = &reach_level_max {
            content_rules["reachLevelMax"] = v.clone();
        }

        // Build nested timeRules
        let mut time_rules = serde_json::json!({
            "timeWindows": time_windows,
        });
        if let Some(v) = &session_max_minutes {
            time_rules["sessionMaxMinutes"] = v.clone();
        }
        if let Some(v) = &daily_max_minutes {
            time_rules["dailyMaxMinutes"] = v.clone();
        }
        if let Some(v) = &cooldown_minutes {
            time_rules["cooldownMinutes"] = v.clone();
        }

        let feature_rules = serde_json::json!({
            "disabledFeatures": disabled_features,
            "disabledRoutes": disabled_routes,
            "requireApproval": require_approval,
        });

        let mut out = serde_json::json!({
            "subjectId": agent_id,
            "computedAt": obj.get("computedAt").cloned().unwrap_or(Value::Null),
            "contentRules": content_rules,
            "timeRules": time_rules,
            "featureRules": feature_rules,
            // Flat aliases for backwards compat
            "blockedCategories": blocked_categories,
            "blockedHashes": blocked_hashes,
            "timeWindows": time_windows,
            "disabledFeatures": disabled_features,
            "disabledRoutes": disabled_routes,
            "requireApproval": require_approval,
            "logSessions": obj.get("logSessions").cloned().unwrap_or(Value::Bool(false)),
            "logCategories": obj.get("logCategories").cloned().unwrap_or(Value::Bool(false)),
            "logPolicyEvents": obj.get("logPolicyEvents").cloned().unwrap_or(Value::Bool(true)),
            "retentionDays": obj.get("retentionDays").cloned().unwrap_or(serde_json::json!(30)),
            "subjectCanView": obj.get("subjectCanView").cloned().unwrap_or(Value::Bool(true)),
        });

        // Append optional scalar fields
        if let Some(v) = age_rating_max {
            out["ageRatingMax"] = v;
        }
        if let Some(v) = reach_level_max {
            out["reachLevelMax"] = v;
        }
        if let Some(v) = session_max_minutes {
            out["sessionMaxMinutes"] = v;
        }
        if let Some(v) = daily_max_minutes {
            out["dailyMaxMinutes"] = v;
        }
        if let Some(v) = cooldown_minutes {
            out["cooldownMinutes"] = v;
        }

        out
    }

    // =========================================================================
    // Device Policy Operations
    // =========================================================================

    /// Upsert a device policy for a subject
    pub fn upsert_device_policy(
        conn: &mut PooledConn,
        input: &crate::db::device_policies::CreateDevicePolicyInput,
    ) -> Result<crate::db::models::DevicePolicy, StorageError> {
        if input.subject_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "subjectId must not be empty".into(),
            ));
        }
        if input.author_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "authorId must not be empty".into(),
            ));
        }
        crate::db::device_policies::upsert_policy(conn, input)
    }

    /// Get all policies for a subject
    pub fn get_policies_for_subject(
        conn: &mut PooledConn,
        subject_id: &str,
    ) -> Result<Vec<crate::db::models::DevicePolicy>, StorageError> {
        crate::db::device_policies::get_policies_for_subject(conn, subject_id)
    }

    /// Get the most recent active policy for a subject
    pub fn get_subject_policy(
        conn: &mut PooledConn,
        subject_id: &str,
    ) -> Result<Option<crate::db::models::DevicePolicy>, StorageError> {
        let policies = crate::db::device_policies::get_policies_for_subject(conn, subject_id)?;
        Ok(policies.into_iter().next())
    }

    /// Get the parent's computed policy for a subject.
    /// Walks the inherits_from chain to find the parent policy.
    pub fn get_parent_policy(
        conn: &mut PooledConn,
        subject_id: &str,
    ) -> Result<Option<Value>, StorageError> {
        let policies = crate::db::device_policies::get_policies_for_subject(conn, subject_id)?;
        let parent_id = policies.iter().find_map(|p| p.inherits_from.clone());
        match parent_id {
            Some(pid) => match crate::db::device_policies::get_policy_by_id(conn, &pid) {
                Ok(parent) => Ok(Some(Self::policy_to_computed_json(&parent))),
                Err(_) => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// Get the policy chain for a subject as PolicyChainLinks
    pub fn get_policy_chain(
        conn: &mut PooledConn,
        subject_id: &str,
    ) -> Result<Vec<Value>, StorageError> {
        let policies = crate::db::device_policies::get_policies_for_subject(conn, subject_id)?;
        Ok(policies
            .iter()
            .map(|p| {
                serde_json::json!({
                    "policyId": p.id,
                    "authorTier": p.author_tier,
                    "layerOrder": tier_to_layer_order(&p.author_tier),
                })
            })
            .collect())
    }

    /// Convert a DevicePolicy to computed policy JSON
    fn policy_to_computed_json(p: &crate::db::models::DevicePolicy) -> Value {
        let blocked_categories: Value =
            serde_json::from_str(&p.blocked_categories_json).unwrap_or(Value::Array(vec![]));
        let blocked_hashes: Value =
            serde_json::from_str(&p.blocked_hashes_json).unwrap_or(Value::Array(vec![]));
        let disabled_features: Value =
            serde_json::from_str(&p.disabled_features_json).unwrap_or(Value::Array(vec![]));
        let disabled_routes: Value =
            serde_json::from_str(&p.disabled_routes_json).unwrap_or(Value::Array(vec![]));
        let require_approval: Value =
            serde_json::from_str(&p.require_approval_json).unwrap_or(Value::Array(vec![]));
        let time_windows: Value =
            serde_json::from_str(&p.time_windows_json).unwrap_or(Value::Array(vec![]));

        serde_json::json!({
            "subjectId": p.subject_id,
            "computedAt": p.updated_at,
            "blockedCategories": blocked_categories,
            "blockedHashes": blocked_hashes,
            "ageRatingMax": p.age_rating_max,
            "reachLevelMax": p.reach_level_max,
            "sessionMaxMinutes": p.session_max_minutes,
            "dailyMaxMinutes": p.daily_max_minutes,
            "timeWindows": time_windows,
            "cooldownMinutes": p.cooldown_minutes,
            "disabledFeatures": disabled_features,
            "disabledRoutes": disabled_routes,
            "requireApproval": require_approval,
            "logSessions": p.log_sessions != 0,
            "logCategories": p.log_categories != 0,
            "logPolicyEvents": p.log_policy_events != 0,
            "retentionDays": p.retention_days,
            "subjectCanView": p.subject_can_view != 0,
        })
    }

    // =========================================================================
    // Normalization helpers
    // =========================================================================

    /// Normalize ratios for a content item so they sum to 1.0.
    ///
    /// Updates each allocation's `allocation_ratio` proportionally.
    /// Returns the updated allocations.
    pub fn normalize_ratios(
        conn: &mut PooledConn,
        ctx: &AppContext,
        content_id: &str,
    ) -> Result<Vec<StewardshipAllocation>, StorageError> {
        let allocations =
            stewardship_allocations::get_allocations_for_content(conn, ctx, content_id)?;

        if allocations.is_empty() {
            return Ok(allocations);
        }

        let total: f32 = allocations.iter().map(|a| a.allocation_ratio).sum();
        if total <= 0.0 {
            return Err(StorageError::InvalidInput(
                "Cannot normalize: total allocation ratio is zero".into(),
            ));
        }

        // Only normalize if total differs from 1.0 by more than floating-point epsilon
        if (total - 1.0f32).abs() < 1e-5 {
            return Ok(allocations);
        }

        let mut updated = Vec::with_capacity(allocations.len());
        for alloc in &allocations {
            let new_ratio = alloc.allocation_ratio / total;
            let result = stewardship_allocations::update_allocation(
                conn,
                ctx,
                &alloc.id,
                &UpdateAllocationInput {
                    allocation_ratio: Some(new_ratio),
                    allocation_method: None,
                    contribution_type: None,
                    contribution_evidence_json: None,
                    governance_state: None,
                    dispute_id: None,
                    dispute_reason: None,
                    elohim_ratified_at: None,
                    elohim_ratifier_id: None,
                    note: None,
                },
            )?;
            updated.push(result);
        }

        Ok(updated)
    }
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Map author tier to layer order for policy chain
fn tier_to_layer_order(tier: &str) -> i32 {
    match tier {
        "self" => 0,
        "guide" => 1,
        "guardian" => 2,
        "coordinator" => 3,
        "constitutional" => 4,
        _ => 0,
    }
}

/// Parse a field that may be either a JSON string or an already-parsed JSON value.
/// Returns the parsed value or an empty array on failure.
fn parse_json_string_field(obj: &serde_json::Map<String, Value>, field: &str) -> Value {
    match obj.get(field) {
        Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(Value::Array(vec![])),
        Some(v) if !v.is_null() => v.clone(),
        _ => Value::Array(vec![]),
    }
}
