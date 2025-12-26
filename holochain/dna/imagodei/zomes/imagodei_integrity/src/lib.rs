//! Imago Dei Integrity Zome
//!
//! Defines entry and link types for identity and social relationships:
//! - Human/Agent profiles
//! - Progress tracking (learning journeys)
//! - Attestations (credentials and achievements)
//! - Human relationships (social graph, custody)
//! - Content mastery (competency tracking)
//!
//! "Imago Dei" (Image of God) - every human has inherent dignity and worth.
//! This DNA provides self-sovereign identity without platform lock-in.

use hdi::prelude::*;

// =============================================================================
// Identity Constants
// =============================================================================

/// Agent types in the Elohim network
pub const AGENT_TYPES: [&str; 4] = [
    "human",        // Human individual
    "organization", // Organization or institution
    "ai-agent",     // AI assistant or agent
    "elohim",       // Governance/system entities
];

/// Visibility levels for profiles and content
pub const VISIBILITY_LEVELS: [&str; 3] = [
    "public",      // Visible to everyone
    "connections", // Visible to connected agents
    "private",     // Visible only to self
];

// =============================================================================
// Relationship Constants
// =============================================================================

/// Intimacy levels for human relationships (determines custody and access rights)
pub const INTIMACY_LEVELS: [&str; 5] = [
    "intimate",   // Spouse, immediate family, closest confidants (auto-custody enabled)
    "trusted",    // Extended family, very close friends (custody by consent)
    "familiar",   // Friends, colleagues, regular contacts (no auto-custody)
    "acquainted", // People you know but don't interact with regularly
    "public",     // Known publicly but no personal relationship
];

/// Human relationship types (social bonds between agents)
pub const HUMAN_RELATIONSHIP_TYPES: [&str; 12] = [
    "spouse",          // Married/life partner (intimate)
    "parent",          // Parent-child (intimate)
    "child",           // Parent-child (intimate)
    "sibling",         // Brother/sister (intimate or trusted)
    "grandparent",     // Grandparent-grandchild (trusted)
    "grandchild",      // Grandparent-grandchild (trusted)
    "extended-family", // Aunt, uncle, cousin (trusted or familiar)
    "trusted-friend",  // Close friend (trusted)
    "colleague",       // Work colleague (familiar)
    "neighbor",        // Geographic proximity (familiar)
    "community-member", // Same community/church/organization (familiar)
    "acquaintance",    // Known but not close (acquainted)
];

// =============================================================================
// Attestation Constants
// =============================================================================

/// Attestation categories
pub const ATTESTATION_CATEGORIES: [&str; 8] = [
    "learning",    // Path completion, content mastery
    "stewardship", // Content steward credentials
    "governance",  // Council, board, committee roles
    "community",   // Community participation
    "technical",   // Skills verification
    "conduct",     // Behavior attestations
    "identity",    // Identity verification
    "peer",        // Peer-granted recognitions
];

/// Attestation tiers (for tiered credentials)
pub const ATTESTATION_TIERS: [&str; 4] = ["bronze", "silver", "gold", "platinum"];

// =============================================================================
// ContributorPresence Constants
// =============================================================================

/// Presence states in the claim lifecycle
/// UNCLAIMED: Created for absent contributor, recognition accumulating
/// STEWARDED: A steward is caring for the presence
/// CLAIMED: Contributor has verified ownership and claimed the presence
pub const PRESENCE_STATES: [&str; 3] = ["unclaimed", "stewarded", "claimed"];

/// Verification methods for claiming a presence
pub const CLAIM_VERIFICATION_METHODS: [&str; 5] = [
    "email",       // Email verification to known address
    "social",      // Social media proof (Twitter, GitHub, etc.)
    "attestation", // Vouched by trusted attesters
    "signature",   // Cryptographic signature proof
    "manual",      // Manual verification by governance
];

// =============================================================================
// Mastery Constants
// =============================================================================

/// Mastery levels (Bloom's Taxonomy progression)
pub const MASTERY_LEVELS: [&str; 8] = [
    "not_started", // Never engaged
    "aware",       // Knows it exists
    "remember",    // Can recall basic facts
    "understand",  // Grasps meaning and concepts
    "apply",       // Can use knowledge in new situations
    "analyze",     // Can break down and examine
    "evaluate",    // Can make judgments
    "create",      // Can produce original work
];

// =============================================================================
// Human/Agent Entry Types
// =============================================================================

/// Human agent in the Elohim network (legacy, kept for backward compatibility)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Human {
    pub id: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub affinities: Vec<String>, // Topics/areas of interest
    pub profile_reach: String,   // public, community, private
    pub location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Progress tracking for a human on a learning path (legacy)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HumanProgress {
    pub id: String,
    pub human_id: String,
    pub path_id: String,
    pub current_step_index: u32,
    pub completed_step_indices: Vec<u32>,
    pub completed_content_ids: Vec<String>,
    pub started_at: String,
    pub last_activity_at: String,
    pub completed_at: Option<String>,
}

/// Agent - Universal identity for humans, orgs, AI agents, and Elohim.
///
/// This is the expanded identity model that supersedes Human for full
/// identity representation. Human is kept for backward compatibility.
///
/// Holochain mapping:
/// - id becomes AgentPubKey derivable
/// - Profile data published to DHT if visibility allows
///
/// W3C DID alignment: The `did` field provides cryptographic identity
/// while `id` remains human-friendly for routing.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Agent {
    pub id: String,
    pub agent_type: String, // human, organization, ai-agent, elohim
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar: Option<String>,
    pub affinities: Vec<String>, // Topics/areas of interest
    pub visibility: String,      // public, connections, private
    pub location: Option<String>,
    pub holochain_agent_key: Option<String>, // AgentPubKey as base64
    pub did: Option<String>,                 // W3C DID (did:web:elohim.host:agents:{id})
    pub activity_pub_type: Option<String>,   // Person, Organization, Service, etc.
    pub created_at: String,
    pub updated_at: String,
}

/// AgentProgress - Private progress on a specific learning path.
///
/// This expands HumanProgress with additional fields for:
/// - Affinity tracking (engagement depth)
/// - Personal artifacts (notes, reflections)
/// - Attestations earned
///
/// Lives on agent's private source chain, NOT published to DHT unless shared.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AgentProgress {
    pub id: String,
    pub agent_id: String,
    pub path_id: String,
    pub current_step_index: u32,
    pub completed_step_indices: Vec<u32>,
    pub completed_content_ids: Vec<String>,
    pub step_affinity_json: String,        // Record<number, number> as JSON
    pub step_notes_json: String,           // Record<number, string> as JSON
    pub reflection_responses_json: String, // Record<number, string[]> as JSON
    pub attestations_earned: Vec<String>,
    pub started_at: String,
    pub last_activity_at: String,
    pub completed_at: Option<String>,
}

// =============================================================================
// Human Relationships (Qahal - Social Graph)
// =============================================================================

/// Human-to-human relationship for social graph and custody.
/// Enables multi-tier replication: family network backup + emergency access.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HumanRelationship {
    pub id: String,

    // === PARTIES ===
    pub party_a_id: String, // Agent ID of first party
    pub party_b_id: String, // Agent ID of second party

    // === RELATIONSHIP NATURE ===
    pub relationship_type: String, // See HUMAN_RELATIONSHIP_TYPES
    pub intimacy_level: String,    // See INTIMACY_LEVELS
    pub is_bidirectional: bool,    // Both parties acknowledge (true) or one-sided (false)

    // === CONSENT & PERMISSIONS ===
    pub consent_given_by_a: bool,  // Party A agrees to relationship
    pub consent_given_by_b: bool,  // Party B agrees to relationship
    pub custody_enabled_by_a: bool, // Party A allows B to custody their data
    pub custody_enabled_by_b: bool, // Party B allows A to custody their data

    // === CUSTODY & BACKUP ===
    pub auto_custody_enabled: bool,              // Should intimate/private content auto-replicate?
    pub shared_encryption_key_id: Option<String>, // For family-shared content
    pub emergency_access_enabled: bool,          // Can this relationship trigger emergency recovery?

    // === METADATA ===
    pub initiated_by: String,         // Which party initiated (party_a_id or party_b_id)
    pub verified_at: Option<String>,  // When both parties confirmed
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>, // Optional expiration (e.g., temporary trust)

    // === CONTEXT ===
    pub context_json: Option<String>, // Additional metadata: how they met, shared interests, etc.
    pub reach: String,                // Visibility of relationship itself (private, local, public)
}

// =============================================================================
// Attestation Entry
// =============================================================================

/// Attestation - Permanent achievement record.
///
/// Attestations are PERMANENT ACHIEVEMENTS, not competency tracking.
/// ContentMastery handles graduated competency.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Attestation {
    pub id: String,
    pub agent_id: String,
    pub category: String,         // AttestationCategory
    pub attestation_type: String, // Specific type within category
    pub display_name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub tier: Option<String>,     // bronze, silver, gold, platinum
    pub earned_via_json: String,  // EarnedVia details as JSON
    pub issued_at: String,
    pub issued_by: String,        // System, steward, governance, or peer
    pub expires_at: Option<String>,
    pub proof: Option<String>, // Cryptographic signature
}

// =============================================================================
// Content Mastery Entry
// =============================================================================

/// ContentMastery - An agent's mastery state for a specific content node.
///
/// Implements Bloom's Taxonomy progression from passive consumption
/// to active contribution. This powers:
/// - Khan Academy-style cross-path completion views
/// - Participation privilege gating (apply level = attestation gate)
/// - Expertise discovery queries
/// - Freshness/decay tracking
///
/// Storage: Lives on agent's private source chain.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContentMastery {
    pub id: String,
    pub human_id: String,
    pub content_id: String,
    pub mastery_level: String,      // MasteryLevel: not_started → create
    pub mastery_level_index: u32,   // 0-7 for comparison
    pub freshness_score: f64,       // 0.0-1.0, decays over time
    pub needs_refresh: bool,
    pub engagement_count: u32,
    pub last_engagement_type: String, // EngagementType
    pub last_engagement_at: String,
    pub level_achieved_at: String,
    pub content_version_at_mastery: Option<String>,
    pub assessment_evidence_json: String, // AssessmentEvidence[] as JSON
    pub privileges_json: String,          // ContentPrivilege[] as JSON
    pub created_at: String,
    pub updated_at: String,
    // Self-healing DNA fields
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
}

// =============================================================================
// ContributorPresence Entry
// =============================================================================

/// ContributorPresence - Recognition placeholder for absent contributors.
///
/// Philosophy:
/// - Anyone can create a presence for an absent contributor
/// - Recognition accumulates even while unclaimed
/// - Stewards care-take presences until the contributor joins
/// - Contributors can claim their presence and receive accumulated recognition
///
/// Lifecycle: UNCLAIMED → STEWARDED → CLAIMED
///
/// This enables "recognition before registration" - a key feature for
/// citing authors, speakers, and contributors who aren't yet on the network.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContributorPresence {
    pub id: String,
    pub display_name: String,
    pub presence_state: String, // unclaimed, stewarded, claimed

    // === EXTERNAL IDENTITY ===
    pub external_identifiers_json: String,    // ExternalIdentifier[] as JSON
    pub establishing_content_ids_json: String, // Content IDs that cite this contributor
    pub established_at: String,

    // === RECOGNITION ACCUMULATION ===
    pub affinity_total: i64,         // Total affinity points accumulated
    pub unique_engagers: u32,        // Count of unique agents who engaged
    pub citation_count: u32,         // How many times cited in content
    pub endorsements_json: String,   // Endorsement[] as JSON
    pub recognition_score: f64,      // Computed recognition score
    pub recognition_by_content_json: String, // Record<contentId, score> as JSON
    pub accumulating_since: String,
    pub last_recognition_at: String,

    // === STEWARDSHIP ===
    pub steward_id: Option<String>,
    pub stewardship_started_at: Option<String>,
    pub stewardship_commitment_id: Option<String>,
    pub stewardship_quality_score: Option<f64>,

    // === CLAIM PROCESS ===
    pub claim_initiated_at: Option<String>,
    pub claim_verified_at: Option<String>,
    pub claim_verification_method: Option<String>,
    pub claim_evidence_json: Option<String>,
    pub claimed_agent_id: Option<String>,
    pub claim_recognition_transferred_value: Option<i64>,
    pub claim_recognition_transferred_unit: Option<String>,
    pub claim_facilitated_by: Option<String>,

    // === METADATA ===
    pub invitations_json: String, // Invitation[] as JSON
    pub note: Option<String>,
    pub image: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Anchor Entry (for link indexing)
// =============================================================================

/// Generic string anchor for creating deterministic link bases
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StringAnchor {
    pub anchor_type: String,
    pub anchor_value: String,
}

impl StringAnchor {
    pub fn new(anchor_type: &str, anchor_value: &str) -> Self {
        Self {
            anchor_type: anchor_type.to_string(),
            anchor_value: anchor_value.to_string(),
        }
    }
}

// =============================================================================
// Entry Types Enum
// =============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Human(Human),
    HumanProgress(HumanProgress),
    Agent(Agent),
    AgentProgress(AgentProgress),
    HumanRelationship(HumanRelationship),
    Attestation(Attestation),
    ContentMastery(ContentMastery),
    ContributorPresence(ContributorPresence),
    StringAnchor(StringAnchor),
}

// =============================================================================
// Link Types
// =============================================================================

#[hdk_link_types]
pub enum LinkTypes {
    // Human/Agent identity links
    IdToHuman,              // Anchor(human_id) -> Human
    IdToAgent,              // Anchor(agent_id) -> Agent
    AgentKeyToHuman,        // AgentPubKey -> Human (one-to-one binding for auth)
    AgentKeyToAgent,        // AgentPubKey -> Agent
    HumanByAffinity,        // Anchor(affinity) -> Human
    AgentByAffinity,        // Anchor(affinity) -> Agent
    HumanByExternalId,      // Anchor(provider:credential_hash) -> Human

    // Progress links
    HumanToProgress,        // Human -> HumanProgress
    AgentToProgress,        // Agent -> AgentProgress
    ProgressToPath,         // Progress -> LearningPath (cross-DNA reference)

    // Human Relationship links (Social Graph)
    IdToHumanRelationship,       // Anchor(relationship_id) -> HumanRelationship
    AgentToRelationship,         // Anchor(agent_id) -> HumanRelationship
    HumanRelationshipByIntimacy, // Anchor(intimacy_level) -> HumanRelationship
    HumanRelationshipByType,     // Anchor(relationship_type) -> HumanRelationship
    RelationshipPendingConsent,  // Anchor(pending) -> HumanRelationship
    RelationshipWithCustody,     // Anchor(custody_enabled) -> HumanRelationship

    // Attestation links
    AgentToAttestation,      // Anchor(agent_id) -> Attestation
    AttestationByCategory,   // Anchor(category) -> Attestation
    AttestationByType,       // Anchor(attestation_type) -> Attestation

    // Content Mastery links
    HumanToMastery,          // Anchor(human_id) -> ContentMastery
    ContentToMastery,        // Anchor(content_id) -> ContentMastery
    MasteryByLevel,          // Anchor(level) -> ContentMastery

    // ContributorPresence links
    IdToPresence,            // Anchor(presence_id) -> ContributorPresence
    PresenceByState,         // Anchor(state) -> ContributorPresence
    StewardToPresence,       // Anchor(steward_id) -> ContributorPresence
    ClaimedAgentToPresence,  // Anchor(claimed_agent_id) -> ContributorPresence
}

// =============================================================================
// Validation
// =============================================================================

#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::Human(human) => validate_human(&human),
                EntryTypes::Agent(agent) => validate_agent(&agent),
                EntryTypes::HumanRelationship(rel) => validate_human_relationship(&rel, &action),
                EntryTypes::Attestation(attestation) => validate_attestation(&attestation),
                EntryTypes::ContentMastery(mastery) => validate_content_mastery(&mastery),
                EntryTypes::ContributorPresence(presence) => validate_contributor_presence(&presence),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            OpEntry::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Human(human) => validate_human(&human),
                EntryTypes::Agent(agent) => validate_agent(&agent),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

/// Validate Human entry
fn validate_human(human: &Human) -> ExternResult<ValidateCallbackResult> {
    if human.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Human ID cannot be empty".to_string(),
        ));
    }

    if human.display_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Human display_name cannot be empty".to_string(),
        ));
    }

    let valid_reaches = ["public", "community", "private"];
    if !valid_reaches.contains(&human.profile_reach.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid profile_reach '{}'. Must be one of: {:?}",
            human.profile_reach, valid_reaches
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate Agent entry
fn validate_agent(agent: &Agent) -> ExternResult<ValidateCallbackResult> {
    if agent.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Agent ID cannot be empty".to_string(),
        ));
    }

    if agent.display_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Agent display_name cannot be empty".to_string(),
        ));
    }

    if !AGENT_TYPES.contains(&agent.agent_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid agent_type '{}'. Must be one of: {:?}",
            agent.agent_type, AGENT_TYPES
        )));
    }

    if !VISIBILITY_LEVELS.contains(&agent.visibility.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid visibility '{}'. Must be one of: {:?}",
            agent.visibility, VISIBILITY_LEVELS
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate HumanRelationship entry
fn validate_human_relationship(
    rel: &HumanRelationship,
    _action: &Create,
) -> ExternResult<ValidateCallbackResult> {
    if rel.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Relationship ID cannot be empty".to_string(),
        ));
    }

    if rel.party_a_id.is_empty() || rel.party_b_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Both party IDs are required".to_string(),
        ));
    }

    if rel.party_a_id == rel.party_b_id {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot create relationship with self".to_string(),
        ));
    }

    if !HUMAN_RELATIONSHIP_TYPES.contains(&rel.relationship_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid relationship_type '{}'. Must be one of: {:?}",
            rel.relationship_type, HUMAN_RELATIONSHIP_TYPES
        )));
    }

    if !INTIMACY_LEVELS.contains(&rel.intimacy_level.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid intimacy_level '{}'. Must be one of: {:?}",
            rel.intimacy_level, INTIMACY_LEVELS
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate Attestation entry
fn validate_attestation(attestation: &Attestation) -> ExternResult<ValidateCallbackResult> {
    if attestation.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Attestation ID cannot be empty".to_string(),
        ));
    }

    if attestation.agent_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Attestation agent_id cannot be empty".to_string(),
        ));
    }

    if !ATTESTATION_CATEGORIES.contains(&attestation.category.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid category '{}'. Must be one of: {:?}",
            attestation.category, ATTESTATION_CATEGORIES
        )));
    }

    if let Some(ref tier) = attestation.tier {
        if !ATTESTATION_TIERS.contains(&tier.as_str()) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "Invalid tier '{}'. Must be one of: {:?}",
                tier, ATTESTATION_TIERS
            )));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate ContentMastery entry
fn validate_content_mastery(mastery: &ContentMastery) -> ExternResult<ValidateCallbackResult> {
    if mastery.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ContentMastery ID cannot be empty".to_string(),
        ));
    }

    if mastery.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ContentMastery human_id cannot be empty".to_string(),
        ));
    }

    if mastery.content_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ContentMastery content_id cannot be empty".to_string(),
        ));
    }

    if !MASTERY_LEVELS.contains(&mastery.mastery_level.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid mastery_level '{}'. Must be one of: {:?}",
            mastery.mastery_level, MASTERY_LEVELS
        )));
    }

    if mastery.freshness_score < 0.0 || mastery.freshness_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "freshness_score must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate ContributorPresence entry
fn validate_contributor_presence(presence: &ContributorPresence) -> ExternResult<ValidateCallbackResult> {
    if presence.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ContributorPresence ID cannot be empty".to_string(),
        ));
    }

    if presence.display_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ContributorPresence display_name cannot be empty".to_string(),
        ));
    }

    if !PRESENCE_STATES.contains(&presence.presence_state.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid presence_state '{}'. Must be one of: {:?}",
            presence.presence_state, PRESENCE_STATES
        )));
    }

    // Validate claim verification method if claim has been initiated
    if let Some(ref method) = presence.claim_verification_method {
        if !CLAIM_VERIFICATION_METHODS.contains(&method.as_str()) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "Invalid claim_verification_method '{}'. Must be one of: {:?}",
                method, CLAIM_VERIFICATION_METHODS
            )));
        }
    }

    // If claimed, must have claimed_agent_id
    if presence.presence_state == "claimed" && presence.claimed_agent_id.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Claimed presence must have claimed_agent_id".to_string(),
        ));
    }

    // If stewarded, must have steward_id
    if presence.presence_state == "stewarded" && presence.steward_id.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Stewarded presence must have steward_id".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
