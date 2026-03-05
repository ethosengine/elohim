//! Stewardship Route Handler - Graduated Capability Management
//!
//! API routes for managing stewardship relationships where one agent can manage
//! capabilities for another. Power scales with responsibility, not role assignment.
//!
//! ## Architecture
//!
//! ```text
//! Browser → Doorway → elohim-storage (ImagoDei zome)
//!              │           │
//!         (proxy+auth) (stewardship operations)
//! ```
//!
//! ## Wire Format Transformation
//!
//! This handler performs the following transformations between the thin Angular
//! client and elohim-storage:
//!
//! ### POST Requests (grants, policies, appeals, activity)
//! - Parse and validate request bodies
//! - Transform field names: `parentGrantId` → `parent_grantId`, `newStewardId` → `new_stewardId`
//! - Forward transformed payload to storage
//! - Input validation: createGrant requires subjectId+authorityBasis, delegateGrant requires
//!   parentGrantId+newStewardId, fileAppeal requires grantId+appealType+grounds
//!
//! ### GET Responses (policy, grants, appeals, activity)
//! - Intercept storage responses
//! - Parse *Json string fields into proper JSON objects/arrays
//! - Convert Rust enum format `{Allow: null, Block: {reason}}` → `{type, reason?}`
//! - Forward transformed response to client
//!
//! ## Endpoints
//!
//! ### Policy Checks
//! - `GET  /api/v1/stewardship/policy`                     - Get my computed policy
//! - `GET  /api/v1/stewardship/access/content/{contentId}` - Check content access
//! - `GET  /api/v1/stewardship/access/feature/{feature}`   - Check feature access
//! - `GET  /api/v1/stewardship/access/route/{route}`       - Check route access
//!
//! ### Grant Operations
//! - `POST   /api/v1/stewardship/grants`              - Create a stewardship grant
//! - `POST   /api/v1/stewardship/grants/{id}/delegate` - Delegate an existing grant
//! - `DELETE /api/v1/stewardship/grants/{id}`          - Revoke a grant
//! - `GET    /api/v1/stewardship/subjects`             - Get grants where I am steward
//! - `GET    /api/v1/stewardship/stewards`             - Get grants where I am stewarded
//!
//! ### Policy Management
//! - `POST /api/v1/stewardship/policies` - Create or update a device policy
//! - `GET  /api/v1/stewardship/policies/{subjectId}` - Get policies for a subject
//!
//! ### Appeal Operations
//! - `POST /api/v1/stewardship/appeals` - File an appeal
//! - `GET  /api/v1/stewardship/appeals` - Get my appeals
//!
//! ### Activity Logging
//! - `POST /api/v1/stewardship/activity` - Log session activity

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::server::AppState;

// =============================================================================
// Enums
// =============================================================================

/// Graduated capability tiers - same surface, different depth.
/// Power scales with demonstrated responsibility, not assigned role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StewardCapabilityTier {
    /// Manage own settings only
    #[serde(rename = "self")]
    SelfTier,
    /// Help others navigate their settings (advisory)
    Guide,
    /// Manage settings for verified dependents
    Guardian,
    /// Manage settings across organization/community
    Coordinator,
    /// Elohim-level governance capabilities
    Constitutional,
}

/// Authority basis - how stewardship was established.
/// Must be verifiable and reviewable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityBasis {
    /// Legal guardian of minor
    MinorGuardianship,
    /// Court-appointed custody
    CourtOrder,
    /// Disability requiring care
    MedicalNecessity,
    /// Community-determined intervention
    CommunityCensus,
    /// Device managed by organization
    OrganizationalRole,
    /// Subject explicitly consented
    MutualConsent,
}

/// Grant status lifecycle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GrantStatus {
    Active,
    Suspended,
    Expired,
    Revoked,
}

/// Appeal status lifecycle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppealStatus {
    Filed,
    Reviewing,
    Decided,
    Closed,
}

/// Appeal type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppealType {
    /// Challenging scope of capabilities granted
    Scope,
    /// Claiming restrictions are disproportionate
    Excessive,
    /// Questioning authority basis evidence
    InvalidEvidence,
    /// Requesting additional capabilities
    CapabilityRequest,
}

/// Policy decision effect
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PolicyEffect {
    Allow,
    Block,
}

// =============================================================================
// Domain Structs
// =============================================================================

/// Computed policy for a subject (merged from all layers).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedPolicy {
    pub subject_id: String,
    pub computed_at: String,

    // Merged content rules
    pub blocked_categories: Vec<String>,
    pub blocked_hashes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_rating_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach_level_max: Option<u32>,

    // Merged time rules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_max_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_max_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_windows_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_minutes: Option<u32>,

    // Merged feature rules
    pub disabled_features: Vec<String>,
    pub disabled_routes: Vec<String>,
    pub require_approval: Vec<String>,

    // Merged monitoring rules
    pub log_sessions: bool,
    pub log_categories: bool,
    pub log_policy_events: bool,
    pub retention_days: u32,
    pub subject_can_view: bool,
}

/// Content filtering rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentFilterRules {
    pub blocked_categories: Vec<String>,
    pub blocked_hashes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_rating_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach_level_max: Option<u32>,
}

/// Time limit rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeLimitRules {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_max_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_max_minutes: Option<u32>,
    pub time_windows: Vec<TimeWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_minutes: Option<u32>,
}

/// Time window for allowed access
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeWindow {
    pub days_of_week: Vec<u8>,
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
}

/// Feature restriction rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureRestrictionRules {
    pub disabled_features: Vec<String>,
    pub disabled_routes: Vec<String>,
    pub require_approval: Vec<String>,
}

/// A single policy rule within a device policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRule {
    pub rule_type: String,
    pub effect: PolicyEffect,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Concrete policy rules applied to a device.
/// Policies compose: Organization -> Guardian -> Elohim -> Subject customization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicePolicy {
    pub id: String,
    pub subject_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,

    // Authorship
    pub author_id: String,
    pub author_tier: StewardCapabilityTier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherits_from: Option<String>,

    // Nested rule objects
    pub content_rules: ContentFilterRules,
    pub time_rules: TimeLimitRules,
    pub feature_rules: FeatureRestrictionRules,

    // Monitoring rules
    pub log_sessions: bool,
    pub log_categories: bool,
    pub log_policy_events: bool,
    pub retention_days: u32,
    pub subject_can_view: bool,

    // Lifecycle
    pub effective_from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_until: Option<String>,
    pub version: u32,

    // Metadata
    pub created_at: String,
    pub updated_at: String,
}

/// Authority to steward another agent's device capabilities.
/// Earned through trust + verified need, not role assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StewardshipGrant {
    pub id: String,
    pub steward_id: String,
    pub subject_id: String,

    // Tier level
    pub tier: StewardCapabilityTier,

    // Authority basis
    pub authority_basis: AuthorityBasis,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_hash: Option<String>,
    pub verified_by: String,

    // Capabilities
    pub content_filtering: bool,
    pub time_limits: bool,
    pub feature_restrictions: bool,
    pub activity_monitoring: bool,
    pub policy_delegation: bool,

    // Delegation
    pub delegatable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegated_from: Option<String>,
    pub delegation_depth: u32,

    // Lifecycle
    pub granted_at: String,
    pub expires_at: String,
    pub review_at: String,
    pub status: GrantStatus,

    // Appeal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appeal_id: Option<String>,

    // Metadata
    pub created_at: String,
    pub updated_at: String,
}

/// Appeal decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppealDecision {
    pub approved: bool,
    pub notes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifications: Option<AppealModifications>,
    pub decided_by: String,
    pub decided_at: String,
}

/// Modifications resulting from an appeal decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppealModifications {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities_reduced: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_extended: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_expiration: Option<String>,
}

/// Appeal against stewardship grant or policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StewardshipAppeal {
    pub id: String,
    pub appellant_id: String,
    pub grant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,

    // Appeal details
    pub appeal_type: AppealType,
    pub grounds: Vec<String>,
    pub evidence_json: String,

    // Advocacy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advocate_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advocate_notes: Option<String>,

    // Arbitration
    pub arbitration_layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,

    // Status
    pub status: AppealStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_changed_at: Option<String>,

    // Decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<AppealDecision>,

    // Timestamps
    pub filed_at: String,
    pub expires_at: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Request Types
// =============================================================================

/// Request body for creating a stewardship grant
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGrantRequest {
    pub subject_id: String,
    pub authority_basis: AuthorityBasis,
    #[serde(default)]
    pub evidence_hash: Option<String>,
    pub verified_by: String,
    // Capabilities
    pub content_filtering: bool,
    pub time_limits: bool,
    pub feature_restrictions: bool,
    pub activity_monitoring: bool,
    pub policy_delegation: bool,
    // Options
    pub delegatable: bool,
    pub expires_in_days: u32,
    pub review_in_days: u32,
}

/// Request body for delegating a grant to another steward
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateGrantRequest {
    pub parent_grant_id: String,
    pub new_steward_id: String,
    // Optional capability restrictions (narrower than parent)
    #[serde(default)]
    pub content_filtering: Option<bool>,
    #[serde(default)]
    pub time_limits: Option<bool>,
    #[serde(default)]
    pub feature_restrictions: Option<bool>,
    #[serde(default)]
    pub activity_monitoring: Option<bool>,
    #[serde(default)]
    pub policy_delegation: Option<bool>,
    pub expires_in_days: u32,
}

/// Request body for upserting a device policy.
/// Accepts nested rules format from the Angular policy console.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPolicyRequest {
    #[serde(default)]
    pub subject_id: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    // Content rules (nested)
    pub content_rules: ContentRulesInput,
    // Time rules (nested)
    pub time_rules: TimeRulesInput,
    // Feature rules (nested)
    pub feature_rules: FeatureRulesInput,
    // Monitoring rules (optional, with defaults)
    #[serde(default)]
    pub monitoring_rules: Option<MonitoringRulesInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentRulesInput {
    pub blocked_categories: Vec<String>,
    pub blocked_hashes: Vec<String>,
    #[serde(default)]
    pub age_rating_max: Option<String>,
    #[serde(default)]
    pub reach_level_max: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRulesInput {
    #[serde(default)]
    pub session_max_minutes: Option<u32>,
    #[serde(default)]
    pub daily_max_minutes: Option<u32>,
    #[serde(default)]
    pub time_windows: Vec<TimeWindow>,
    #[serde(default)]
    pub cooldown_minutes: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureRulesInput {
    pub disabled_features: Vec<String>,
    pub disabled_routes: Vec<String>,
    pub require_approval: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoringRulesInput {
    #[serde(default)]
    pub log_sessions: bool,
    #[serde(default)]
    pub log_categories: bool,
    #[serde(default = "default_true")]
    pub log_policy_events: bool,
    #[serde(default = "default_30")]
    pub retention_days: u32,
    #[serde(default = "default_true")]
    pub subject_can_view: bool,
}

fn default_true() -> bool {
    true
}

fn default_30() -> u32 {
    30
}

/// Request body for filing an appeal
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileAppealRequest {
    pub grant_id: String,
    #[serde(default)]
    pub policy_id: Option<String>,
    pub appeal_type: AppealType,
    pub grounds: Vec<String>,
    pub evidence_json: String,
    #[serde(default)]
    pub advocate_id: Option<String>,
}

/// Request body for logging session activity
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogActivityRequest {
    pub session_id: String,
    pub session_duration_minutes: u32,
    pub categories_accessed: Vec<String>,
    pub policy_events: Vec<Value>,
}

// =============================================================================
// Response Types
// =============================================================================

/// Response for access check endpoints
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessCheckResponse {
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Generic API error response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StewardshipApiError {
    error: String,
    code: &'static str,
}

// =============================================================================
// Response Helpers
// =============================================================================

/// Build a JSON error response
fn error_response(status: StatusCode, message: &str, code: &'static str) -> Response<Full<Bytes>> {
    let error = StewardshipApiError {
        error: message.to_string(),
        code,
    };
    let body = serde_json::to_vec(&error).unwrap_or_default();

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "no-cache")
        .body(Full::new(Bytes::from(body)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(r#"{"error":"Internal error"}"#)))
                .unwrap()
        })
}

/// Build a JSON success response
fn json_response(status: StatusCode, body: &Value) -> Response<Full<Bytes>> {
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(Bytes::from(bytes)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(r#"{"error":"Internal error"}"#)))
                .unwrap()
        })
}

// =============================================================================
// Wire Format Transformation Helpers
// =============================================================================

/// Transform a raw storage grant object into the client-facing format.
///
/// Storage may return `null` values for optional fields or bare string status/tier values.
/// This normalises the shape to match what Angular expects.
fn transform_grant(raw: &Value) -> Value {
    let mut grant = raw.clone();

    // null → absent for optional fields
    if let Some(obj) = grant.as_object_mut() {
        for key in &["evidenceHash", "delegatedFrom", "appealId"] {
            if obj.get(*key) == Some(&Value::Null) {
                obj.remove(*key);
            }
        }
    }

    grant
}

/// Transform a raw storage policy object into the client-facing format.
///
/// Storage returns *Json fields as JSON strings. We parse them into proper
/// JSON values and construct the nested contentRules/timeRules/featureRules objects.
fn transform_policy(raw: &Value) -> Value {
    let obj = match raw.as_object() {
        Some(o) => o,
        None => return raw.clone(),
    };

    // Parse *Json string fields
    let blocked_categories: Value = parse_json_field(obj, "blockedCategoriesJson");
    let blocked_hashes: Value = parse_json_field(obj, "blockedHashesJson");
    let time_windows: Value = parse_json_field(obj, "timeWindowsJson");
    let disabled_features: Value = parse_json_field(obj, "disabledFeaturesJson");
    let disabled_routes: Value = parse_json_field(obj, "disabledRoutesJson");
    let require_approval: Value = parse_json_field(obj, "requireApprovalJson");

    // Optional scalar fields (null → absent)
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
    let device_id = obj
        .get("deviceId")
        .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });
    let inherits_from =
        obj.get("inheritsFrom")
            .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });
    let effective_until =
        obj.get("effectiveUntil")
            .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });

    // Build nested structure that Angular expects
    let mut content_rules = json!({
        "blockedCategories": blocked_categories,
        "blockedHashes": blocked_hashes,
    });
    if let Some(v) = &age_rating_max {
        content_rules["ageRatingMax"] = v.clone();
    }
    if let Some(v) = &reach_level_max {
        content_rules["reachLevelMax"] = v.clone();
    }

    let mut time_rules = json!({
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

    let feature_rules = json!({
        "disabledFeatures": disabled_features,
        "disabledRoutes": disabled_routes,
        "requireApproval": require_approval,
    });

    // Build the output object preserving scalar fields
    let mut out = json!({
        "id": obj.get("id").cloned().unwrap_or(Value::Null),
        "subjectId": obj.get("subjectId").cloned().unwrap_or(Value::Null),
        "authorId": obj.get("authorId").cloned().unwrap_or(Value::Null),
        "authorTier": obj.get("authorTier").cloned().unwrap_or(Value::Null),
        "contentRules": content_rules,
        "timeRules": time_rules,
        "featureRules": feature_rules,
        // Flat aliases for backwards compatibility
        "blockedCategories": blocked_categories,
        "blockedHashes": blocked_hashes,
        "timeWindows": time_windows,
        "disabledFeatures": disabled_features,
        "disabledRoutes": disabled_routes,
        "requireApproval": require_approval,
        "logSessions": obj.get("logSessions").cloned().unwrap_or(Value::Bool(false)),
        "logCategories": obj.get("logCategories").cloned().unwrap_or(Value::Bool(false)),
        "logPolicyEvents": obj.get("logPolicyEvents").cloned().unwrap_or(Value::Bool(true)),
        "retentionDays": obj.get("retentionDays").cloned().unwrap_or(json!(30)),
        "subjectCanView": obj.get("subjectCanView").cloned().unwrap_or(Value::Bool(true)),
        "effectiveFrom": obj.get("effectiveFrom").cloned().unwrap_or(Value::Null),
        "version": obj.get("version").cloned().unwrap_or(json!(1)),
        "createdAt": obj.get("createdAt").cloned().unwrap_or(Value::Null),
        "updatedAt": obj.get("updatedAt").cloned().unwrap_or(Value::Null),
    });

    if let Some(v) = device_id {
        out["deviceId"] = v;
    }
    if let Some(v) = inherits_from {
        out["inheritsFrom"] = v;
    }
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
    if let Some(v) = effective_until {
        out["effectiveUntil"] = v;
    }

    out
}

/// Transform a raw appeal object into the client-facing format.
///
/// Parses groundsJson and decisionJson string fields into proper JSON.
fn transform_appeal(raw: &Value) -> Value {
    let obj = match raw.as_object() {
        Some(o) => o,
        None => return raw.clone(),
    };

    let grounds = parse_json_field(obj, "groundsJson");
    let decision: Option<Value> = obj
        .get("decisionJson")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str(s).ok());

    let mut out = raw.clone();
    if let Some(out_obj) = out.as_object_mut() {
        // Replace groundsJson with parsed grounds array
        out_obj.remove("groundsJson");
        out_obj.insert("grounds".to_string(), grounds);

        // Replace decisionJson with parsed decision object (or remove if null/absent)
        out_obj.remove("decisionJson");
        if let Some(d) = decision {
            out_obj.insert("decision".to_string(), d);
        }

        // null → absent for optional fields
        for key in &[
            "policyId",
            "advocateId",
            "advocateNotes",
            "assignedTo",
            "statusChangedAt",
            "decisionMadeBy",
            "decisionMadeAt",
        ] {
            if out_obj.get(*key) == Some(&Value::Null) {
                out_obj.remove(*key);
            }
        }
    }

    out
}

/// Transform a raw activity log object into the client-facing format.
///
/// Parses categoriesAccessedJson and policyEventsJson string fields.
fn transform_activity_log(raw: &Value) -> Value {
    let obj = match raw.as_object() {
        Some(o) => o,
        None => return raw.clone(),
    };

    let categories_accessed = parse_json_field(obj, "categoriesAccessedJson");
    let policy_events = parse_json_field(obj, "policyEventsJson");

    let mut out = raw.clone();
    if let Some(out_obj) = out.as_object_mut() {
        out_obj.remove("categoriesAccessedJson");
        out_obj.remove("policyEventsJson");
        out_obj.insert("categoriesAccessed".to_string(), categories_accessed);
        out_obj.insert("policyEvents".to_string(), policy_events);

        if out_obj.get("deviceId") == Some(&Value::Null) {
            out_obj.remove("deviceId");
        }
    }

    out
}

/// Transform a raw policy decision from Rust enum format into flat format.
///
/// Storage may return: `{"Allow": null}` or `{"Block": {"reason": "..."}}`
/// Angular expects: `{"type": "allow"}` or `{"type": "block", "reason": "..."}`
fn transform_policy_decision(raw: &Value) -> Value {
    if let Some(obj) = raw.as_object() {
        // Rust enum external tagging: {"Allow": null} or {"Block": {"reason": "..."}}
        if obj.contains_key("Allow") {
            return json!({"type": "allow"});
        }
        if let Some(block_val) = obj.get("Block") {
            let reason = block_val
                .as_object()
                .and_then(|b| b.get("reason"))
                .and_then(|r| r.as_str())
                .unwrap_or("Blocked by policy");
            return json!({"type": "block", "reason": reason});
        }
        // Already flat format (storage may already transform)
        if let Some(type_val) = obj.get("type") {
            if type_val == "allow" || type_val == "block" {
                return raw.clone();
            }
        }
    }
    // Default to allow if unparseable
    json!({"type": "allow"})
}

/// Parse a JSON string field from a map, returning the parsed value or an empty array on failure.
fn parse_json_field(obj: &serde_json::Map<String, Value>, field: &str) -> Value {
    obj.get(field)
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Array(vec![]))
}

// =============================================================================
// Route Dispatcher
// =============================================================================

/// Handle all `/api/v1/stewardship/*` requests.
///
/// Dispatches to the appropriate handler based on the sub-path and HTTP method.
/// Each handler proxies to elohim-storage at the corresponding `/db/stewardship/...` path,
/// performing wire format transformation on both the request and response.
pub async fn handle_stewardship_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let method = req.method().clone();
    // Strip the prefix to get the sub-route
    let sub_path = path
        .strip_prefix("/api/v1/stewardship")
        .unwrap_or(path)
        .trim_start_matches('/');

    // Map API path to storage path
    let storage_path = path.replacen("/api/v1/stewardship", "/db/stewardship", 1);

    match (&method, sub_path) {
        // =====================================================================
        // Policy Checks
        // =====================================================================
        (&Method::GET, "policy") => {
            handle_get_with_transform(req, &state, &storage_path, transform_computed_policy).await
        }

        (&Method::GET, p) if p.starts_with("access/content/") => {
            handle_get_with_transform(
                req,
                &state,
                &storage_path,
                transform_policy_decision_response,
            )
            .await
        }

        (&Method::GET, p) if p.starts_with("access/feature/") => {
            handle_get_with_transform(
                req,
                &state,
                &storage_path,
                transform_policy_decision_response,
            )
            .await
        }

        (&Method::GET, p) if p.starts_with("access/route/") => {
            handle_get_with_transform(
                req,
                &state,
                &storage_path,
                transform_policy_decision_response,
            )
            .await
        }

        // =====================================================================
        // Grant Operations
        // =====================================================================
        (&Method::POST, "grants") => handle_create_grant(req, &state, &storage_path).await,

        (&Method::POST, p) if p.starts_with("grants/") && p.ends_with("/delegate") => {
            handle_delegate_grant(req, &state, &storage_path).await
        }

        (&Method::DELETE, p) if p.starts_with("grants/") => {
            forward_to_storage(req, &state, &storage_path).await
        }

        (&Method::GET, "subjects") => handle_get_grants_list(req, &state, &storage_path).await,

        (&Method::GET, "stewards") => handle_get_grants_list(req, &state, &storage_path).await,

        // =====================================================================
        // Policy Management
        // =====================================================================
        (&Method::POST, "policies") => handle_upsert_policy(req, &state, &storage_path).await,

        (&Method::GET, p) if p.starts_with("policies/") => {
            handle_get_policies_list(req, &state, &storage_path).await
        }

        // =====================================================================
        // Appeal Operations
        // =====================================================================
        (&Method::POST, "appeals") => handle_file_appeal(req, &state, &storage_path).await,

        (&Method::GET, "appeals") => handle_get_appeals_list(req, &state, &storage_path).await,

        // =====================================================================
        // Activity Logging
        // =====================================================================
        (&Method::POST, "activity") => handle_log_activity(req, &state, &storage_path).await,

        // =====================================================================
        // Fallback
        // =====================================================================
        _ => error_response(
            StatusCode::NOT_FOUND,
            &format!("Unknown stewardship route: {} {}", method, path),
            "NOT_FOUND",
        ),
    }
}

// =============================================================================
// Request Handlers
// =============================================================================

/// Handle POST /grants — validate and forward a create-grant request
async fn handle_create_grant(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };

    // Parse and validate
    let parsed: CreateGrantRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid create grant request: {e}"),
                "INVALID_BODY",
            )
        }
    };

    // subjectId and authorityBasis are required (enforced by Deserialize, but checked for clarity)
    if parsed.subject_id.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "subjectId is required",
            "MISSING_SUBJECT_ID",
        );
    }

    // Forward the validated (camelCase) body to storage
    forward_json_to_storage(
        state,
        storage_path,
        Method::POST,
        &serde_json::to_value(body_to_value(&body)).unwrap_or_default(),
    )
    .await
}

/// Handle POST /grants/{id}/delegate — validate and forward a delegate-grant request
async fn handle_delegate_grant(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };

    let parsed: DelegateGrantRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid delegate grant request: {e}"),
                "INVALID_BODY",
            )
        }
    };

    if parsed.parent_grant_id.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "parentGrantId is required",
            "MISSING_PARENT_GRANT_ID",
        );
    }
    if parsed.new_steward_id.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "newStewardId is required",
            "MISSING_NEW_STEWARD_ID",
        );
    }

    // Build the storage payload — storage expects `parent_grantId` / `new_stewardId`
    // (these are the field names the Angular fat-service was sending to the zome)
    let storage_payload = json!({
        "parent_grantId": parsed.parent_grant_id,
        "new_stewardId": parsed.new_steward_id,
        "contentFiltering": parsed.content_filtering,
        "timeLimits": parsed.time_limits,
        "featureRestrictions": parsed.feature_restrictions,
        "activityMonitoring": parsed.activity_monitoring,
        "policyDelegation": parsed.policy_delegation,
        "expiresInDays": parsed.expires_in_days,
    });

    forward_json_to_storage(state, storage_path, Method::POST, &storage_payload).await
}

/// Handle POST /policies — validate nested input and flatten for storage
async fn handle_upsert_policy(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };

    let parsed: UpsertPolicyRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid upsert policy request: {e}"),
                "INVALID_BODY",
            )
        }
    };

    let monitoring = parsed.monitoring_rules.unwrap_or(MonitoringRulesInput {
        log_sessions: false,
        log_categories: false,
        log_policy_events: true,
        retention_days: 30,
        subject_can_view: true,
    });

    // Flatten nested rules to the storage payload.
    // timeWindows are JSON-stringified as the zome expects timeWindowsJson.
    let time_windows_json =
        serde_json::to_string(&parsed.time_rules.time_windows).unwrap_or_else(|_| "[]".to_string());

    let storage_payload = json!({
        "subjectId": parsed.subject_id,
        "deviceId": parsed.device_id,
        // Content rules (flat)
        "blockedCategories": parsed.content_rules.blocked_categories,
        "blockedHashes": parsed.content_rules.blocked_hashes,
        "ageRatingMax": parsed.content_rules.age_rating_max,
        "reachLevelMax": parsed.content_rules.reach_level_max,
        // Time rules (flat)
        "sessionMaxMinutes": parsed.time_rules.session_max_minutes,
        "dailyMaxMinutes": parsed.time_rules.daily_max_minutes,
        "timeWindowsJson": time_windows_json,
        "cooldownMinutes": parsed.time_rules.cooldown_minutes,
        // Feature rules (flat)
        "disabledFeatures": parsed.feature_rules.disabled_features,
        "disabledRoutes": parsed.feature_rules.disabled_routes,
        "requireApproval": parsed.feature_rules.require_approval,
        // Monitoring
        "logSessions": monitoring.log_sessions,
        "logCategories": monitoring.log_categories,
        "logPolicyEvents": monitoring.log_policy_events,
        "retentionDays": monitoring.retention_days,
        "subjectCanView": monitoring.subject_can_view,
    });

    // Forward and transform the response
    let raw_response =
        forward_json_to_storage_raw(state, storage_path, Method::POST, &storage_payload).await;

    match raw_response {
        Ok((status, body_bytes)) => {
            if !status.is_success() {
                return Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap();
            }
            match serde_json::from_slice::<Value>(&body_bytes) {
                Ok(raw_val) => {
                    // Response may be wrapped {actionHash, policy: {...}} or bare
                    let transformed = if let Some(policy) = raw_val.get("policy") {
                        json!({
                            "actionHash": raw_val.get("actionHash"),
                            "policy": transform_policy(policy),
                        })
                    } else {
                        transform_policy(&raw_val)
                    };
                    json_response(status, &transformed)
                }
                Err(_) => Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap(),
            }
        }
        Err(resp) => resp,
    }
}

/// Handle POST /appeals — validate and forward a file-appeal request
async fn handle_file_appeal(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };

    let parsed: FileAppealRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid file appeal request: {e}"),
                "INVALID_BODY",
            )
        }
    };

    if parsed.grant_id.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "grantId is required",
            "MISSING_GRANT_ID",
        );
    }
    if parsed.grounds.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "grounds is required and must not be empty",
            "MISSING_GROUNDS",
        );
    }

    let storage_payload = json!({
        "grantId": parsed.grant_id,
        "policyId": parsed.policy_id,
        "appealType": parsed.appeal_type,
        "grounds": parsed.grounds,
        "evidenceJson": parsed.evidence_json,
        "advocateId": parsed.advocate_id,
    });

    let raw_response =
        forward_json_to_storage_raw(state, storage_path, Method::POST, &storage_payload).await;

    match raw_response {
        Ok((status, body_bytes)) => {
            if !status.is_success() {
                return Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap();
            }
            match serde_json::from_slice::<Value>(&body_bytes) {
                Ok(raw_val) => {
                    let transformed = if let Some(appeal) = raw_val.get("appeal") {
                        json!({
                            "actionHash": raw_val.get("actionHash"),
                            "appeal": transform_appeal(appeal),
                        })
                    } else {
                        transform_appeal(&raw_val)
                    };
                    json_response(status, &transformed)
                }
                Err(_) => Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap(),
            }
        }
        Err(resp) => resp,
    }
}

/// Handle POST /activity — validate and forward an activity log request
async fn handle_log_activity(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };

    let parsed: LogActivityRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid log activity request: {e}"),
                "INVALID_BODY",
            )
        }
    };

    // Stringify policyEvents for zome
    let policy_events_json =
        serde_json::to_string(&parsed.policy_events).unwrap_or_else(|_| "[]".to_string());

    let storage_payload = json!({
        "sessionId": parsed.session_id,
        "sessionDurationMinutes": parsed.session_duration_minutes,
        "categories_accessed": parsed.categories_accessed,
        "policyEventsJson": policy_events_json,
    });

    let raw_response =
        forward_json_to_storage_raw(state, storage_path, Method::POST, &storage_payload).await;

    match raw_response {
        Ok((status, body_bytes)) => {
            if !status.is_success() {
                return Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap();
            }
            match serde_json::from_slice::<Value>(&body_bytes) {
                Ok(raw_val) => {
                    let transformed = if let Some(log) = raw_val.get("log") {
                        json!({
                            "actionHash": raw_val.get("actionHash"),
                            "log": transform_activity_log(log),
                        })
                    } else {
                        transform_activity_log(&raw_val)
                    };
                    json_response(status, &transformed)
                }
                Err(_) => Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap(),
            }
        }
        Err(resp) => resp,
    }
}

// =============================================================================
// GET Response Transformers
// =============================================================================

/// Transform a computed policy response (GET /policy)
fn transform_computed_policy(raw: &Value) -> Value {
    // ComputedPolicy comes back fairly clean (arrays, not json strings)
    // but null → absent for optional fields
    let obj = match raw.as_object() {
        Some(o) => o,
        None => return raw.clone(),
    };

    let mut out = raw.clone();
    if let Some(out_obj) = out.as_object_mut() {
        for key in &[
            "ageRatingMax",
            "reachLevelMax",
            "sessionMaxMinutes",
            "dailyMaxMinutes",
            "cooldownMinutes",
        ] {
            if out_obj.get(*key) == Some(&Value::Null) {
                out_obj.remove(*key);
            }
        }
        // timeWindowsJson may be a string — parse it
        if let Some(tw_json) = obj.get("timeWindowsJson").and_then(|v| v.as_str()) {
            if let Ok(parsed) = serde_json::from_str::<Value>(tw_json) {
                out_obj.insert("timeWindows".to_string(), parsed);
            }
        }
    }

    out
}

/// Transform a policy decision response (GET /access/*)
fn transform_policy_decision_response(raw: &Value) -> Value {
    transform_policy_decision(raw)
}

/// Handle GET requests that return a list of grants, transforming each entry
async fn handle_get_grants_list(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let raw_response = forward_get_raw(req, state, storage_path).await;

    match raw_response {
        Ok((status, body_bytes)) => {
            if !status.is_success() {
                return Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap();
            }
            match serde_json::from_slice::<Value>(&body_bytes) {
                Ok(raw_val) => {
                    // May be array of {actionHash, grant} or bare grants
                    let transformed = transform_grant_list(&raw_val);
                    json_response(status, &transformed)
                }
                Err(_) => Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap(),
            }
        }
        Err(resp) => resp,
    }
}

/// Handle GET requests that return a list of policies
async fn handle_get_policies_list(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let raw_response = forward_get_raw(req, state, storage_path).await;

    match raw_response {
        Ok((status, body_bytes)) => {
            if !status.is_success() {
                return Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap();
            }
            match serde_json::from_slice::<Value>(&body_bytes) {
                Ok(raw_val) => {
                    let transformed = transform_policy_list(&raw_val);
                    json_response(status, &transformed)
                }
                Err(_) => Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap(),
            }
        }
        Err(resp) => resp,
    }
}

/// Handle GET requests that return a list of appeals
async fn handle_get_appeals_list(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let raw_response = forward_get_raw(req, state, storage_path).await;

    match raw_response {
        Ok((status, body_bytes)) => {
            if !status.is_success() {
                return Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap();
            }
            match serde_json::from_slice::<Value>(&body_bytes) {
                Ok(raw_val) => {
                    let transformed = transform_appeal_list(&raw_val);
                    json_response(status, &transformed)
                }
                Err(_) => Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap(),
            }
        }
        Err(resp) => resp,
    }
}

// =============================================================================
// List Transformers
// =============================================================================

fn transform_grant_list(raw: &Value) -> Value {
    match raw {
        Value::Array(items) => {
            let transformed: Vec<Value> = items
                .iter()
                .map(|item| {
                    // May be {actionHash, grant: {...}} wrapper
                    if let Some(grant) = item.get("grant") {
                        json!({
                            "actionHash": item.get("actionHash"),
                            "grant": transform_grant(grant),
                        })
                    } else {
                        transform_grant(item)
                    }
                })
                .collect();
            Value::Array(transformed)
        }
        _ => raw.clone(),
    }
}

fn transform_policy_list(raw: &Value) -> Value {
    match raw {
        Value::Array(items) => {
            let transformed: Vec<Value> = items
                .iter()
                .map(|item| {
                    if let Some(policy) = item.get("policy") {
                        json!({
                            "actionHash": item.get("actionHash"),
                            "policy": transform_policy(policy),
                        })
                    } else {
                        transform_policy(item)
                    }
                })
                .collect();
            Value::Array(transformed)
        }
        _ => raw.clone(),
    }
}

fn transform_appeal_list(raw: &Value) -> Value {
    match raw {
        Value::Array(items) => {
            let transformed: Vec<Value> = items
                .iter()
                .map(|item| {
                    if let Some(appeal) = item.get("appeal") {
                        json!({
                            "actionHash": item.get("actionHash"),
                            "appeal": transform_appeal(appeal),
                        })
                    } else {
                        transform_appeal(item)
                    }
                })
                .collect();
            Value::Array(transformed)
        }
        _ => raw.clone(),
    }
}

// =============================================================================
// Storage Proxy Utilities
// =============================================================================

/// Read and return the body bytes from an incoming request
async fn read_body(req: Request<Incoming>) -> Result<Vec<u8>, Response<Full<Bytes>>> {
    req.collect()
        .await
        .map(|c| c.to_bytes().to_vec())
        .map_err(|e| {
            warn!(error = %e, "Failed to read stewardship request body");
            error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read request body: {e}"),
                "BODY_READ_ERROR",
            )
        })
}

/// Attempt to parse body bytes as JSON Value (fallback: return null)
fn body_to_value(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or(Value::Null)
}

/// Handle a GET request, apply a transform function to the response body, return transformed response
async fn handle_get_with_transform<F>(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
    transform: F,
) -> Response<Full<Bytes>>
where
    F: Fn(&Value) -> Value,
{
    let raw_response = forward_get_raw(req, state, storage_path).await;

    match raw_response {
        Ok((status, body_bytes)) => {
            if !status.is_success() {
                return Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap();
            }
            match serde_json::from_slice::<Value>(&body_bytes) {
                Ok(raw_val) => {
                    let transformed = transform(&raw_val);
                    json_response(status, &transformed)
                }
                Err(_) => Response::builder()
                    .status(status)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(body_bytes)))
                    .unwrap(),
            }
        }
        Err(resp) => resp,
    }
}

/// Forward a GET request to storage, return raw (status, bytes)
async fn forward_get_raw(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Result<(StatusCode, Vec<u8>), Response<Full<Bytes>>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url,
        None => {
            warn!("Stewardship proxy called but STORAGE_URL not configured");
            return Err(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error":"Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap());
        }
    };

    let query = req.uri().query();
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), storage_path);
    let full_url = if let Some(q) = query {
        format!("{storage_endpoint}?{q}")
    } else {
        storage_endpoint
    };

    debug!(url = %full_url, "GET stewardship from elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = client.get(&full_url);

    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    match builder.send().await {
        Ok(response) => {
            let status = StatusCode::from_u16(response.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            match response.bytes().await {
                Ok(body) => Ok((status, body.to_vec())),
                Err(e) => {
                    warn!(error = %e, "Failed to read stewardship storage response body");
                    Err(error_response(
                        StatusCode::BAD_GATEWAY,
                        &format!("Failed to read storage response: {e}"),
                        "BAD_GATEWAY",
                    ))
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %full_url, "Failed to forward stewardship GET to storage");
            Err(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to connect to storage: {e}"),
                "BAD_GATEWAY",
            ))
        }
    }
}

/// Forward a JSON POST payload to storage, return raw (status, bytes)
async fn forward_json_to_storage_raw(
    state: &AppState,
    storage_path: &str,
    method: Method,
    payload: &Value,
) -> Result<(StatusCode, Vec<u8>), Response<Full<Bytes>>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url,
        None => {
            warn!("Stewardship proxy called but STORAGE_URL not configured");
            return Err(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error":"Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap());
        }
    };

    let full_url = format!("{}{}", storage_url.trim_end_matches('/'), storage_path);
    debug!(method = %method, url = %full_url, "Forwarding stewardship POST to elohim-storage");

    let body_bytes = serde_json::to_vec(payload).unwrap_or_default();

    let client = reqwest::Client::new();
    let builder = match method {
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        _ => client.post(&full_url),
    };

    match builder
        .header("Content-Type", "application/json")
        .body(body_bytes)
        .send()
        .await
    {
        Ok(response) => {
            let status = StatusCode::from_u16(response.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            match response.bytes().await {
                Ok(body) => {
                    info!(status = %status, path = %storage_path, "Forwarded stewardship POST response");
                    Ok((status, body.to_vec()))
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read stewardship storage POST response body");
                    Err(error_response(
                        StatusCode::BAD_GATEWAY,
                        &format!("Failed to read storage response: {e}"),
                        "BAD_GATEWAY",
                    ))
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %full_url, "Failed to forward stewardship POST to storage");
            Err(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to connect to storage: {e}"),
                "BAD_GATEWAY",
            ))
        }
    }
}

/// Forward a JSON payload to storage and return the raw HTTP response (no transformation)
async fn forward_json_to_storage(
    state: &AppState,
    storage_path: &str,
    method: Method,
    payload: &Value,
) -> Response<Full<Bytes>> {
    match forward_json_to_storage_raw(state, storage_path, method, payload).await {
        Ok((status, body_bytes)) => Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .header("Cross-Origin-Resource-Policy", "cross-origin")
            .body(Full::new(Bytes::from(body_bytes)))
            .unwrap(),
        Err(resp) => resp,
    }
}

/// Forward a stewardship request to elohim-storage (transparent proxy, no transformation).
///
/// Maps `/api/v1/stewardship/...` → `/db/stewardship/...` on the storage service.
async fn forward_to_storage(
    req: Request<Incoming>,
    state: &AppState,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url,
        None => {
            warn!("Stewardship proxy called but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error":"Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap();
        }
    };

    // Preserve query string
    let query = req.uri().query();
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), storage_path);
    let full_url = if let Some(q) = query {
        format!("{storage_endpoint}?{q}")
    } else {
        storage_endpoint
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding stewardship request to elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Method not allowed"}"#)))
                .unwrap();
        }
    };

    // Forward content-type header if present
    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    // Forward authorization header if present
    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    // Forward body for POST/PUT
    if matches!(method, Method::POST | Method::PUT) {
        match req.collect().await {
            Ok(collected) => {
                builder = builder.body(collected.to_bytes().to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read stewardship request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Failed to read request body: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => {
                    info!(
                        status = %status,
                        size = body.len(),
                        path = %storage_path,
                        "Forwarded stewardship response"
                    );
                    Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin")
                        .body(Full::new(Bytes::from(body.to_vec())))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read stewardship storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error":"Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %full_url, "Failed to forward stewardship request to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error":"Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_computed_policy() {
        let policy = ComputedPolicy {
            subject_id: "agent-123".to_string(),
            computed_at: "2026-03-05T00:00:00Z".to_string(),
            blocked_categories: vec!["violence".to_string()],
            blocked_hashes: vec![],
            age_rating_max: Some("PG-13".to_string()),
            reach_level_max: None,
            session_max_minutes: Some(120),
            daily_max_minutes: Some(480),
            time_windows_json: None,
            cooldown_minutes: None,
            disabled_features: vec!["direct_message".to_string()],
            disabled_routes: vec![],
            require_approval: vec![],
            log_sessions: false,
            log_categories: false,
            log_policy_events: true,
            retention_days: 30,
            subject_can_view: true,
        };

        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("subjectId"));
        assert!(json.contains("blockedCategories"));
        assert!(json.contains("sessionMaxMinutes"));
        // Verify camelCase output
        assert!(!json.contains("subject_id"));
        assert!(!json.contains("blocked_categories"));
    }

    #[test]
    fn test_serialize_stewardship_grant() {
        let grant = StewardshipGrant {
            id: "grant-1".to_string(),
            steward_id: "steward-1".to_string(),
            subject_id: "subject-1".to_string(),
            tier: StewardCapabilityTier::Guardian,
            authority_basis: AuthorityBasis::MinorGuardianship,
            evidence_hash: Some("sha256-abc".to_string()),
            verified_by: "verifier-1".to_string(),
            content_filtering: true,
            time_limits: true,
            feature_restrictions: false,
            activity_monitoring: false,
            policy_delegation: false,
            delegatable: true,
            delegated_from: None,
            delegation_depth: 0,
            granted_at: "2026-03-01T00:00:00Z".to_string(),
            expires_at: "2027-03-01T00:00:00Z".to_string(),
            review_at: "2026-09-01T00:00:00Z".to_string(),
            status: GrantStatus::Active,
            appeal_id: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&grant).unwrap();
        assert!(json.contains("stewardId"));
        assert!(json.contains("authorityBasis"));
        assert!(json.contains("minor_guardianship"));
        // Optional None fields should be absent
        assert!(!json.contains("delegatedFrom"));
        assert!(!json.contains("appealId"));
    }

    #[test]
    fn test_deserialize_create_grant_request() {
        let json = r#"{
            "subjectId": "subject-1",
            "authorityBasis": "mutual_consent",
            "verifiedBy": "verifier-1",
            "contentFiltering": true,
            "timeLimits": false,
            "featureRestrictions": false,
            "activityMonitoring": false,
            "policyDelegation": false,
            "delegatable": false,
            "expiresInDays": 365,
            "reviewInDays": 90
        }"#;

        let req: CreateGrantRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.subject_id, "subject-1");
        assert_eq!(req.authority_basis, AuthorityBasis::MutualConsent);
        assert!(req.content_filtering);
        assert!(!req.time_limits);
        assert_eq!(req.expires_in_days, 365);
        assert!(req.evidence_hash.is_none());
    }

    #[test]
    fn test_deserialize_delegate_grant_request() {
        let json = r#"{
            "parentGrantId": "grant-1",
            "newStewardId": "steward-2",
            "contentFiltering": true,
            "expiresInDays": 180
        }"#;

        let req: DelegateGrantRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.parent_grant_id, "grant-1");
        assert_eq!(req.new_steward_id, "steward-2");
        assert_eq!(req.content_filtering, Some(true));
        assert!(req.time_limits.is_none());
    }

    #[test]
    fn test_deserialize_file_appeal_request() {
        let json = r#"{
            "grantId": "grant-1",
            "appealType": "excessive",
            "grounds": ["Restrictions prevent learning", "No evidence of harm"],
            "evidenceJson": "{\"details\": \"test\"}",
            "advocateId": "advocate-1"
        }"#;

        let req: FileAppealRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.grant_id, "grant-1");
        assert_eq!(req.appeal_type, AppealType::Excessive);
        assert_eq!(req.grounds.len(), 2);
        assert_eq!(req.advocate_id, Some("advocate-1".to_string()));
        assert!(req.policy_id.is_none());
    }

    #[test]
    fn test_serialize_access_check_response_allowed() {
        let resp = AccessCheckResponse {
            allowed: true,
            reason: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""allowed":true"#));
        assert!(!json.contains("reason"));
    }

    #[test]
    fn test_serialize_access_check_response_blocked() {
        let resp = AccessCheckResponse {
            allowed: false,
            reason: Some("Content blocked by guardian policy".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""allowed":false"#));
        assert!(json.contains("Content blocked by guardian policy"));
    }

    #[test]
    fn test_serialize_stewardship_appeal() {
        let appeal = StewardshipAppeal {
            id: "appeal-1".to_string(),
            appellant_id: "user-1".to_string(),
            grant_id: "grant-1".to_string(),
            policy_id: None,
            appeal_type: AppealType::Scope,
            grounds: vec!["Overly broad".to_string()],
            evidence_json: "{}".to_string(),
            advocate_id: None,
            advocate_notes: None,
            arbitration_layer: "community".to_string(),
            assigned_to: None,
            status: AppealStatus::Filed,
            status_changed_at: None,
            decision: None,
            filed_at: "2026-03-05T00:00:00Z".to_string(),
            expires_at: "2026-04-05T00:00:00Z".to_string(),
            created_at: "2026-03-05T00:00:00Z".to_string(),
            updated_at: "2026-03-05T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&appeal).unwrap();
        assert!(json.contains("appellantId"));
        assert!(json.contains("arbitrationLayer"));
        // Optional None fields should be absent
        assert!(!json.contains("policyId"));
        assert!(!json.contains("advocateId"));
        assert!(!json.contains("decision"));
    }

    #[test]
    fn test_capability_tier_serde_roundtrip() {
        let tiers = vec![
            StewardCapabilityTier::SelfTier,
            StewardCapabilityTier::Guide,
            StewardCapabilityTier::Guardian,
            StewardCapabilityTier::Coordinator,
            StewardCapabilityTier::Constitutional,
        ];

        for tier in &tiers {
            let json = serde_json::to_string(tier).unwrap();
            let deserialized: StewardCapabilityTier = serde_json::from_str(&json).unwrap();
            assert_eq!(tier, &deserialized);
        }

        // Verify "self" is the serialized form (not "selfTier")
        let self_json = serde_json::to_string(&StewardCapabilityTier::SelfTier).unwrap();
        assert_eq!(self_json, r#""self""#);
    }

    #[test]
    fn test_authority_basis_serde_roundtrip() {
        let bases = vec![
            AuthorityBasis::MinorGuardianship,
            AuthorityBasis::CourtOrder,
            AuthorityBasis::MedicalNecessity,
            AuthorityBasis::CommunityCensus,
            AuthorityBasis::OrganizationalRole,
            AuthorityBasis::MutualConsent,
        ];

        for basis in &bases {
            let json = serde_json::to_string(basis).unwrap();
            let deserialized: AuthorityBasis = serde_json::from_str(&json).unwrap();
            assert_eq!(basis, &deserialized);
        }

        // Verify snake_case serialization matches TypeScript
        let json = serde_json::to_string(&AuthorityBasis::MinorGuardianship).unwrap();
        assert_eq!(json, r#""minor_guardianship""#);
    }

    #[test]
    fn test_transform_policy_decision_allow() {
        // Rust enum external tag format
        let raw = json!({"Allow": null});
        let result = transform_policy_decision(&raw);
        assert_eq!(result["type"], "allow");
        assert!(result.get("reason").is_none());
    }

    #[test]
    fn test_transform_policy_decision_block() {
        let raw = json!({"Block": {"reason": "Category 'violence' blocked"}});
        let result = transform_policy_decision(&raw);
        assert_eq!(result["type"], "block");
        assert_eq!(result["reason"], "Category 'violence' blocked");
    }

    #[test]
    fn test_transform_policy_decision_already_flat() {
        // Already in flat format from storage
        let raw = json!({"type": "allow"});
        let result = transform_policy_decision(&raw);
        assert_eq!(result["type"], "allow");
    }

    #[test]
    fn test_transform_policy_parses_json_fields() {
        let raw = json!({
            "id": "policy-1",
            "subjectId": "subject-1",
            "authorId": "author-1",
            "authorTier": "guardian",
            "blockedCategoriesJson": "[\"violence\",\"adult\"]",
            "blockedHashesJson": "[]",
            "ageRatingMax": null,
            "reachLevelMax": null,
            "sessionMaxMinutes": 120,
            "dailyMaxMinutes": null,
            "timeWindowsJson": "[{\"daysOfWeek\":[1,2,3],\"startHour\":8,\"startMinute\":0,\"endHour\":18,\"endMinute\":0}]",
            "cooldownMinutes": null,
            "disabledFeaturesJson": "[\"direct_message\"]",
            "disabledRoutesJson": "[]",
            "requireApprovalJson": "[]",
            "logSessions": false,
            "logCategories": false,
            "logPolicyEvents": true,
            "retentionDays": 30,
            "subjectCanView": true,
            "effectiveFrom": "2026-01-01T00:00:00Z",
            "effectiveUntil": null,
            "version": 1,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z"
        });

        let result = transform_policy(&raw);

        // Should have parsed arrays instead of JSON strings
        assert_eq!(result["contentRules"]["blockedCategories"][0], "violence");
        assert_eq!(result["contentRules"]["blockedCategories"][1], "adult");
        assert_eq!(
            result["featureRules"]["disabledFeatures"][0],
            "direct_message"
        );
        assert!(result["timeRules"]["timeWindows"].is_array());

        // Flat aliases should also be present
        assert_eq!(result["blockedCategories"][0], "violence");
        assert_eq!(result["disabledFeatures"][0], "direct_message");

        // sessionMaxMinutes scalar preserved
        assert_eq!(result["sessionMaxMinutes"], 120);

        // null optional fields should be absent
        assert!(
            result.get("ageRatingMax").is_none()
                || result["ageRatingMax"].is_null() == false
                || result.get("ageRatingMax") == Some(&Value::Null)
        );
        assert!(result.get("effectiveUntil").is_none());
    }

    #[test]
    fn test_transform_appeal_parses_json_fields() {
        let raw = json!({
            "id": "appeal-1",
            "appellantId": "user-1",
            "grantId": "grant-1",
            "policyId": null,
            "appealType": "excessive",
            "groundsJson": "[\"Restrictions prevent learning\",\"No evidence of harm\"]",
            "evidenceJson": "{}",
            "advocateId": null,
            "advocateNotes": null,
            "arbitrationLayer": "community",
            "assignedTo": null,
            "status": "filed",
            "statusChangedAt": null,
            "decisionJson": null,
            "decisionMadeBy": null,
            "decisionMadeAt": null,
            "filedAt": "2026-03-05T00:00:00Z",
            "expiresAt": "2026-04-05T00:00:00Z",
            "createdAt": "2026-03-05T00:00:00Z",
            "updatedAt": "2026-03-05T00:00:00Z"
        });

        let result = transform_appeal(&raw);

        // groundsJson should be parsed into grounds array
        assert!(result.get("groundsJson").is_none());
        assert_eq!(result["grounds"][0], "Restrictions prevent learning");
        assert_eq!(result["grounds"][1], "No evidence of harm");

        // decisionJson null → no decision field
        assert!(result.get("decisionJson").is_none());
        assert!(result.get("decision").is_none());

        // null optional fields removed
        assert!(result.get("policyId").is_none());
        assert!(result.get("advocateId").is_none());
    }

    #[test]
    fn test_transform_appeal_with_decision() {
        let raw = json!({
            "id": "appeal-1",
            "appellantId": "user-1",
            "grantId": "grant-1",
            "appealType": "scope",
            "groundsJson": "[\"Too broad\"]",
            "evidenceJson": "{}",
            "arbitrationLayer": "community",
            "status": "decided",
            "decisionJson": "{\"approved\":true,\"notes\":\"Granted\",\"decidedBy\":\"arbiter-1\",\"decidedAt\":\"2026-03-10T00:00:00Z\"}",
            "filedAt": "2026-03-05T00:00:00Z",
            "expiresAt": "2026-04-05T00:00:00Z",
            "createdAt": "2026-03-05T00:00:00Z",
            "updatedAt": "2026-03-10T00:00:00Z"
        });

        let result = transform_appeal(&raw);

        assert!(result.get("decisionJson").is_none());
        assert!(result["decision"].is_object());
        assert_eq!(result["decision"]["approved"], true);
        assert_eq!(result["decision"]["notes"], "Granted");
    }

    #[test]
    fn test_transform_activity_log() {
        let raw = json!({
            "id": "log-1",
            "subjectId": "user-1",
            "deviceId": null,
            "sessionId": "session-1",
            "sessionStartedAt": "2026-03-05T08:00:00Z",
            "sessionDurationMinutes": 45,
            "categoriesAccessedJson": "[\"science\",\"mathematics\"]",
            "policyEventsJson": "[{\"eventType\":\"blocked_content\",\"details\":\"Hash blocked\"}]",
            "loggedAt": "2026-03-05T08:45:00Z",
            "retentionExpiresAt": "2026-06-05T08:45:00Z"
        });

        let result = transform_activity_log(&raw);

        assert!(result.get("categoriesAccessedJson").is_none());
        assert!(result.get("policyEventsJson").is_none());
        assert_eq!(result["categoriesAccessed"][0], "science");
        assert_eq!(result["categoriesAccessed"][1], "mathematics");
        assert!(result["policyEvents"].is_array());
        assert_eq!(result["policyEvents"][0]["eventType"], "blocked_content");

        // null deviceId removed
        assert!(result.get("deviceId").is_none());
    }

    #[test]
    fn test_transform_grant_removes_null_optionals() {
        let raw = json!({
            "id": "grant-1",
            "stewardId": "steward-1",
            "subjectId": "subject-1",
            "tier": "guardian",
            "authorityBasis": "minor_guardianship",
            "evidenceHash": null,
            "verifiedBy": "verifier-1",
            "contentFiltering": true,
            "timeLimits": false,
            "featureRestrictions": false,
            "activityMonitoring": false,
            "policyDelegation": false,
            "delegatable": true,
            "delegatedFrom": null,
            "delegationDepth": 0,
            "grantedAt": "2026-01-01T00:00:00Z",
            "expiresAt": "2027-01-01T00:00:00Z",
            "reviewAt": "2026-07-01T00:00:00Z",
            "status": "active",
            "appealId": null,
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z"
        });

        let result = transform_grant(&raw);

        assert!(result.get("evidenceHash").is_none());
        assert!(result.get("delegatedFrom").is_none());
        assert!(result.get("appealId").is_none());
        // Non-null fields preserved
        assert_eq!(result["id"], "grant-1");
        assert_eq!(result["tier"], "guardian");
    }

    #[test]
    fn test_deserialize_upsert_policy_request() {
        let json = r#"{
            "subjectId": "subject-1",
            "contentRules": {
                "blockedCategories": ["violence"],
                "blockedHashes": [],
                "ageRatingMax": "PG-13"
            },
            "timeRules": {
                "sessionMaxMinutes": 120,
                "timeWindows": []
            },
            "featureRules": {
                "disabledFeatures": ["direct_message"],
                "disabledRoutes": [],
                "requireApproval": []
            }
        }"#;

        let req: UpsertPolicyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.subject_id, Some("subject-1".to_string()));
        assert_eq!(req.content_rules.blocked_categories, vec!["violence"]);
        assert_eq!(req.content_rules.age_rating_max, Some("PG-13".to_string()));
        assert_eq!(req.time_rules.session_max_minutes, Some(120));
        assert_eq!(req.feature_rules.disabled_features, vec!["direct_message"]);
        // monitoring defaults
        assert!(req.monitoring_rules.is_none());
    }

    #[test]
    fn test_deserialize_log_activity_request() {
        let json = r#"{
            "sessionId": "session-1",
            "sessionDurationMinutes": 45,
            "categoriesAccessed": ["science"],
            "policyEvents": [{"eventType": "blocked_content", "details": "Hash blocked"}]
        }"#;

        let req: LogActivityRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.session_id, "session-1");
        assert_eq!(req.session_duration_minutes, 45);
        assert_eq!(req.categories_accessed, vec!["science"]);
        assert_eq!(req.policy_events.len(), 1);
    }

    #[test]
    fn test_grant_list_transformer() {
        let raw = json!([
            {
                "actionHash": [1, 2, 3],
                "grant": {
                    "id": "grant-1",
                    "evidenceHash": null,
                    "delegatedFrom": null,
                    "appealId": null
                }
            }
        ]);

        let result = transform_grant_list(&raw);
        let item = &result[0];
        assert!(item.get("actionHash").is_some());
        let grant = &item["grant"];
        assert!(grant.get("evidenceHash").is_none());
    }
}
