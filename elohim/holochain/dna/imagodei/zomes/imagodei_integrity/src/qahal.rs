//! Qahal substrate entries: Collective, Membership (polymorphic), CollabAgreement.
//!
//! Per spec: genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md
//! Pure-data validation only — link traversal happens in the coordinator zome.

use hdi::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberKind {
    Person,
    Collective,
    ElohimAgent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipRole {
    Steward,
    Contributor,
    Observer,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct Collective {
    pub founder_agent_cid: String,
    pub charter: String,
    pub display_name: String,
    pub created_at_block_height: u64,
    pub salt: String,
    /// When this Collective is a Collab-Qahal instantiated from a CollabAgreement,
    /// references the agreement's ActionHash. None for first-order Collectives.
    pub anchor_agreement_cid: Option<String>,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct Membership {
    pub member_cid: String,
    pub member_kind: MemberKind,
    pub collective_cid: String,
    pub role: MembershipRole,
    /// Set when role == Steward and pending counter-attestation. Cleared once attested.
    pub sponsor_cid: Option<String>,
    pub joined_at_block_height: u64,
    /// Set when the Membership has been cleanly withdrawn. Future EconomicEvents
    /// emitted at block heights >= this value do not accrue to this member.
    pub withdrawn_at_block_height: Option<u64>,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CollabAgreement {
    pub authored_by_agent_cid: String,
    pub participants: Vec<String>, // Collective CIDs
    pub scope: String,
    pub share_allocation_json: String, // serialized ShareAllocation; see Task 7
    pub commons_pool_tribute: f64,     // 0.0 < value <= 1.0
    pub governance_terms_json: String, // serialized GovernanceTerms; see Task 7
    pub anchor_collective_cid: Option<String>, // populated once Collab-Qahal instantiated
    pub initial_tier: String, // "T0" only for M1; "T1" rejected (M3) until commons-elohim path lands
    pub created_at_block_height: u64,
    pub salt: String,
}

/// Pure-data validation for Collective.
pub fn validate_collective_pure(c: &Collective) -> ExternResult<ValidateCallbackResult> {
    if c.charter.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Collective.charter must be non-empty".into(),
        ));
    }
    if c.charter.len() > 16 * 1024 {
        return Ok(ValidateCallbackResult::Invalid(
            "Collective.charter exceeds 16 KiB".into(),
        ));
    }
    if c.display_name.is_empty() || c.display_name.len() > 256 {
        return Ok(ValidateCallbackResult::Invalid(
            "Collective.display_name must be 1..=256 chars".into(),
        ));
    }
    if c.salt.len() != 32 || !c.salt.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Ok(ValidateCallbackResult::Invalid(
            "Collective.salt must be 32 hex chars".into(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Pure-data validation for Membership.
pub fn validate_membership_pure(m: &Membership) -> ExternResult<ValidateCallbackResult> {
    if m.member_cid.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Membership.member_cid must be non-empty".into(),
        ));
    }
    if m.collective_cid.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Membership.collective_cid must be non-empty".into(),
        ));
    }
    if matches!(m.role, MembershipRole::Steward) && m.sponsor_cid.is_none() {
        // Founder bypass: at Collective creation the founder's Steward Membership is created
        // by the coordinator atomically with no sponsor. The coordinator sets a synthetic
        // sponsor_cid = "founder" to satisfy this gate. See coordinator (Task 4).
        return Ok(ValidateCallbackResult::Invalid(
            "Steward role requires sponsor_cid".into(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Pure-data validation for CollabAgreement.
pub fn validate_collab_agreement_pure(a: &CollabAgreement) -> ExternResult<ValidateCallbackResult> {
    if a.participants.len() < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "CollabAgreement requires >= 2 participating Collectives".into(),
        ));
    }
    if a.commons_pool_tribute <= 0.0 || a.commons_pool_tribute > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "CollabAgreement.commons_pool_tribute must be in (0.0, 1.0]".into(),
        ));
    }
    if a.scope.is_empty() || a.scope.len() > 16 * 1024 {
        return Ok(ValidateCallbackResult::Invalid(
            "CollabAgreement.scope must be 1..=16 KiB".into(),
        ));
    }
    if a.initial_tier != "T0" {
        return Ok(ValidateCallbackResult::Invalid(
            "M1 only supports initial_tier=\"T0\"; T1+ requires commons-elohim path (M3)".into(),
        ));
    }
    if a.salt.len() != 32 || !a.salt.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Ok(ValidateCallbackResult::Invalid(
            "CollabAgreement.salt must be 32 hex chars".into(),
        ));
    }
    // share_allocation_json + governance_terms_json structural validation lives in the
    // coordinator (parsing arbitrary JSON inside the integrity validator is avoided —
    // pure-data field shape checks only here). The coordinator performs structural
    // checks before commit_entry. The DHT entry remains the source of truth either way.
    Ok(ValidateCallbackResult::Valid)
}
