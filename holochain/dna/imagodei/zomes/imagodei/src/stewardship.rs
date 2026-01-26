//! Stewardship Coordinator Functions
//!
//! Functions for managing graduated stewardship capabilities:
//! - Stewardship grants (authority to manage another's capabilities)
//! - Device policies (content filtering, time limits, feature restrictions)
//! - Policy inheritance (composing policies across layers)
//! - Appeals (challenging grants or policies)
//! - Activity logging (when monitoring is enabled)
//!
//! Key principle: This is ImagoDei - about identity and self-knowledge.
//! Everyone has limits. This helps people explore and manage their own
//! constraints through trusted relationships.

use hdk::prelude::*;
use imagodei_integrity::*;

// =============================================================================
// Input/Output Types
// =============================================================================

/// Output from stewardship grant operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StewardshipGrantOutput {
    pub action_hash: ActionHash,
    pub grant: StewardshipGrant,
}

/// Output from device policy operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePolicyOutput {
    pub action_hash: ActionHash,
    pub policy: DevicePolicy,
}

/// Output from policy inheritance operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInheritanceOutput {
    pub action_hash: ActionHash,
    pub inheritance: PolicyInheritance,
}

/// Output from appeal operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StewardshipAppealOutput {
    pub action_hash: ActionHash,
    pub appeal: StewardshipAppeal,
}

/// Output from activity log operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogOutput {
    pub action_hash: ActionHash,
    pub log: ActivityLog,
}

/// Input for creating a stewardship grant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGrantInput {
    pub subject_id: String,
    pub authority_basis: String,
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

/// Input for delegating a grant to another steward
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateGrantInput {
    pub parent_grant_id: String,
    pub new_steward_id: String,
    // Can optionally restrict capabilities
    pub content_filtering: Option<bool>,
    pub time_limits: Option<bool>,
    pub feature_restrictions: Option<bool>,
    pub activity_monitoring: Option<bool>,
    pub policy_delegation: Option<bool>,
    pub expires_in_days: u32,
}

/// Input for creating/updating a device policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertPolicyInput {
    pub subject_id: String,
    pub device_id: Option<String>,
    // Content rules
    pub blocked_categories: Vec<String>,
    pub blocked_hashes: Vec<String>,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<u8>,
    // Time rules
    pub session_max_minutes: Option<u32>,
    pub daily_max_minutes: Option<u32>,
    pub time_windows_json: String,
    pub cooldown_minutes: Option<u32>,
    // Feature rules
    pub disabled_features: Vec<String>,
    pub disabled_routes: Vec<String>,
    pub require_approval: Vec<String>,
    // Monitoring rules
    pub log_sessions: bool,
    pub log_categories: bool,
    pub log_policy_events: bool,
    pub retention_days: u32,
    pub subject_can_view: bool,
}

/// Input for filing an appeal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAppealInput {
    pub grant_id: String,
    pub policy_id: Option<String>,
    pub appeal_type: String,
    pub grounds: Vec<String>,
    pub evidence_json: String,
    pub advocate_id: Option<String>,
}

/// Input for deciding an appeal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecideAppealInput {
    pub appeal_id: String,
    pub approved: bool,
    pub decision_notes: String,
    pub modifications_json: Option<String>,
}

/// Input for logging activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogActivityInput {
    pub session_id: String,
    pub session_duration_minutes: u32,
    pub categories_accessed: Vec<String>,
    pub policy_events_json: String,
}

/// Computed policy for a subject (merged from inheritance chain)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedPolicy {
    pub subject_id: String,
    pub computed_at: String,
    // Merged content rules
    pub blocked_categories: Vec<String>,
    pub blocked_hashes: Vec<String>,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<u8>,
    // Merged time rules
    pub session_max_minutes: Option<u32>,
    pub daily_max_minutes: Option<u32>,
    pub time_windows_json: String,
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

/// Policy decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow,
    Block { reason: String },
}

/// Content check input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentCheckInput {
    pub content_hash: String,
    pub categories: Vec<String>,
    pub age_rating: Option<String>,
    pub reach_level: Option<u8>,
}

// =============================================================================
// Stewardship Signals
// =============================================================================

/// Signals for real-time notification of stewardship events
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum StewardshipSignal {
    GrantCreated {
        action_hash: ActionHash,
        grant: StewardshipGrant,
    },
    GrantRevoked {
        grant_id: String,
        reason: String,
    },
    PolicyUpdated {
        action_hash: ActionHash,
        policy: DevicePolicy,
    },
    AppealFiled {
        action_hash: ActionHash,
        appeal: StewardshipAppeal,
    },
    AppealDecided {
        appeal_id: String,
        approved: bool,
    },
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get steward tier for the calling agent based on their grants
fn get_my_steward_tier(subject_id: &str) -> ExternResult<String> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    // Check if I'm the subject (self tier)
    if my_human.human.id == subject_id {
        return Ok("self".to_string());
    }

    // Check active grants where I am steward
    let grants = get_grants_as_steward(my_human.human.id.clone())?;

    // Find highest tier grant for this subject
    for grant in grants {
        if grant.grant.subject_id == subject_id && grant.grant.status == "active" {
            // Check for coordinator attestation (placeholder for future attestation check)
            if grant.grant.policy_delegation {
                return Ok("coordinator".to_string());
            }
            return Ok("guardian".to_string());
        }
    }

    // Check if I have guide attestation (advisory only)
    // For now, default to guide if no active grant
    Ok("guide".to_string())
}

/// Merge two policies (child adds restrictions to parent)
fn merge_policies(parent: &DevicePolicy, child: &DevicePolicy) -> DevicePolicy {
    // Parse JSON arrays
    let mut blocked_categories: Vec<String> = serde_json::from_str(&parent.blocked_categories_json)
        .unwrap_or_default();
    let child_categories: Vec<String> = serde_json::from_str(&child.blocked_categories_json)
        .unwrap_or_default();
    for cat in child_categories {
        if !blocked_categories.contains(&cat) {
            blocked_categories.push(cat);
        }
    }

    let mut blocked_hashes: Vec<String> = serde_json::from_str(&parent.blocked_hashes_json)
        .unwrap_or_default();
    let child_hashes: Vec<String> = serde_json::from_str(&child.blocked_hashes_json)
        .unwrap_or_default();
    for hash in child_hashes {
        if !blocked_hashes.contains(&hash) {
            blocked_hashes.push(hash);
        }
    }

    let mut disabled_features: Vec<String> = serde_json::from_str(&child.disabled_features_json)
        .unwrap_or_default();
    let parent_features: Vec<String> = serde_json::from_str(&parent.disabled_features_json)
        .unwrap_or_default();
    for feat in parent_features {
        if !disabled_features.contains(&feat) {
            disabled_features.push(feat);
        }
    }

    // Take more restrictive time limits
    let session_max = match (parent.session_max_minutes, child.session_max_minutes) {
        (Some(p), Some(c)) => Some(p.min(c)),
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };

    let daily_max = match (parent.daily_max_minutes, child.daily_max_minutes) {
        (Some(p), Some(c)) => Some(p.min(c)),
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };

    // Take more restrictive age rating (lower index = more restrictive)
    let age_rating_order = ["G", "PG", "PG-13", "R", "NC-17"];
    let age_rating = match (&parent.age_rating_max, &child.age_rating_max) {
        (Some(p), Some(c)) => {
            let p_idx = age_rating_order.iter().position(|&r| r == p).unwrap_or(4);
            let c_idx = age_rating_order.iter().position(|&r| r == c).unwrap_or(4);
            Some(age_rating_order[p_idx.min(c_idx)].to_string())
        }
        (Some(p), None) => Some(p.clone()),
        (None, Some(c)) => Some(c.clone()),
        (None, None) => None,
    };

    DevicePolicy {
        id: child.id.clone(),
        subject_id: child.subject_id.clone(),
        device_id: child.device_id.clone(),
        author_id: child.author_id.clone(),
        author_tier: child.author_tier.clone(),
        inherits_from: Some(parent.id.clone()),
        blocked_categories_json: serde_json::to_string(&blocked_categories).unwrap_or_default(),
        blocked_hashes_json: serde_json::to_string(&blocked_hashes).unwrap_or_default(),
        age_rating_max: age_rating,
        reach_level_max: parent.reach_level_max.or(child.reach_level_max),
        session_max_minutes: session_max,
        daily_max_minutes: daily_max,
        time_windows_json: if child.time_windows_json.is_empty() || child.time_windows_json == "[]" {
            parent.time_windows_json.clone()
        } else {
            child.time_windows_json.clone()
        },
        cooldown_minutes: child.cooldown_minutes.or(parent.cooldown_minutes),
        disabled_features_json: serde_json::to_string(&disabled_features).unwrap_or_default(),
        disabled_routes_json: child.disabled_routes_json.clone(),
        require_approval_json: child.require_approval_json.clone(),
        log_sessions: parent.log_sessions || child.log_sessions,
        log_categories: parent.log_categories || child.log_categories,
        log_policy_events: parent.log_policy_events || child.log_policy_events,
        retention_days: parent.retention_days.max(child.retention_days),
        subject_can_view: parent.subject_can_view && child.subject_can_view,
        effective_from: child.effective_from.clone(),
        effective_until: child.effective_until.clone(),
        version: child.version,
        created_at: child.created_at.clone(),
        updated_at: child.updated_at.clone(),
    }
}

// =============================================================================
// Stewardship Grant Functions
// =============================================================================

/// Create a new stewardship grant
#[hdk_extern]
pub fn create_stewardship_grant(input: CreateGrantInput) -> ExternResult<StewardshipGrantOutput> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Calculate dates
    let expires_ms = input.expires_in_days as u64 * 24 * 60 * 60 * 1000 * 1000;
    let review_ms = input.review_in_days as u64 * 24 * 60 * 60 * 1000 * 1000;

    let expires_at = format!("{:?}", now.checked_add(&std::time::Duration::from_micros(expires_ms)).unwrap_or(now));
    let review_at = format!("{:?}", now.checked_add(&std::time::Duration::from_micros(review_ms)).unwrap_or(now));

    let grant_id = format!("grant-{}-{}-{}", my_human.human.id, input.subject_id,
        timestamp.replace([':', ' ', '(', ')'], "-"));

    let grant = StewardshipGrant {
        id: grant_id.clone(),
        steward_id: my_human.human.id.clone(),
        subject_id: input.subject_id.clone(),
        authority_basis: input.authority_basis,
        evidence_hash: input.evidence_hash,
        verified_by: input.verified_by,
        content_filtering: input.content_filtering,
        time_limits: input.time_limits,
        feature_restrictions: input.feature_restrictions,
        activity_monitoring: input.activity_monitoring,
        policy_delegation: input.policy_delegation,
        delegatable: input.delegatable,
        delegated_from: None,
        delegation_depth: 0,
        granted_at: timestamp.clone(),
        expires_at,
        review_at,
        status: "active".to_string(),
        appeal_id: None,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::StewardshipGrant(grant.clone()))?;

    // Create lookup links
    let id_anchor = StringAnchor::new("stewardship_grant_id", &grant_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToStewardshipGrant, ())?;

    let steward_anchor = StringAnchor::new("steward_grants", &my_human.human.id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor))?;
    create_link(steward_anchor_hash, action_hash.clone(), LinkTypes::StewardToGrant, ())?;

    let subject_anchor = StringAnchor::new("subject_grants", &input.subject_id);
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;
    create_link(subject_anchor_hash, action_hash.clone(), LinkTypes::SubjectToGrant, ())?;

    let status_anchor = StringAnchor::new("grant_status", "active");
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::GrantByStatus, ())?;

    // Emit signal
    emit_signal(StewardshipSignal::GrantCreated {
        action_hash: action_hash.clone(),
        grant: grant.clone(),
    })?;

    Ok(StewardshipGrantOutput { action_hash, grant })
}

/// Delegate a grant to another steward
#[hdk_extern]
pub fn delegate_grant(input: DelegateGrantInput) -> ExternResult<StewardshipGrantOutput> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    // Get parent grant
    let parent = get_grant_by_id(input.parent_grant_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Parent grant not found".to_string())))?;

    // Verify I am the steward of parent grant
    if parent.grant.steward_id != my_human.human.id {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the steward can delegate a grant".to_string()
        )));
    }

    // Verify parent is delegatable
    if !parent.grant.delegatable {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "This grant cannot be delegated".to_string()
        )));
    }

    // Check delegation depth
    if parent.grant.delegation_depth >= 3 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Maximum delegation depth reached".to_string()
        )));
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let expires_ms = input.expires_in_days as u64 * 24 * 60 * 60 * 1000 * 1000;
    let expires_at = format!("{:?}", now.checked_add(&std::time::Duration::from_micros(expires_ms)).unwrap_or(now));

    let grant_id = format!("grant-{}-{}-{}", input.new_steward_id, parent.grant.subject_id,
        timestamp.replace([':', ' ', '(', ')'], "-"));

    // Delegated grants can only have equal or fewer capabilities than parent
    let grant = StewardshipGrant {
        id: grant_id.clone(),
        steward_id: input.new_steward_id.clone(),
        subject_id: parent.grant.subject_id.clone(),
        authority_basis: parent.grant.authority_basis.clone(),
        evidence_hash: parent.grant.evidence_hash.clone(),
        verified_by: my_human.human.id.clone(), // Delegating steward verifies
        content_filtering: input.content_filtering.unwrap_or(parent.grant.content_filtering) && parent.grant.content_filtering,
        time_limits: input.time_limits.unwrap_or(parent.grant.time_limits) && parent.grant.time_limits,
        feature_restrictions: input.feature_restrictions.unwrap_or(parent.grant.feature_restrictions) && parent.grant.feature_restrictions,
        activity_monitoring: input.activity_monitoring.unwrap_or(parent.grant.activity_monitoring) && parent.grant.activity_monitoring,
        policy_delegation: input.policy_delegation.unwrap_or(false) && parent.grant.policy_delegation,
        delegatable: false, // Delegated grants cannot be further delegated by default
        delegated_from: Some(input.parent_grant_id),
        delegation_depth: parent.grant.delegation_depth + 1,
        granted_at: timestamp.clone(),
        expires_at: expires_at.clone(),
        review_at: expires_at, // Review at expiry for delegated grants
        status: "active".to_string(),
        appeal_id: None,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::StewardshipGrant(grant.clone()))?;

    // Create lookup links
    let id_anchor = StringAnchor::new("stewardship_grant_id", &grant_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToStewardshipGrant, ())?;

    let steward_anchor = StringAnchor::new("steward_grants", &input.new_steward_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor))?;
    create_link(steward_anchor_hash, action_hash.clone(), LinkTypes::StewardToGrant, ())?;

    let subject_anchor = StringAnchor::new("subject_grants", &parent.grant.subject_id);
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;
    create_link(subject_anchor_hash, action_hash.clone(), LinkTypes::SubjectToGrant, ())?;

    // Link to parent grant for delegation chain
    let delegation_anchor = StringAnchor::new("delegated_from", &parent.grant.id);
    let delegation_anchor_hash = hash_entry(&EntryTypes::StringAnchor(delegation_anchor))?;
    create_link(delegation_anchor_hash, action_hash.clone(), LinkTypes::DelegatedFromGrant, ())?;

    emit_signal(StewardshipSignal::GrantCreated {
        action_hash: action_hash.clone(),
        grant: grant.clone(),
    })?;

    Ok(StewardshipGrantOutput { action_hash, grant })
}

/// Revoke a stewardship grant
#[hdk_extern]
pub fn revoke_grant(grant_id: String) -> ExternResult<StewardshipGrantOutput> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let existing = get_grant_by_id(grant_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Grant not found".to_string())))?;

    // Only steward or subject can revoke (subject can always revoke consent)
    if existing.grant.steward_id != my_human.human.id && existing.grant.subject_id != my_human.human.id {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only steward or subject can revoke a grant".to_string()
        )));
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let mut updated = existing.grant.clone();
    updated.status = "revoked".to_string();
    updated.updated_at = timestamp;

    let action_hash = create_entry(&EntryTypes::StewardshipGrant(updated.clone()))?;

    // Update ID link
    let id_anchor = StringAnchor::new("stewardship_grant_id", &grant_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    let old_links = get_links(
        LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToStewardshipGrant)?,
        GetStrategy::default(),
    )?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToStewardshipGrant, ())?;

    // Update status links
    let old_status_anchor = StringAnchor::new("grant_status", &existing.grant.status);
    let old_status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_status_anchor))?;
    let old_status_links = get_links(
        LinkQuery::try_new(old_status_anchor_hash, LinkTypes::GrantByStatus)?,
        GetStrategy::default(),
    )?;
    for link in old_status_links {
        if link.target == existing.action_hash.clone().into() {
            delete_link(link.create_link_hash, GetOptions::default())?;
        }
    }

    emit_signal(StewardshipSignal::GrantRevoked {
        grant_id,
        reason: "Revoked by steward or subject".to_string(),
    })?;

    Ok(StewardshipGrantOutput { action_hash, grant: updated })
}

/// Get grant by ID
#[hdk_extern]
pub fn get_grant_by_id(id: String) -> ExternResult<Option<StewardshipGrantOutput>> {
    let id_anchor = StringAnchor::new("stewardship_grant_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToStewardshipGrant)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(grant) = record.entry().to_app_option::<StewardshipGrant>().ok().flatten() {
                    return Ok(Some(StewardshipGrantOutput { action_hash, grant }));
                }
            }
        }
    }

    Ok(None)
}

/// Get grants where I am steward
#[hdk_extern]
pub fn get_grants_as_steward(steward_id: String) -> ExternResult<Vec<StewardshipGrantOutput>> {
    let steward_anchor = StringAnchor::new("steward_grants", &steward_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor))?;

    let query = LinkQuery::try_new(steward_anchor_hash, LinkTypes::StewardToGrant)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(grant) = record.entry().to_app_option::<StewardshipGrant>().ok().flatten() {
                    results.push(StewardshipGrantOutput { action_hash, grant });
                }
            }
        }
    }

    Ok(results)
}

/// Get grants for a subject (where I am being stewarded)
#[hdk_extern]
pub fn get_grants_for_subject(subject_id: String) -> ExternResult<Vec<StewardshipGrantOutput>> {
    let subject_anchor = StringAnchor::new("subject_grants", &subject_id);
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;

    let query = LinkQuery::try_new(subject_anchor_hash, LinkTypes::SubjectToGrant)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(grant) = record.entry().to_app_option::<StewardshipGrant>().ok().flatten() {
                    results.push(StewardshipGrantOutput { action_hash, grant });
                }
            }
        }
    }

    Ok(results)
}

/// Get my subjects (grants where I am steward)
#[hdk_extern]
pub fn get_my_subjects(_: ()) -> ExternResult<Vec<StewardshipGrantOutput>> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    get_grants_as_steward(my_human.human.id)
}

/// Get my stewards (grants where I am being stewarded)
#[hdk_extern]
pub fn get_my_stewards(_: ()) -> ExternResult<Vec<StewardshipGrantOutput>> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    get_grants_for_subject(my_human.human.id)
}

// =============================================================================
// Device Policy Functions
// =============================================================================

/// Create or update a device policy
#[hdk_extern]
pub fn upsert_policy(input: UpsertPolicyInput) -> ExternResult<DevicePolicyOutput> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    // Get my steward tier for this subject
    let tier = get_my_steward_tier(&input.subject_id)?;

    // Verify I have authority to set policy for this subject
    if tier == "guide" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Guide tier can only suggest policies, not enforce them".to_string()
        )));
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let policy_id = format!("policy-{}-{}-{}",
        input.subject_id,
        input.device_id.clone().unwrap_or_else(|| "all".to_string()),
        my_human.human.id);

    // Check if policy exists
    let existing = get_policy_by_id(policy_id.clone())?;
    let version = existing.as_ref().map(|p| p.policy.version + 1).unwrap_or(1);

    let policy = DevicePolicy {
        id: policy_id.clone(),
        subject_id: input.subject_id.clone(),
        device_id: input.device_id,
        author_id: my_human.human.id.clone(),
        author_tier: tier,
        inherits_from: None, // Will be set by compute_policy_inheritance
        blocked_categories_json: serde_json::to_string(&input.blocked_categories).unwrap_or_default(),
        blocked_hashes_json: serde_json::to_string(&input.blocked_hashes).unwrap_or_default(),
        age_rating_max: input.age_rating_max,
        reach_level_max: input.reach_level_max,
        session_max_minutes: input.session_max_minutes,
        daily_max_minutes: input.daily_max_minutes,
        time_windows_json: input.time_windows_json,
        cooldown_minutes: input.cooldown_minutes,
        disabled_features_json: serde_json::to_string(&input.disabled_features).unwrap_or_default(),
        disabled_routes_json: serde_json::to_string(&input.disabled_routes).unwrap_or_default(),
        require_approval_json: serde_json::to_string(&input.require_approval).unwrap_or_default(),
        log_sessions: input.log_sessions,
        log_categories: input.log_categories,
        log_policy_events: input.log_policy_events,
        retention_days: input.retention_days,
        subject_can_view: input.subject_can_view,
        effective_from: timestamp.clone(),
        effective_until: None,
        version,
        created_at: if existing.is_some() { existing.unwrap().policy.created_at } else { timestamp.clone() },
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::DevicePolicy(policy.clone()))?;

    // Create/update lookup links
    let id_anchor = StringAnchor::new("device_policy_id", &policy_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    // Delete old links if updating
    let old_links = get_links(
        LinkQuery::try_new(id_anchor_hash.clone(), LinkTypes::IdToDevicePolicy)?,
        GetStrategy::default(),
    )?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToDevicePolicy, ())?;

    let subject_anchor = StringAnchor::new("subject_policies", &input.subject_id);
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;
    create_link(subject_anchor_hash, action_hash.clone(), LinkTypes::SubjectToPolicy, ())?;

    emit_signal(StewardshipSignal::PolicyUpdated {
        action_hash: action_hash.clone(),
        policy: policy.clone(),
    })?;

    Ok(DevicePolicyOutput { action_hash, policy })
}

/// Get policy by ID
#[hdk_extern]
pub fn get_policy_by_id(id: String) -> ExternResult<Option<DevicePolicyOutput>> {
    let id_anchor = StringAnchor::new("device_policy_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToDevicePolicy)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(policy) = record.entry().to_app_option::<DevicePolicy>().ok().flatten() {
                    return Ok(Some(DevicePolicyOutput { action_hash, policy }));
                }
            }
        }
    }

    Ok(None)
}

/// Get policies for a subject
#[hdk_extern]
pub fn get_policies_for_subject(subject_id: String) -> ExternResult<Vec<DevicePolicyOutput>> {
    let subject_anchor = StringAnchor::new("subject_policies", &subject_id);
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;

    let query = LinkQuery::try_new(subject_anchor_hash, LinkTypes::SubjectToPolicy)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(policy) = record.entry().to_app_option::<DevicePolicy>().ok().flatten() {
                    results.push(DevicePolicyOutput { action_hash, policy });
                }
            }
        }
    }

    Ok(results)
}

/// Get my computed policy (merged from all layers)
#[hdk_extern]
pub fn get_my_computed_policy(_: ()) -> ExternResult<ComputedPolicy> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    compute_policy_for_subject(my_human.human.id)
}

/// Compute merged policy for a subject
#[hdk_extern]
pub fn compute_policy_for_subject(subject_id: String) -> ExternResult<ComputedPolicy> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get all policies for this subject
    let policies = get_policies_for_subject(subject_id.clone())?;

    // Sort by tier (org -> guardian -> elohim -> self)
    let tier_order = ["coordinator", "guardian", "constitutional", "self"];
    let mut sorted_policies: Vec<_> = policies
        .iter()
        .filter(|p| p.policy.effective_until.is_none() || p.policy.effective_until.as_ref().unwrap() > &timestamp)
        .collect();

    sorted_policies.sort_by(|a, b| {
        let a_idx = tier_order.iter().position(|&t| t == a.policy.author_tier).unwrap_or(99);
        let b_idx = tier_order.iter().position(|&t| t == b.policy.author_tier).unwrap_or(99);
        a_idx.cmp(&b_idx)
    });

    // Merge policies in order (each layer adds restrictions)
    let mut blocked_categories: Vec<String> = Vec::new();
    let mut blocked_hashes: Vec<String> = Vec::new();
    let mut disabled_features: Vec<String> = Vec::new();
    let mut disabled_routes: Vec<String> = Vec::new();
    let mut require_approval: Vec<String> = Vec::new();
    let mut session_max: Option<u32> = None;
    let mut daily_max: Option<u32> = None;
    let mut age_rating: Option<String> = None;
    let mut reach_max: Option<u8> = None;
    let mut time_windows = "[]".to_string();
    let mut cooldown: Option<u32> = None;
    let mut log_sessions = false;
    let mut log_categories = false;
    let mut log_policy_events = false;
    let mut retention_days: u32 = 0;
    let mut subject_can_view = true;

    for policy_output in &sorted_policies {
        let p = &policy_output.policy;

        // Merge blocked categories
        let cats: Vec<String> = serde_json::from_str(&p.blocked_categories_json).unwrap_or_default();
        for cat in cats {
            if !blocked_categories.contains(&cat) {
                blocked_categories.push(cat);
            }
        }

        // Merge blocked hashes
        let hashes: Vec<String> = serde_json::from_str(&p.blocked_hashes_json).unwrap_or_default();
        for hash in hashes {
            if !blocked_hashes.contains(&hash) {
                blocked_hashes.push(hash);
            }
        }

        // Merge disabled features
        let feats: Vec<String> = serde_json::from_str(&p.disabled_features_json).unwrap_or_default();
        for feat in feats {
            if !disabled_features.contains(&feat) {
                disabled_features.push(feat);
            }
        }

        // Merge routes
        let routes: Vec<String> = serde_json::from_str(&p.disabled_routes_json).unwrap_or_default();
        for route in routes {
            if !disabled_routes.contains(&route) {
                disabled_routes.push(route);
            }
        }

        // Merge approval requirements
        let approvals: Vec<String> = serde_json::from_str(&p.require_approval_json).unwrap_or_default();
        for approval in approvals {
            if !require_approval.contains(&approval) {
                require_approval.push(approval);
            }
        }

        // Take most restrictive time limits
        session_max = match (session_max, p.session_max_minutes) {
            (Some(s), Some(p)) => Some(s.min(p)),
            (Some(s), None) => Some(s),
            (None, Some(p)) => Some(p),
            (None, None) => None,
        };

        daily_max = match (daily_max, p.daily_max_minutes) {
            (Some(d), Some(p)) => Some(d.min(p)),
            (Some(d), None) => Some(d),
            (None, Some(p)) => Some(p),
            (None, None) => None,
        };

        // Take most restrictive age rating
        let age_order = ["G", "PG", "PG-13", "R", "NC-17"];
        age_rating = match (&age_rating, &p.age_rating_max) {
            (Some(a), Some(b)) => {
                let a_idx = age_order.iter().position(|&r| r == a).unwrap_or(4);
                let b_idx = age_order.iter().position(|&r| r == b).unwrap_or(4);
                Some(age_order[a_idx.min(b_idx)].to_string())
            }
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        // Take most restrictive reach level
        reach_max = match (reach_max, p.reach_level_max) {
            (Some(r), Some(p)) => Some(r.min(p)),
            (Some(r), None) => Some(r),
            (None, Some(p)) => Some(p),
            (None, None) => None,
        };

        // Use time windows from highest tier that has them
        if !p.time_windows_json.is_empty() && p.time_windows_json != "[]" {
            time_windows = p.time_windows_json.clone();
        }

        cooldown = p.cooldown_minutes.or(cooldown);

        // Monitoring is additive
        log_sessions = log_sessions || p.log_sessions;
        log_categories = log_categories || p.log_categories;
        log_policy_events = log_policy_events || p.log_policy_events;
        retention_days = retention_days.max(p.retention_days);

        // Subject view is restrictive (all must allow)
        subject_can_view = subject_can_view && p.subject_can_view;
    }

    Ok(ComputedPolicy {
        subject_id,
        computed_at: timestamp,
        blocked_categories,
        blocked_hashes,
        age_rating_max: age_rating,
        reach_level_max: reach_max,
        session_max_minutes: session_max,
        daily_max_minutes: daily_max,
        time_windows_json: time_windows,
        cooldown_minutes: cooldown,
        disabled_features,
        disabled_routes,
        require_approval,
        log_sessions,
        log_categories,
        log_policy_events,
        retention_days,
        subject_can_view,
    })
}

/// Check if content can be accessed
#[hdk_extern]
pub fn check_content_access(input: ContentCheckInput) -> ExternResult<PolicyDecision> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let policy = compute_policy_for_subject(my_human.human.id)?;

    // Check blocked hashes
    if policy.blocked_hashes.contains(&input.content_hash) {
        return Ok(PolicyDecision::Block { reason: "Content is blocked".to_string() });
    }

    // Check blocked categories
    for cat in &input.categories {
        if policy.blocked_categories.contains(cat) {
            return Ok(PolicyDecision::Block {
                reason: format!("Category '{}' is blocked", cat)
            });
        }
    }

    // Check age rating
    if let (Some(max), Some(content_rating)) = (&policy.age_rating_max, &input.age_rating) {
        let age_order = ["G", "PG", "PG-13", "R", "NC-17"];
        let max_idx = age_order.iter().position(|&r| r == max).unwrap_or(4);
        let content_idx = age_order.iter().position(|&r| r == content_rating).unwrap_or(4);
        if content_idx > max_idx {
            return Ok(PolicyDecision::Block {
                reason: format!("Content rating '{}' exceeds allowed '{}'", content_rating, max)
            });
        }
    }

    // Check reach level
    if let (Some(max), Some(reach)) = (policy.reach_level_max, input.reach_level) {
        if reach > max {
            return Ok(PolicyDecision::Block {
                reason: format!("Content reach level {} exceeds allowed {}", reach, max)
            });
        }
    }

    Ok(PolicyDecision::Allow)
}

// =============================================================================
// Appeal Functions
// =============================================================================

/// File an appeal against a grant or policy
#[hdk_extern]
pub fn file_appeal(input: FileAppealInput) -> ExternResult<StewardshipAppealOutput> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Verify grant exists
    let grant = get_grant_by_id(input.grant_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Grant not found".to_string())))?;

    // Verify appellant is the subject or an advocate
    if grant.grant.subject_id != my_human.human.id && input.advocate_id.is_none() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the subject or their advocate can file an appeal".to_string()
        )));
    }

    let appeal_id = format!("appeal-{}-{}", input.grant_id,
        timestamp.replace([':', ' ', '(', ')'], "-"));

    // Calculate expiry (appeals must be decided within 30 days)
    let expires_ms = 30u64 * 24 * 60 * 60 * 1000 * 1000;
    let expires_at = format!("{:?}", now.checked_add(&std::time::Duration::from_micros(expires_ms)).unwrap_or(now));

    let appeal = StewardshipAppeal {
        id: appeal_id.clone(),
        appellant_id: if input.advocate_id.is_some() {
            grant.grant.subject_id.clone()
        } else {
            my_human.human.id.clone()
        },
        grant_id: input.grant_id,
        policy_id: input.policy_id,
        appeal_type: input.appeal_type,
        grounds_json: serde_json::to_string(&input.grounds).unwrap_or_default(),
        evidence_json: input.evidence_json,
        advocate_id: input.advocate_id,
        advocate_notes: None,
        arbitration_layer: "constitutional".to_string(),
        assigned_to: None,
        status: "filed".to_string(),
        status_changed_at: Some(timestamp.clone()),
        decision_json: None,
        decision_made_by: None,
        decision_made_at: None,
        filed_at: timestamp.clone(),
        expires_at,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::StewardshipAppeal(appeal.clone()))?;

    // Create lookup links
    let id_anchor = StringAnchor::new("stewardship_appeal_id", &appeal_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToStewardshipAppeal, ())?;

    let appellant_anchor = StringAnchor::new("appellant_appeals", &appeal.appellant_id);
    let appellant_anchor_hash = hash_entry(&EntryTypes::StringAnchor(appellant_anchor))?;
    create_link(appellant_anchor_hash, action_hash.clone(), LinkTypes::AppellantToAppeal, ())?;

    let grant_anchor = StringAnchor::new("grant_appeals", &appeal.grant_id);
    let grant_anchor_hash = hash_entry(&EntryTypes::StringAnchor(grant_anchor))?;
    create_link(grant_anchor_hash, action_hash.clone(), LinkTypes::GrantToAppeal, ())?;

    let status_anchor = StringAnchor::new("appeal_status", "filed");
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::AppealByStatus, ())?;

    emit_signal(StewardshipSignal::AppealFiled {
        action_hash: action_hash.clone(),
        appeal: appeal.clone(),
    })?;

    Ok(StewardshipAppealOutput { action_hash, appeal })
}

/// Get appeal by ID
#[hdk_extern]
pub fn get_appeal_by_id(id: String) -> ExternResult<Option<StewardshipAppealOutput>> {
    let id_anchor = StringAnchor::new("stewardship_appeal_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToStewardshipAppeal)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(appeal) = record.entry().to_app_option::<StewardshipAppeal>().ok().flatten() {
                    return Ok(Some(StewardshipAppealOutput { action_hash, appeal }));
                }
            }
        }
    }

    Ok(None)
}

/// Get my appeals (where I am appellant)
#[hdk_extern]
pub fn get_my_appeals(_: ()) -> ExternResult<Vec<StewardshipAppealOutput>> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    let appellant_anchor = StringAnchor::new("appellant_appeals", &my_human.human.id);
    let appellant_anchor_hash = hash_entry(&EntryTypes::StringAnchor(appellant_anchor))?;

    let query = LinkQuery::try_new(appellant_anchor_hash, LinkTypes::AppellantToAppeal)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(appeal) = record.entry().to_app_option::<StewardshipAppeal>().ok().flatten() {
                    results.push(StewardshipAppealOutput { action_hash, appeal });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Activity Logging Functions
// =============================================================================

/// Log activity (only if monitoring is enabled)
#[hdk_extern]
pub fn log_activity(input: LogActivityInput) -> ExternResult<ActivityLogOutput> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    // Get computed policy to check if logging is enabled
    let policy = compute_policy_for_subject(my_human.human.id.clone())?;

    // Only log if at least one logging option is enabled
    if !policy.log_sessions && !policy.log_categories && !policy.log_policy_events {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Activity logging is not enabled for this subject".to_string()
        )));
    }

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Calculate retention expiry
    let retention_ms = policy.retention_days as u64 * 24 * 60 * 60 * 1000 * 1000;
    let retention_expires_at = format!("{:?}", now.checked_add(&std::time::Duration::from_micros(retention_ms)).unwrap_or(now));

    let log_id = format!("log-{}-{}", my_human.human.id,
        timestamp.replace([':', ' ', '(', ')'], "-"));

    let log = ActivityLog {
        id: log_id.clone(),
        subject_id: my_human.human.id.clone(),
        device_id: None,
        session_id: input.session_id.clone(),
        session_started_at: timestamp.clone(),
        session_duration_minutes: input.session_duration_minutes,
        categories_accessed_json: if policy.log_categories {
            serde_json::to_string(&input.categories_accessed).unwrap_or_default()
        } else {
            "[]".to_string()
        },
        policy_events_json: if policy.log_policy_events {
            input.policy_events_json
        } else {
            "[]".to_string()
        },
        logged_at: timestamp,
        retention_expires_at,
    };

    let action_hash = create_entry(&EntryTypes::ActivityLog(log.clone()))?;

    // Create lookup links
    let id_anchor = StringAnchor::new("activity_log_id", &log_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToActivityLog, ())?;

    let subject_anchor = StringAnchor::new("subject_activity_logs", &my_human.human.id);
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;
    create_link(subject_anchor_hash, action_hash.clone(), LinkTypes::SubjectToActivityLog, ())?;

    let session_anchor = StringAnchor::new("session_logs", &input.session_id);
    let session_anchor_hash = hash_entry(&EntryTypes::StringAnchor(session_anchor))?;
    create_link(session_anchor_hash, action_hash.clone(), LinkTypes::SessionToActivityLog, ())?;

    Ok(ActivityLogOutput { action_hash, log })
}

/// Get my activity logs (if subject_can_view is true)
#[hdk_extern]
pub fn get_my_activity_logs(_: ()) -> ExternResult<Vec<ActivityLogOutput>> {
    let my_human = super::get_my_human(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Must have Human profile".to_string())))?;

    // Check if I can view my own logs
    let policy = compute_policy_for_subject(my_human.human.id.clone())?;
    if !policy.subject_can_view {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Activity logs are not viewable by subject".to_string()
        )));
    }

    let subject_anchor = StringAnchor::new("subject_activity_logs", &my_human.human.id);
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;

    let query = LinkQuery::try_new(subject_anchor_hash, LinkTypes::SubjectToActivityLog)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    let now = format!("{:?}", sys_time()?);

    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(log) = record.entry().to_app_option::<ActivityLog>().ok().flatten() {
                    // Only return logs that haven't expired
                    if log.retention_expires_at > now {
                        results.push(ActivityLogOutput { action_hash, log });
                    }
                }
            }
        }
    }

    Ok(results)
}
