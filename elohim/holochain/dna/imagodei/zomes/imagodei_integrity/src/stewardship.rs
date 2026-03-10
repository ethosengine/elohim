//! Stewardship Types for Graduated Capability Management
//!
//! This module defines entry types for managing stewardship relationships
//! where one agent can manage capabilities for another. This is NOT external
//! control - it's about identity and self-knowledge.
//!
//! Core philosophy:
//! - Everyone has limits (even the most capable benefit from exploring their constraints)
//! - Power scales with responsibility, not role assignment
//! - Relational accountability - limits negotiated through relationships
//! - Self-reflection tool - helps people recognize where they need support
//!
//! Use cases:
//! - Parents stewarding children's device access
//! - Organizations stewarding employee devices
//! - Adults with disabilities receiving guardian support
//! - Community intervention for moral deficit (with appeal rights)
//! - Self-imposed limits for personal discipline

use hdi::prelude::*;

// =============================================================================
// Stewardship Constants
// =============================================================================

/// Graduated capability tiers - same surface, different depth
/// Power scales with demonstrated responsibility, not assigned role
pub const STEWARD_CAPABILITY_TIERS: [&str; 5] = [
    "self",          // Manage own settings only
    "guide",         // Help others navigate their settings (advisory)
    "guardian",      // Manage settings for verified dependents
    "coordinator",   // Manage settings across organization/community
    "constitutional", // Elohim-level governance capabilities
];

/// Authority basis - how stewardship was established
/// Must be verifiable and reviewable
pub const AUTHORITY_BASIS_TYPES: [&str; 6] = [
    "minor_guardianship",   // Legal guardian of minor
    "court_order",          // Court-appointed custody
    "medical_necessity",    // Disability requiring care
    "community_consensus",  // Community-determined intervention
    "organizational_role",  // Device managed by organization
    "mutual_consent",       // Subject explicitly consented
];

/// Grant status lifecycle
pub const GRANT_STATUSES: [&str; 4] = [
    "active",    // Currently in effect
    "suspended", // Temporarily paused
    "expired",   // Past expiration date
    "revoked",   // Explicitly terminated
];

/// Appeal status lifecycle
pub const APPEAL_STATUSES: [&str; 4] = [
    "filed",     // Appeal submitted
    "reviewing", // Under arbitration
    "decided",   // Decision rendered
    "closed",    // Appeal process complete
];

/// Appeal types
pub const APPEAL_TYPES: [&str; 4] = [
    "scope",              // Challenging scope of capabilities granted
    "excessive",          // Claiming restrictions are disproportionate
    "invalid_evidence",   // Questioning authority basis evidence
    "capability_request", // Requesting additional capabilities
];

/// Content categories for filtering
pub const CONTENT_CATEGORIES: [&str; 8] = [
    "violence",
    "adult",
    "gambling",
    "substances",
    "hate",
    "self_harm",
    "spam",
    "misinformation",
];

/// Age ratings for content
pub const AGE_RATINGS: [&str; 5] = [
    "G",     // General audiences
    "PG",    // Parental guidance suggested
    "PG-13", // Some material may be inappropriate for children under 13
    "R",     // Restricted
    "NC-17", // Adults only
];

/// Features that can be restricted
pub const RESTRICTABLE_FEATURES: [&str; 10] = [
    "post",             // Create new content
    "share",            // Share content with others
    "vote",             // Participate in governance
    "comment",          // Add comments/replies
    "transfer",         // Transfer assets/points
    "direct_message",   // Send DMs
    "group_create",     // Create groups/communities
    "profile_edit",     // Modify own profile
    "external_links",   // Access external links
    "download",         // Download content locally
];

/// Inalienable rights that cannot be disabled even by coordinators
pub const INALIENABLE_FEATURES: [&str; 6] = [
    "capabilities_dashboard", // Subject always sees their restrictions
    "file_appeal",            // Subject always has appeal rights
    "contact_steward",        // Subject can always reach steward
    "elohim_chat",            // Subject can always access Elohim advocacy
    "emergency_call",         // Emergency contacts always work
    "time_status",            // Subject always sees remaining time
];

// =============================================================================
// StewardshipGrant - Authority to manage another's capabilities
// =============================================================================

/// Authority to steward another agent's device capabilities.
/// Earned through trust + verified need, not role assignment.
///
/// This is NOT about control - it's about helping someone navigate
/// their own limits through a trusted relationship.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StewardshipGrant {
    pub id: String,
    pub steward_id: String,  // Who has authority
    pub subject_id: String,  // Whose capabilities are managed

    // === AUTHORITY BASIS ===
    pub authority_basis: String, // See AUTHORITY_BASIS_TYPES
    pub evidence_hash: Option<String>, // Supporting documentation hash
    pub verified_by: String, // Who verified (Elohim, council, etc.)

    // === CAPABILITY SCOPE ===
    pub content_filtering: bool,    // Block/allow content
    pub time_limits: bool,          // Session/daily limits
    pub feature_restrictions: bool, // Disable features/routes
    pub activity_monitoring: bool,  // View usage logs
    pub policy_delegation: bool,    // Delegate to other stewards

    // === DELEGATION ===
    pub delegatable: bool, // Can this steward delegate to others?
    pub delegated_from: Option<String>, // Parent grant if delegated
    pub delegation_depth: u32, // How many levels of delegation (0 = original)

    // === LIFECYCLE ===
    pub granted_at: String,
    pub expires_at: String,  // Mandatory expiry
    pub review_at: String,   // Mandatory review date
    pub status: String,      // See GRANT_STATUSES

    // === APPEAL ===
    pub appeal_id: Option<String>, // Active appeal if any

    // === METADATA ===
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// DevicePolicy - Concrete rules applied to a device/agent
// =============================================================================

/// Concrete policy rules applied to a device.
/// Policies compose: Organization -> Guardian -> Elohim -> Subject customization
/// Each layer can only ADD restrictions, never remove parent layer restrictions.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DevicePolicy {
    pub id: String,
    pub subject_id: String,
    pub device_id: Option<String>, // Specific device or all if None

    // === AUTHORSHIP ===
    pub author_id: String,
    pub author_tier: String, // See STEWARD_CAPABILITY_TIERS
    pub inherits_from: Option<String>, // Parent policy ID

    // === CONTENT RULES ===
    pub blocked_categories_json: String, // Vec<String> as JSON
    pub blocked_hashes_json: String,     // Vec<String> as JSON (specific content)
    pub age_rating_max: Option<String>,  // See AGE_RATINGS
    pub reach_level_max: Option<u8>,     // Max reach (0-7)

    // === TIME RULES ===
    pub session_max_minutes: Option<u32>,
    pub daily_max_minutes: Option<u32>,
    pub time_windows_json: String, // Vec<TimeWindow> as JSON
    pub cooldown_minutes: Option<u32>,

    // === FEATURE RULES ===
    pub disabled_features_json: String, // Vec<String> as JSON
    pub disabled_routes_json: String,   // Vec<String> as JSON (path patterns)
    pub require_approval_json: String,  // Vec<String> as JSON (features needing OK)

    // === MONITORING RULES ===
    pub log_sessions: bool,
    pub log_categories: bool,     // Aggregated, not individual
    pub log_policy_events: bool,  // Violations/blocks
    pub retention_days: u32,
    pub subject_can_view: bool,   // Transparency - subject sees logs

    // === LIFECYCLE ===
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub version: u32,

    // === METADATA ===
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// PolicyInheritance - How policies compose across layers
// =============================================================================

/// How policies compose across governance layers.
/// Each layer can only ADD restrictions, never remove.
///
/// Chain order: org (0) -> guardian (1) -> elohim (2) -> subject (3)
/// Subject layer is for self-imposed limits.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PolicyInheritance {
    pub id: String,
    pub subject_id: String,
    pub chain_json: String, // Vec<PolicyChainLink> as JSON
    pub computed_policy_id: String, // Merged result policy ID
    pub computed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// StewardshipAppeal - Challenge a grant or policy
// =============================================================================

/// Appeal against stewardship grant or policy.
/// Everyone has the right to appeal - this is inalienable.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StewardshipAppeal {
    pub id: String,
    pub appellant_id: String, // Subject or advocate
    pub grant_id: String,     // Grant being appealed
    pub policy_id: Option<String>, // Specific policy if applicable

    // === APPEAL DETAILS ===
    pub appeal_type: String, // See APPEAL_TYPES
    pub grounds_json: String, // Vec<String> as JSON - reasons for appeal
    pub evidence_json: String, // Supporting evidence

    // === ADVOCACY ===
    pub advocate_id: Option<String>, // Optional advocate helping appellant
    pub advocate_notes: Option<String>,

    // === ARBITRATION ===
    pub arbitration_layer: String, // Constitutional layer handling
    pub assigned_to: Option<String>, // Arbitrator assigned

    // === STATUS ===
    pub status: String, // See APPEAL_STATUSES
    pub status_changed_at: Option<String>,

    // === DECISION ===
    pub decision_json: Option<String>, // AppealDecision as JSON
    pub decision_made_by: Option<String>,
    pub decision_made_at: Option<String>,

    // === TIMESTAMPS ===
    pub filed_at: String,
    pub expires_at: String, // Appeals must be decided within timeframe
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// ActivityLog - Usage tracking for monitoring
// =============================================================================

/// Activity log entry for monitoring.
/// Only created if activity_monitoring is enabled in grant.
/// Subject can always view if subject_can_view is true.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ActivityLog {
    pub id: String,
    pub subject_id: String,
    pub device_id: Option<String>,

    // === SESSION INFO ===
    pub session_id: String,
    pub session_started_at: String,
    pub session_duration_minutes: u32,

    // === ACTIVITY SUMMARY (aggregated, not individual) ===
    pub categories_accessed_json: String, // Vec<String> as JSON
    pub policy_events_json: String,       // Vec<PolicyEvent> as JSON (violations/blocks)

    // === METADATA ===
    pub logged_at: String,
    pub retention_expires_at: String,
}

// =============================================================================
// Validation Functions
// =============================================================================

/// Validate StewardshipGrant entry
pub fn validate_stewardship_grant(grant: &StewardshipGrant) -> ExternResult<ValidateCallbackResult> {
    if grant.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "StewardshipGrant ID cannot be empty".to_string(),
        ));
    }

    if grant.steward_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "StewardshipGrant steward_id cannot be empty".to_string(),
        ));
    }

    if grant.subject_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "StewardshipGrant subject_id cannot be empty".to_string(),
        ));
    }

    // Cannot steward yourself (except for self tier)
    if grant.steward_id == grant.subject_id {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot grant stewardship to yourself".to_string(),
        ));
    }

    if !AUTHORITY_BASIS_TYPES.contains(&grant.authority_basis.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid authority_basis '{}'. Must be one of: {:?}",
            grant.authority_basis, AUTHORITY_BASIS_TYPES
        )));
    }

    if !GRANT_STATUSES.contains(&grant.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid status '{}'. Must be one of: {:?}",
            grant.status, GRANT_STATUSES
        )));
    }

    // Delegation depth cannot exceed 3 (prevents infinite chains)
    if grant.delegation_depth > 3 {
        return Ok(ValidateCallbackResult::Invalid(
            "Delegation depth cannot exceed 3".to_string(),
        ));
    }

    // If delegated, must have delegated_from
    if grant.delegation_depth > 0 && grant.delegated_from.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Delegated grants must have delegated_from".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate DevicePolicy entry
pub fn validate_device_policy(policy: &DevicePolicy) -> ExternResult<ValidateCallbackResult> {
    if policy.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "DevicePolicy ID cannot be empty".to_string(),
        ));
    }

    if policy.subject_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "DevicePolicy subject_id cannot be empty".to_string(),
        ));
    }

    if policy.author_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "DevicePolicy author_id cannot be empty".to_string(),
        ));
    }

    if !STEWARD_CAPABILITY_TIERS.contains(&policy.author_tier.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid author_tier '{}'. Must be one of: {:?}",
            policy.author_tier, STEWARD_CAPABILITY_TIERS
        )));
    }

    // Validate age rating if provided
    if let Some(ref rating) = policy.age_rating_max {
        if !AGE_RATINGS.contains(&rating.as_str()) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "Invalid age_rating_max '{}'. Must be one of: {:?}",
                rating, AGE_RATINGS
            )));
        }
    }

    // Reach level max must be 0-7
    if let Some(reach) = policy.reach_level_max {
        if reach > 7 {
            return Ok(ValidateCallbackResult::Invalid(
                "reach_level_max must be 0-7".to_string(),
            ));
        }
    }

    // Retention days must be reasonable (max 365)
    if policy.retention_days > 365 {
        return Ok(ValidateCallbackResult::Invalid(
            "retention_days cannot exceed 365".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate PolicyInheritance entry
pub fn validate_policy_inheritance(inheritance: &PolicyInheritance) -> ExternResult<ValidateCallbackResult> {
    if inheritance.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "PolicyInheritance ID cannot be empty".to_string(),
        ));
    }

    if inheritance.subject_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "PolicyInheritance subject_id cannot be empty".to_string(),
        ));
    }

    if inheritance.computed_policy_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "PolicyInheritance computed_policy_id cannot be empty".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate StewardshipAppeal entry
pub fn validate_stewardship_appeal(appeal: &StewardshipAppeal) -> ExternResult<ValidateCallbackResult> {
    if appeal.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "StewardshipAppeal ID cannot be empty".to_string(),
        ));
    }

    if appeal.appellant_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "StewardshipAppeal appellant_id cannot be empty".to_string(),
        ));
    }

    if appeal.grant_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "StewardshipAppeal grant_id cannot be empty".to_string(),
        ));
    }

    if !APPEAL_TYPES.contains(&appeal.appeal_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid appeal_type '{}'. Must be one of: {:?}",
            appeal.appeal_type, APPEAL_TYPES
        )));
    }

    if !APPEAL_STATUSES.contains(&appeal.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid status '{}'. Must be one of: {:?}",
            appeal.status, APPEAL_STATUSES
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate ActivityLog entry
pub fn validate_activity_log(log: &ActivityLog) -> ExternResult<ValidateCallbackResult> {
    if log.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ActivityLog ID cannot be empty".to_string(),
        ));
    }

    if log.subject_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ActivityLog subject_id cannot be empty".to_string(),
        ));
    }

    if log.session_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ActivityLog session_id cannot be empty".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
