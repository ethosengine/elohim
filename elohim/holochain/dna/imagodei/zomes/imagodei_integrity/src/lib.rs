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

// Stewardship module for graduated capability management
pub mod stewardship;
pub use stewardship::*;

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
// Recovery Constants
// =============================================================================

/// Recovery request status lifecycle
pub const RECOVERY_STATUSES: [&str; 4] = [
    "pending",   // Awaiting verification
    "approved",  // Threshold reached, can activate
    "rejected",  // Denied by stewards or expired
    "completed", // Recovery activated successfully
];

/// Recovery methods available
pub const RECOVERY_METHODS: [&str; 3] = [
    "social",       // M-of-N steward votes
    "elohim_check", // AI knowledge verification against imagodei profile
    "hint",         // Encrypted recovery hints
];

/// Recovery hint types
pub const RECOVERY_HINT_TYPES: [&str; 4] = [
    "password_hint",     // Encrypted password reminder
    "security_qa",       // Security questions/answers
    "trusted_doorways",  // Pre-registered doorway list
    "trusted_contacts",  // Emergency contact list
];

// =============================================================================
// Network-Attested Identity Constants (Phase 2 - Red-Team Enhancement)
// =============================================================================

/// Humanity attestation types for continuous identity verification
pub const ATTESTATION_TYPES: [&str; 5] = [
    "behavioral",   // Passive monitoring shows consistent behavior
    "interaction",  // Direct interaction with known human
    "video_call",   // Video verification with trusted party
    "in_person",    // Physical presence verification
    "elohim_check", // AI knowledge verification
];

/// Anomaly types detected by behavioral monitoring
pub const ANOMALY_TYPES: [&str; 6] = [
    "posting_pattern",    // Sudden change in posting frequency/style
    "content_style",      // AI-detected writing style deviation
    "relationship_change", // Rapid/mass relationship modifications
    "geo_shift",          // Geographic location anomaly
    "session_anomaly",    // Unusual login patterns
    "capability_abuse",   // Excessive use of privileged operations
];

/// Anomaly severity levels
pub const ANOMALY_SEVERITIES: [&str; 4] = [
    "low",      // Notable but not concerning
    "medium",   // Warrants monitoring
    "high",     // Should trigger alerts
    "critical", // Should trigger auto-freeze
];

/// Identity challenge types (community override)
pub const CHALLENGE_TYPES: [&str; 4] = [
    "hijack_report",   // Account appears compromised
    "impersonation",   // Someone claiming to be this person
    "spam",            // Posting spam/malicious content
    "anomaly_confirm", // Confirming detected anomaly
];

/// Challenge status values
pub const CHALLENGE_STATUSES: [&str; 4] = [
    "pending",   // Challenge open, accumulating support
    "upheld",    // Challenge succeeded, action taken
    "dismissed", // Challenge rejected
    "expired",   // Challenge timed out
];

/// Key revocation reasons
pub const REVOCATION_REASONS: [&str; 4] = [
    "compromised",       // Key known to be compromised
    "stolen",            // Device/key stolen
    "challenge_upheld",  // Community challenge succeeded
    "voluntary",         // User-initiated revocation
];

/// Identity freeze types
pub const FREEZE_TYPES: [&str; 3] = [
    "auto_anomaly",       // Triggered by anomaly detection
    "community_challenge", // Triggered by community reports
    "steward_emergency",  // Triggered by M-of-N steward consensus
];

/// Capabilities that can be frozen
pub const FREEZABLE_CAPABILITIES: [&str; 5] = [
    "post",               // Create new content
    "transfer",           // Transfer assets/points
    "modify_relationships", // Change relationship graph
    "vote",               // Participate in governance votes
    "attest",             // Issue attestations to others
];

/// Verification requirements to unfreeze
pub const UNFREEZE_REQUIREMENTS: [&str; 4] = [
    "elohim_check",      // AI knowledge verification
    "social_recovery",   // M-of-N steward approval
    "steward_interview", // Direct interview by steward
    "time_decay",        // Auto-unfreeze after timeout (low severity only)
];

/// Signing policy levels for distributed key custody
pub const SIGNING_POLICIES: [&str; 3] = [
    "normal",   // Standard M-of-N threshold
    "elevated", // Higher threshold for sensitive ops
    "emergency", // Recovery mode (relaxed threshold)
];

// =============================================================================
// Renewal Protocol Constants
// =============================================================================

/// Reasons for identity renewal (life transitions)
pub const RENEWAL_REASONS: [&str; 5] = [
    "device_destruction", // Device lost/destroyed, key irrecoverable
    "graduation",         // Life transition (child→adult, student→graduate)
    "key_compromise",     // Key known or suspected compromised
    "custodianship",      // Steward taking over for incapacitated human
    "legacy",             // Death or permanent incapacitation
];

/// Renewal attestation status lifecycle
pub const RENEWAL_STATUSES: [&str; 4] = [
    "pending",   // Awaiting witness votes
    "witnessed", // M-of-N threshold reached
    "rejected",  // Stewards rejected the renewal
    "expired",   // Time limit reached without threshold
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
// Recovery Entry Types
// =============================================================================

/// RecoveryRequest - Initiated when sovereign user needs to regain access.
///
/// Created by a doorway when a user who lost device access attempts recovery.
/// Stewards with `emergency_access_enabled` relationships can vote to approve.
/// Uses M-of-N threshold based on relationship intimacy levels.
///
/// Lifecycle: pending → approved/rejected → completed
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoveryRequest {
    pub id: String,

    // === REQUEST DETAILS ===
    pub human_id: String,        // Human ID requesting recovery
    pub doorway_id: String,      // Doorway initiating the request
    pub recovery_method: String, // See RECOVERY_METHODS: social, elohim_check, hint

    // === STATUS TRACKING ===
    pub status: String,             // See RECOVERY_STATUSES
    pub required_approvals: u32,    // M (threshold)
    pub current_approvals: u32,     // Current vote count
    pub confidence_score: f64,      // Combined confidence from all methods (0.0-1.0)

    // === ELOHIM VERIFICATION ===
    pub elohim_questions_json: Option<String>,  // Questions asked by Elohim
    pub elohim_score: Option<f64>,              // Score from AI verification (0.0-1.0)
    pub elohim_verified_at: Option<String>,

    // === TIMESTAMPS ===
    pub requested_at: String,
    pub expires_at: String,        // 48 hours default
    pub approved_at: Option<String>,
    pub completed_at: Option<String>,
}

/// RecoveryVote - Vote on a recovery request from a trusted relationship.
///
/// Only agents with `emergency_access_enabled` in their HumanRelationship
/// with the recovering human can vote. Intimacy level determines weight.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoveryVote {
    pub id: String,
    pub request_id: String,        // ID of RecoveryRequest being voted on
    pub voter_human_id: String,    // Human ID of voter
    pub approved: bool,            // Approve or reject
    pub attestation: String,       // Verification note: "Verified via video call"
    pub intimacy_level: String,    // Voter's intimacy level with requestor
    pub confidence_weight: f64,    // Weight contribution (based on intimacy)
    pub verification_method: String, // How voter verified: video_call, phone, in_person
    pub voted_at: String,
}

/// RecoveryHint - Encrypted recovery metadata stored by user.
///
/// Users can store encrypted hints to help with self-recovery:
/// - Password hints (encrypted)
/// - Security Q&A (encrypted)
/// - Trusted doorway list (encrypted)
/// - Emergency contact info (encrypted)
///
/// Only the user can decrypt with their password/key.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoveryHint {
    pub id: String,
    pub human_id: String,
    pub hint_type: String,       // See RECOVERY_HINT_TYPES
    pub encrypted_data: String,  // AES-GCM encrypted hint data
    pub encryption_nonce: String, // Nonce used for encryption
    pub version: u32,            // For updating hints
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Network-Attested Identity Entry Types (Phase 2 - Red-Team Enhancement)
// =============================================================================

/// HumanityWitness - Continuous attestation of identity.
///
/// Unlike one-time authentication, HumanityWitness provides ongoing
/// proof that an agent is acting as a consistent human identity.
/// Intimate relationships automatically generate witnesses through
/// normal interaction patterns.
///
/// Key insight: Identity is not just "who has the key" but "who is
/// consistently acting like this person."
///
/// Attestations decay over time (expires_at) requiring ongoing
/// relationship maintenance.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HumanityWitness {
    pub id: String,
    pub human_id: String,        // Human being attested
    pub witness_agent_id: String, // Agent providing attestation

    // === ATTESTATION DETAILS ===
    pub attestation_type: String,       // See ATTESTATION_TYPES
    pub confidence: f64,                // 0.0 - 1.0 confidence in identity
    pub behavioral_hash: Option<String>, // Hash of behavioral baseline for comparison

    // === EVIDENCE ===
    pub evidence_json: Option<String>,  // Supporting evidence (e.g., video call timestamp)
    pub verification_method: Option<String>, // How identity was verified

    // === LIFECYCLE ===
    pub created_at: String,
    pub expires_at: String,             // Attestations decay - must be renewed
    pub revoked_at: Option<String>,     // Explicitly revoked if witness changes mind
}

/// KeyStewardship - Distributed key custody via Shamir Secret Sharing.
///
/// Instead of a single doorway holding the full custodial key,
/// the key is split into N shards held by M trusted stewards.
/// Any M-of-N can cooperate to sign, but no single party can act alone.
///
/// This prevents the "doorway admin compromised" attack vector.
///
/// Signing policy escalates for sensitive operations:
/// - normal: Standard threshold for everyday actions
/// - elevated: Higher threshold for relationship changes, transfers
/// - emergency: Recovery mode allows threshold reduction
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct KeyStewardship {
    pub id: String,
    pub human_id: String,

    // === SHARD HOLDERS ===
    pub key_shard_holders: Vec<String>,  // Agent IDs holding key shards
    pub threshold_m: u32,                // M required to sign
    pub total_shards_n: u32,             // Total shards distributed

    // === POLICY ===
    pub signing_policy: String,          // See SIGNING_POLICIES
    pub elevated_threshold: Option<u32>, // Higher M for sensitive ops

    // === KEY METADATA ===
    pub key_generation_id: String,       // Which key generation this stewardship covers
    pub shard_commitment_hash: String,   // Commitment to verify shards

    // === LIFECYCLE ===
    pub created_at: String,
    pub updated_at: String,
    pub rotated_at: Option<String>,      // Last key rotation
}

/// IdentityAnomaly - Detected behavioral deviation.
///
/// AI monitors posting patterns, content style, relationship changes,
/// and login patterns. When deviation exceeds threshold, an anomaly
/// is recorded.
///
/// Critical anomalies can trigger auto-freeze (like the Sheila scenario
/// where clickbait posting pattern was obviously not the real person).
///
/// Anomalies create an audit trail even if they don't trigger action.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IdentityAnomaly {
    pub id: String,
    pub human_id: String,

    // === ANOMALY DETAILS ===
    pub anomaly_type: String,       // See ANOMALY_TYPES
    pub severity: String,           // See ANOMALY_SEVERITIES
    pub deviation_score: f64,       // 0.0 - 1.0 (1.0 = extreme deviation)

    // === EVIDENCE ===
    pub evidence_json: String,      // JSON blob: { baseline: {...}, current: {...}, diff: {...} }
    pub detection_method: String,   // AI model, rule-based, etc.

    // === ACTIONS ===
    pub auto_freeze_triggered: bool, // Did this anomaly trigger a freeze?
    pub freeze_id: Option<String>,   // Link to IdentityFreeze if triggered

    // === RESOLUTION ===
    pub acknowledged_at: Option<String>,  // User acknowledged anomaly
    pub resolved_at: Option<String>,
    pub resolution_json: Option<String>,  // How it was resolved

    // === TIMESTAMPS ===
    pub detected_at: String,
    pub expires_at: Option<String>,       // Anomalies may expire if not concerning
}

/// IdentityChallenge - Community override mechanism.
///
/// When humans report "this isn't the real person," challenges accumulate.
/// Each challenger's report is weighted by their relationship trust level.
/// When weighted_support exceeds threshold, automatic action is triggered.
///
/// Unlike Meta's ignored reports, challenges have BINDING EFFECT.
/// The architecture makes ignoring mass reports impossible.
///
/// Key innovation: Reports from intimate relationships count 3x,
/// from trusted 2x, familiar 1x, acquaintances 0.5x, public 0.1x.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IdentityChallenge {
    pub id: String,
    pub human_id: String,           // Human being challenged

    // === CHALLENGE DETAILS ===
    pub challenge_type: String,     // See CHALLENGE_TYPES
    pub initiator_id: String,       // Who started the challenge
    pub initiator_weight: f64,      // Weight of initial challenger

    // === EVIDENCE ===
    pub evidence_json: String,      // JSON: { description: "", screenshots: [], etc. }
    pub supporting_anomaly_id: Option<String>, // Link to IdentityAnomaly if related

    // === SUPPORT ACCUMULATION ===
    pub weighted_support: f64,      // Sum of all supporter weights
    pub supporter_count: u32,       // Number of unique supporters
    pub supporters_json: String,    // JSON array of { agent_id, weight, voted_at }

    // === THRESHOLDS ===
    pub freeze_threshold: f64,      // Weight needed to trigger freeze (default 10.0)
    pub revoke_threshold: f64,      // Weight needed to trigger revocation (default 25.0)

    // === STATUS ===
    pub status: String,             // See CHALLENGE_STATUSES
    pub status_changed_at: Option<String>,
    pub resolution_json: Option<String>, // How challenge was resolved

    // === TIMESTAMPS ===
    pub created_at: String,
    pub expires_at: String,         // Challenges expire after timeout (default 7 days)
}

/// ChallengeSupport - Support for an IdentityChallenge.
///
/// Separate entry type to prevent single-point manipulation of
/// the challenge's weighted_support field.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ChallengeSupport {
    pub id: String,
    pub challenge_id: String,       // Challenge being supported
    pub supporter_id: String,       // Human supporting the challenge
    pub weight: f64,                // Trust weight of supporter
    pub intimacy_level: String,     // Relationship intimacy level
    pub evidence_json: Option<String>, // Optional additional evidence
    pub created_at: String,
}

/// KeyRevocation - DHT consensus to invalidate a compromised key.
///
/// Unlike current architecture where keys are permanent, this allows
/// the network to revoke compromised keys via M-of-N steward consensus.
///
/// Once a key is revoked:
/// - All new actions signed by that key are REJECTED
/// - Content signed by revoked key can be flagged/removed
/// - User must go through recovery to get new key
///
/// This is the "nuclear option" for account takeover scenarios.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct KeyRevocation {
    pub id: String,
    pub human_id: String,

    // === REVOKED KEY ===
    pub revoked_key: String,        // The agent_pub_key being revoked
    pub reason: String,             // See REVOCATION_REASONS

    // === TRIGGER ===
    pub initiated_by: String,       // challenge_id, steward consensus, or voluntary
    pub trigger_type: String,       // challenge, steward_vote, voluntary

    // === STEWARD VOTES ===
    pub required_votes: u32,        // M required for revocation
    pub current_votes: u32,
    pub votes_json: String,         // JSON array of RevocationVote

    // === STATUS ===
    pub threshold_reached: bool,
    pub effective_at: Option<String>, // When revocation became active

    // === TIMESTAMPS ===
    pub created_at: String,
    pub updated_at: String,
}

/// RevocationVote - Individual steward vote on key revocation.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RevocationVote {
    pub id: String,
    pub revocation_id: String,      // KeyRevocation being voted on
    pub steward_id: String,         // Steward voting
    pub approved: bool,             // Approve or reject revocation
    pub attestation: String,        // Why they're voting this way
    pub voted_at: String,
}

/// IdentityFreeze - Emergency suspension of capabilities.
///
/// When anomaly detection or community challenge threshold is reached,
/// an IdentityFreeze immediately suspends specified capabilities.
/// The hijacker cannot continue posting/acting while verification
/// is pending.
///
/// Key difference from Meta: Freeze happens AUTOMATICALLY based on
/// network consensus, not on manual review that can be ignored.
///
/// Freeze can be:
/// - Partial: Only specific capabilities frozen
/// - Total: All capabilities frozen
/// - Time-limited: Auto-expires for low-severity
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct IdentityFreeze {
    pub id: String,
    pub human_id: String,

    // === FREEZE DETAILS ===
    pub freeze_type: String,           // See FREEZE_TYPES
    pub frozen_capabilities: Vec<String>, // See FREEZABLE_CAPABILITIES
    pub severity: String,              // Anomaly severity that triggered

    // === TRIGGER ===
    pub triggered_by: String,          // ID of anomaly/challenge/steward action
    pub trigger_type: String,          // anomaly, challenge, steward

    // === VERIFICATION REQUIRED ===
    pub requires_verification: String, // See UNFREEZE_REQUIREMENTS
    pub verification_attempts: u32,
    pub last_verification_at: Option<String>,

    // === STATUS ===
    pub is_active: bool,
    pub lifted_at: Option<String>,
    pub lifted_by: Option<String>,     // Who lifted the freeze
    pub lift_reason: Option<String>,

    // === TIMESTAMPS ===
    pub frozen_at: String,
    pub expires_at: Option<String>,    // Auto-lift for low severity
}

// =============================================================================
// Renewal Protocol Entry Types
// =============================================================================

/// RenewalAttestation - Core witness entry for identity renewal.
///
/// When a steward's devices are destroyed, the Holochain agent key dies.
/// Rather than fighting lair's constraints (keys aren't exportable), the
/// community witnesses a social ceremony and helps re-author the human's
/// world under a new identity.
///
/// This is the same protocol for ALL life transitions:
/// - Device loss → new key
/// - Graduation → new key (child→adult autonomy)
/// - Key compromise → new key (emergency rotation)
/// - Custodianship → steward takes over
/// - Legacy → community preserves after death
///
/// Signed by creating agent (a steward voter).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RenewalAttestation {
    pub id: String,
    pub human_id: String,                    // Stable identity being renewed
    pub old_agent_key: String,               // Agent key being retired (base64)
    pub new_agent_key: String,               // New agent key taking over (base64)
    pub renewal_reason: String,              // See RENEWAL_REASONS
    pub doorway_id: Option<String>,          // Doorway facilitating the renewal
    pub recovery_request_id: Option<String>, // Link to RecoveryRequest if applicable
    pub votes_json: String,                  // Serialized Vec<RenewalVote> — voter_id, approved, weight, intimacy, voted_at
    pub required_approvals: u32,             // M threshold
    pub current_approvals: u32,              // How many approved so far
    pub confidence_score: f64,               // 0.0-1.0 weighted confidence
    pub status: String,                      // See RENEWAL_STATUSES
    pub witnessed_at: Option<String>,        // When threshold was reached
    pub created_at: String,
    pub expires_at: String,
}

/// AgentRetirement - Marks an agent key as superseded.
///
/// Once a renewal is witnessed, the old key is retired and a new key takes
/// over. This creates an immutable chain: old→new→newer that queries can
/// follow to resolve "who is this agent now?"
///
/// The data plane (blobs, shards) survives fine — it's content-addressed.
/// This retirement record bridges the identity/control plane gap.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AgentRetirement {
    pub id: String,
    pub human_id: String,
    pub retired_agent_key: String,           // The dead key
    pub renewed_into_agent_key: String,      // The new key
    pub renewal_attestation_id: String,      // Proof of social witness
    pub retirement_reason: String,           // See RENEWAL_REASONS
    pub retired_at: String,
    pub created_at: String,
}

/// RelationshipRenewal - Bilateral reaffirmation of relationship under new key.
///
/// When a human gets a new agent key, all their HumanRelationships need to
/// be re-established with the new key. This is a bilateral process:
/// 1. The renewed human creates RelationshipRenewal entries
/// 2. Each counterparty co-signs to reaffirm
///
/// This ensures the social graph survives key rotation with explicit consent.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RelationshipRenewal {
    pub id: String,
    pub original_relationship_id: String,    // The HumanRelationship being renewed
    pub renewal_attestation_id: String,      // Proof of social witness
    pub human_id: String,                    // The human whose key changed
    pub new_agent_key: String,               // Their new key
    pub counterparty_id: String,             // The other party's human_id
    pub counterparty_agent_key: String,      // The other party's current key
    pub relationship_type: String,           // Carried forward from original
    pub intimacy_level: String,              // Carried forward or renegotiated
    pub emergency_access_enabled: bool,      // Reaffirmed
    pub reaffirmed_by_counterparty: bool,    // True when other party co-signs
    pub reaffirmed_at: Option<String>,       // When counterparty confirmed
    pub created_at: String,
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
    // Recovery entry types
    RecoveryRequest(RecoveryRequest),
    RecoveryVote(RecoveryVote),
    RecoveryHint(RecoveryHint),
    // Network-Attested Identity entry types (Phase 2)
    HumanityWitness(HumanityWitness),
    KeyStewardship(KeyStewardship),
    IdentityAnomaly(IdentityAnomaly),
    IdentityChallenge(IdentityChallenge),
    ChallengeSupport(ChallengeSupport),
    KeyRevocation(KeyRevocation),
    RevocationVote(RevocationVote),
    IdentityFreeze(IdentityFreeze),
    // Stewardship entry types (Graduated Capabilities)
    StewardshipGrant(StewardshipGrant),
    DevicePolicy(DevicePolicy),
    PolicyInheritance(PolicyInheritance),
    StewardshipAppeal(StewardshipAppeal),
    ActivityLog(ActivityLog),
    // Renewal protocol entry types
    RenewalAttestation(RenewalAttestation),
    AgentRetirement(AgentRetirement),
    RelationshipRenewal(RelationshipRenewal),
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

    // Recovery links
    IdToRecoveryRequest,         // Anchor(request_id) -> RecoveryRequest
    HumanToRecoveryRequest,      // Anchor(human_id) -> RecoveryRequest (user's requests)
    RecoveryRequestByStatus,     // Anchor(status) -> RecoveryRequest
    PendingRecoveryVote,         // Anchor(voter_human_id) -> RecoveryRequest (requests voter can act on)
    RecoveryVoteToRequest,       // RecoveryVote -> RecoveryRequest
    HumanToRecoveryHint,         // Anchor(human_id) -> RecoveryHint
    RecoveryHintByType,          // Anchor(hint_type) -> RecoveryHint

    // Network-Attested Identity links (Phase 2)
    // HumanityWitness links
    IdToHumanityWitness,         // Anchor(witness_id) -> HumanityWitness
    HumanToWitness,              // Anchor(human_id) -> HumanityWitness (witnesses FOR this human)
    WitnessAgentToWitness,       // Anchor(witness_agent_id) -> HumanityWitness (witnesses BY this agent)
    WitnessByType,               // Anchor(attestation_type) -> HumanityWitness
    ActiveWitnesses,             // Anchor(active) -> HumanityWitness (non-expired, non-revoked)

    // KeyStewardship links
    IdToKeyStewardship,          // Anchor(stewardship_id) -> KeyStewardship
    HumanToKeyStewardship,       // Anchor(human_id) -> KeyStewardship
    ShardHolderToStewardship,    // Anchor(shard_holder_id) -> KeyStewardship (stewardships this agent participates in)

    // IdentityAnomaly links
    IdToIdentityAnomaly,         // Anchor(anomaly_id) -> IdentityAnomaly
    HumanToAnomaly,              // Anchor(human_id) -> IdentityAnomaly
    AnomalyByType,               // Anchor(anomaly_type) -> IdentityAnomaly
    AnomalyBySeverity,           // Anchor(severity) -> IdentityAnomaly
    UnresolvedAnomalies,         // Anchor(unresolved) -> IdentityAnomaly

    // IdentityChallenge links
    IdToIdentityChallenge,       // Anchor(challenge_id) -> IdentityChallenge
    HumanToChallenge,            // Anchor(human_id) -> IdentityChallenge (challenges against this human)
    InitiatorToChallenge,        // Anchor(initiator_id) -> IdentityChallenge
    ChallengeByType,             // Anchor(challenge_type) -> IdentityChallenge
    ChallengeByStatus,           // Anchor(status) -> IdentityChallenge
    ActiveChallenges,            // Anchor(active) -> IdentityChallenge (pending, not expired)

    // ChallengeSupport links
    IdToChallengeSupport,        // Anchor(support_id) -> ChallengeSupport
    ChallengeToSupport,          // Anchor(challenge_id) -> ChallengeSupport
    SupporterToSupport,          // Anchor(supporter_id) -> ChallengeSupport

    // KeyRevocation links
    IdToKeyRevocation,           // Anchor(revocation_id) -> KeyRevocation
    HumanToKeyRevocation,        // Anchor(human_id) -> KeyRevocation
    RevokedKeyToRevocation,      // Anchor(revoked_key) -> KeyRevocation (lookup by key)
    PendingRevocations,          // Anchor(pending) -> KeyRevocation
    EffectiveRevocations,        // Anchor(effective) -> KeyRevocation (threshold reached)

    // RevocationVote links
    IdToRevocationVote,          // Anchor(vote_id) -> RevocationVote
    RevocationToVote,            // Anchor(revocation_id) -> RevocationVote
    StewardToRevocationVote,     // Anchor(steward_id) -> RevocationVote

    // IdentityFreeze links
    IdToIdentityFreeze,          // Anchor(freeze_id) -> IdentityFreeze
    HumanToFreeze,               // Anchor(human_id) -> IdentityFreeze
    ActiveFreezes,               // Anchor(active) -> IdentityFreeze (currently frozen)
    FreezeByType,                // Anchor(freeze_type) -> IdentityFreeze

    // Stewardship links (Graduated Capabilities)
    // StewardshipGrant links
    IdToStewardshipGrant,        // Anchor(grant_id) -> StewardshipGrant
    StewardToGrant,              // Anchor(steward_id) -> StewardshipGrant (grants where I am steward)
    SubjectToGrant,              // Anchor(subject_id) -> StewardshipGrant (grants affecting me)
    GrantByStatus,               // Anchor(status) -> StewardshipGrant
    GrantByAuthorityBasis,       // Anchor(authority_basis) -> StewardshipGrant
    ActiveGrants,                // Anchor(active) -> StewardshipGrant (currently active)
    DelegatedFromGrant,          // Anchor(parent_grant_id) -> StewardshipGrant (delegated children)

    // DevicePolicy links
    IdToDevicePolicy,            // Anchor(policy_id) -> DevicePolicy
    SubjectToPolicy,             // Anchor(subject_id) -> DevicePolicy
    AuthorToPolicy,              // Anchor(author_id) -> DevicePolicy
    PolicyByTier,                // Anchor(author_tier) -> DevicePolicy
    InheritedFromPolicy,         // Anchor(parent_policy_id) -> DevicePolicy (children)
    EffectivePolicies,           // Anchor(effective) -> DevicePolicy (currently in effect)

    // PolicyInheritance links
    IdToPolicyInheritance,       // Anchor(inheritance_id) -> PolicyInheritance
    SubjectToInheritance,        // Anchor(subject_id) -> PolicyInheritance

    // StewardshipAppeal links
    IdToStewardshipAppeal,       // Anchor(appeal_id) -> StewardshipAppeal
    AppellantToAppeal,           // Anchor(appellant_id) -> StewardshipAppeal
    GrantToAppeal,               // Anchor(grant_id) -> StewardshipAppeal (appeals against grant)
    AppealByStatus,              // Anchor(status) -> StewardshipAppeal
    AppealByType,                // Anchor(appeal_type) -> StewardshipAppeal
    ActiveAppeals,               // Anchor(pending) -> StewardshipAppeal

    // ActivityLog links
    IdToActivityLog,             // Anchor(log_id) -> ActivityLog
    SubjectToActivityLog,        // Anchor(subject_id) -> ActivityLog
    SessionToActivityLog,        // Anchor(session_id) -> ActivityLog

    // Renewal protocol links
    IdToRenewalAttestation,          // Anchor(renewal_id) -> RenewalAttestation
    HumanToRenewalAttestation,       // Anchor(human_id) -> RenewalAttestation
    OldAgentToRetirement,            // Anchor(retired_agent_key) -> AgentRetirement
    NewAgentFromRetirement,          // Anchor(renewed_into_key) -> AgentRetirement
    IdToAgentRetirement,             // Anchor(retirement_id) -> AgentRetirement
    IdToRelationshipRenewal,         // Anchor(rel_renewal_id) -> RelationshipRenewal
    OriginalRelToRenewal,            // Anchor(original_relationship_id) -> RelationshipRenewal
    RenewalAttestationByStatus,      // Anchor(renewal_status) -> RenewalAttestation
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
                EntryTypes::RecoveryRequest(request) => validate_recovery_request(&request),
                EntryTypes::RecoveryVote(vote) => validate_recovery_vote(&vote),
                EntryTypes::RecoveryHint(hint) => validate_recovery_hint(&hint),
                // Network-Attested Identity validation (Phase 2)
                EntryTypes::HumanityWitness(witness) => validate_humanity_witness(&witness),
                EntryTypes::KeyStewardship(stewardship) => validate_key_stewardship(&stewardship),
                EntryTypes::IdentityAnomaly(anomaly) => validate_identity_anomaly(&anomaly),
                EntryTypes::IdentityChallenge(challenge) => validate_identity_challenge(&challenge),
                EntryTypes::ChallengeSupport(support) => validate_challenge_support(&support),
                EntryTypes::KeyRevocation(revocation) => validate_key_revocation(&revocation),
                EntryTypes::RevocationVote(vote) => validate_revocation_vote(&vote),
                EntryTypes::IdentityFreeze(freeze) => validate_identity_freeze(&freeze),
                // Stewardship entry validation
                EntryTypes::StewardshipGrant(grant) => validate_stewardship_grant(&grant),
                EntryTypes::DevicePolicy(policy) => validate_device_policy(&policy),
                EntryTypes::PolicyInheritance(inheritance) => validate_policy_inheritance(&inheritance),
                EntryTypes::StewardshipAppeal(appeal) => validate_stewardship_appeal(&appeal),
                EntryTypes::ActivityLog(log) => validate_activity_log(&log),
                // Renewal protocol validation
                EntryTypes::RenewalAttestation(attestation) => validate_renewal_attestation(&attestation),
                EntryTypes::AgentRetirement(retirement) => validate_agent_retirement(&retirement),
                EntryTypes::RelationshipRenewal(renewal) => validate_relationship_renewal(&renewal),
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

// =============================================================================
// Recovery Validation Functions
// =============================================================================

/// Validate RecoveryRequest entry
fn validate_recovery_request(request: &RecoveryRequest) -> ExternResult<ValidateCallbackResult> {
    if request.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryRequest ID cannot be empty".to_string(),
        ));
    }

    if request.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryRequest human_id cannot be empty".to_string(),
        ));
    }

    if request.doorway_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryRequest doorway_id cannot be empty".to_string(),
        ));
    }

    if !RECOVERY_METHODS.contains(&request.recovery_method.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid recovery_method '{}'. Must be one of: {:?}",
            request.recovery_method, RECOVERY_METHODS
        )));
    }

    if !RECOVERY_STATUSES.contains(&request.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid status '{}'. Must be one of: {:?}",
            request.status, RECOVERY_STATUSES
        )));
    }

    if request.confidence_score < 0.0 || request.confidence_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "confidence_score must be between 0.0 and 1.0".to_string(),
        ));
    }

    // Minimum required approvals is 2
    if request.required_approvals < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "required_approvals must be at least 2 for security".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate RecoveryVote entry
fn validate_recovery_vote(vote: &RecoveryVote) -> ExternResult<ValidateCallbackResult> {
    if vote.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryVote ID cannot be empty".to_string(),
        ));
    }

    if vote.request_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryVote request_id cannot be empty".to_string(),
        ));
    }

    if vote.voter_human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryVote voter_human_id cannot be empty".to_string(),
        ));
    }

    if !INTIMACY_LEVELS.contains(&vote.intimacy_level.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid intimacy_level '{}'. Must be one of: {:?}",
            vote.intimacy_level, INTIMACY_LEVELS
        )));
    }

    if vote.confidence_weight < 0.0 || vote.confidence_weight > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "confidence_weight must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate RecoveryHint entry
fn validate_recovery_hint(hint: &RecoveryHint) -> ExternResult<ValidateCallbackResult> {
    if hint.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryHint ID cannot be empty".to_string(),
        ));
    }

    if hint.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryHint human_id cannot be empty".to_string(),
        ));
    }

    if !RECOVERY_HINT_TYPES.contains(&hint.hint_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid hint_type '{}'. Must be one of: {:?}",
            hint.hint_type, RECOVERY_HINT_TYPES
        )));
    }

    if hint.encrypted_data.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryHint encrypted_data cannot be empty".to_string(),
        ));
    }

    if hint.encryption_nonce.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryHint encryption_nonce cannot be empty".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

// =============================================================================
// Network-Attested Identity Validation Functions (Phase 2)
// =============================================================================

/// Validate HumanityWitness entry
fn validate_humanity_witness(witness: &HumanityWitness) -> ExternResult<ValidateCallbackResult> {
    if witness.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "HumanityWitness ID cannot be empty".to_string(),
        ));
    }

    if witness.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "HumanityWitness human_id cannot be empty".to_string(),
        ));
    }

    if witness.witness_agent_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "HumanityWitness witness_agent_id cannot be empty".to_string(),
        ));
    }

    // Cannot witness yourself
    if witness.human_id == witness.witness_agent_id {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot witness your own identity".to_string(),
        ));
    }

    if !ATTESTATION_TYPES.contains(&witness.attestation_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid attestation_type '{}'. Must be one of: {:?}",
            witness.attestation_type, ATTESTATION_TYPES
        )));
    }

    if witness.confidence < 0.0 || witness.confidence > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "confidence must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate KeyStewardship entry
fn validate_key_stewardship(stewardship: &KeyStewardship) -> ExternResult<ValidateCallbackResult> {
    if stewardship.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyStewardship ID cannot be empty".to_string(),
        ));
    }

    if stewardship.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyStewardship human_id cannot be empty".to_string(),
        ));
    }

    // Must have at least 2 shard holders for security
    if stewardship.key_shard_holders.len() < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyStewardship must have at least 2 shard holders".to_string(),
        ));
    }

    // M must be at least 2
    if stewardship.threshold_m < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "threshold_m must be at least 2 for security".to_string(),
        ));
    }

    // M must be <= N
    if stewardship.threshold_m > stewardship.total_shards_n {
        return Ok(ValidateCallbackResult::Invalid(
            "threshold_m cannot exceed total_shards_n".to_string(),
        ));
    }

    // N must match shard holders count
    if stewardship.total_shards_n as usize != stewardship.key_shard_holders.len() {
        return Ok(ValidateCallbackResult::Invalid(
            "total_shards_n must match number of shard holders".to_string(),
        ));
    }

    if !SIGNING_POLICIES.contains(&stewardship.signing_policy.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid signing_policy '{}'. Must be one of: {:?}",
            stewardship.signing_policy, SIGNING_POLICIES
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate IdentityAnomaly entry
fn validate_identity_anomaly(anomaly: &IdentityAnomaly) -> ExternResult<ValidateCallbackResult> {
    if anomaly.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityAnomaly ID cannot be empty".to_string(),
        ));
    }

    if anomaly.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityAnomaly human_id cannot be empty".to_string(),
        ));
    }

    if !ANOMALY_TYPES.contains(&anomaly.anomaly_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid anomaly_type '{}'. Must be one of: {:?}",
            anomaly.anomaly_type, ANOMALY_TYPES
        )));
    }

    if !ANOMALY_SEVERITIES.contains(&anomaly.severity.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid severity '{}'. Must be one of: {:?}",
            anomaly.severity, ANOMALY_SEVERITIES
        )));
    }

    if anomaly.deviation_score < 0.0 || anomaly.deviation_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "deviation_score must be between 0.0 and 1.0".to_string(),
        ));
    }

    if anomaly.evidence_json.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityAnomaly evidence_json cannot be empty".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate IdentityChallenge entry
fn validate_identity_challenge(challenge: &IdentityChallenge) -> ExternResult<ValidateCallbackResult> {
    if challenge.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityChallenge ID cannot be empty".to_string(),
        ));
    }

    if challenge.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityChallenge human_id cannot be empty".to_string(),
        ));
    }

    if challenge.initiator_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityChallenge initiator_id cannot be empty".to_string(),
        ));
    }

    // Cannot challenge yourself
    if challenge.human_id == challenge.initiator_id {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot challenge your own identity".to_string(),
        ));
    }

    if !CHALLENGE_TYPES.contains(&challenge.challenge_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid challenge_type '{}'. Must be one of: {:?}",
            challenge.challenge_type, CHALLENGE_TYPES
        )));
    }

    if !CHALLENGE_STATUSES.contains(&challenge.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid status '{}'. Must be one of: {:?}",
            challenge.status, CHALLENGE_STATUSES
        )));
    }

    if challenge.freeze_threshold <= 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "freeze_threshold must be positive".to_string(),
        ));
    }

    if challenge.revoke_threshold <= challenge.freeze_threshold {
        return Ok(ValidateCallbackResult::Invalid(
            "revoke_threshold must be greater than freeze_threshold".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate ChallengeSupport entry
fn validate_challenge_support(support: &ChallengeSupport) -> ExternResult<ValidateCallbackResult> {
    if support.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ChallengeSupport ID cannot be empty".to_string(),
        ));
    }

    if support.challenge_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ChallengeSupport challenge_id cannot be empty".to_string(),
        ));
    }

    if support.supporter_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ChallengeSupport supporter_id cannot be empty".to_string(),
        ));
    }

    if support.weight < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "weight cannot be negative".to_string(),
        ));
    }

    if !INTIMACY_LEVELS.contains(&support.intimacy_level.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid intimacy_level '{}'. Must be one of: {:?}",
            support.intimacy_level, INTIMACY_LEVELS
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate KeyRevocation entry
fn validate_key_revocation(revocation: &KeyRevocation) -> ExternResult<ValidateCallbackResult> {
    if revocation.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRevocation ID cannot be empty".to_string(),
        ));
    }

    if revocation.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRevocation human_id cannot be empty".to_string(),
        ));
    }

    if revocation.revoked_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRevocation revoked_key cannot be empty".to_string(),
        ));
    }

    if !REVOCATION_REASONS.contains(&revocation.reason.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid reason '{}'. Must be one of: {:?}",
            revocation.reason, REVOCATION_REASONS
        )));
    }

    // Minimum required votes is 2
    if revocation.required_votes < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "required_votes must be at least 2 for security".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate RevocationVote entry
fn validate_revocation_vote(vote: &RevocationVote) -> ExternResult<ValidateCallbackResult> {
    if vote.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RevocationVote ID cannot be empty".to_string(),
        ));
    }

    if vote.revocation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RevocationVote revocation_id cannot be empty".to_string(),
        ));
    }

    if vote.steward_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RevocationVote steward_id cannot be empty".to_string(),
        ));
    }

    if vote.attestation.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RevocationVote attestation cannot be empty (must provide reason)".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate IdentityFreeze entry
fn validate_identity_freeze(freeze: &IdentityFreeze) -> ExternResult<ValidateCallbackResult> {
    if freeze.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityFreeze ID cannot be empty".to_string(),
        ));
    }

    if freeze.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityFreeze human_id cannot be empty".to_string(),
        ));
    }

    if !FREEZE_TYPES.contains(&freeze.freeze_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid freeze_type '{}'. Must be one of: {:?}",
            freeze.freeze_type, FREEZE_TYPES
        )));
    }

    if freeze.frozen_capabilities.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IdentityFreeze must freeze at least one capability".to_string(),
        ));
    }

    // Validate all frozen capabilities
    for cap in &freeze.frozen_capabilities {
        if !FREEZABLE_CAPABILITIES.contains(&cap.as_str()) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "Invalid frozen capability '{}'. Must be one of: {:?}",
                cap, FREEZABLE_CAPABILITIES
            )));
        }
    }

    if !ANOMALY_SEVERITIES.contains(&freeze.severity.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid severity '{}'. Must be one of: {:?}",
            freeze.severity, ANOMALY_SEVERITIES
        )));
    }

    if !UNFREEZE_REQUIREMENTS.contains(&freeze.requires_verification.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid requires_verification '{}'. Must be one of: {:?}",
            freeze.requires_verification, UNFREEZE_REQUIREMENTS
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

// =============================================================================
// Renewal Protocol Validation Functions
// =============================================================================

/// Validate RenewalAttestation entry
fn validate_renewal_attestation(attestation: &RenewalAttestation) -> ExternResult<ValidateCallbackResult> {
    if attestation.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RenewalAttestation ID cannot be empty".to_string(),
        ));
    }

    if attestation.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RenewalAttestation human_id cannot be empty".to_string(),
        ));
    }

    if attestation.old_agent_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RenewalAttestation old_agent_key cannot be empty".to_string(),
        ));
    }

    if attestation.new_agent_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RenewalAttestation new_agent_key cannot be empty".to_string(),
        ));
    }

    if attestation.old_agent_key == attestation.new_agent_key {
        return Ok(ValidateCallbackResult::Invalid(
            "old_agent_key and new_agent_key must be different".to_string(),
        ));
    }

    if !RENEWAL_REASONS.contains(&attestation.renewal_reason.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid renewal_reason '{}'. Must be one of: {:?}",
            attestation.renewal_reason, RENEWAL_REASONS
        )));
    }

    if !RENEWAL_STATUSES.contains(&attestation.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid status '{}'. Must be one of: {:?}",
            attestation.status, RENEWAL_STATUSES
        )));
    }

    if attestation.required_approvals < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "required_approvals must be at least 2 for security".to_string(),
        ));
    }

    if attestation.confidence_score < 0.0 || attestation.confidence_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "confidence_score must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate AgentRetirement entry
fn validate_agent_retirement(retirement: &AgentRetirement) -> ExternResult<ValidateCallbackResult> {
    if retirement.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AgentRetirement ID cannot be empty".to_string(),
        ));
    }

    if retirement.human_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AgentRetirement human_id cannot be empty".to_string(),
        ));
    }

    if retirement.retired_agent_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AgentRetirement retired_agent_key cannot be empty".to_string(),
        ));
    }

    if retirement.renewed_into_agent_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AgentRetirement renewed_into_agent_key cannot be empty".to_string(),
        ));
    }

    if retirement.retired_agent_key == retirement.renewed_into_agent_key {
        return Ok(ValidateCallbackResult::Invalid(
            "retired_agent_key and renewed_into_agent_key must be different".to_string(),
        ));
    }

    if retirement.renewal_attestation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AgentRetirement renewal_attestation_id cannot be empty".to_string(),
        ));
    }

    if !RENEWAL_REASONS.contains(&retirement.retirement_reason.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid retirement_reason '{}'. Must be one of: {:?}",
            retirement.retirement_reason, RENEWAL_REASONS
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate RelationshipRenewal entry
fn validate_relationship_renewal(renewal: &RelationshipRenewal) -> ExternResult<ValidateCallbackResult> {
    if renewal.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RelationshipRenewal ID cannot be empty".to_string(),
        ));
    }

    if renewal.original_relationship_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RelationshipRenewal original_relationship_id cannot be empty".to_string(),
        ));
    }

    if renewal.renewal_attestation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "RelationshipRenewal renewal_attestation_id cannot be empty".to_string(),
        ));
    }

    if renewal.human_id.is_empty() || renewal.counterparty_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Both human_id and counterparty_id are required".to_string(),
        ));
    }

    if renewal.human_id == renewal.counterparty_id {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot renew relationship with self".to_string(),
        ));
    }

    if !HUMAN_RELATIONSHIP_TYPES.contains(&renewal.relationship_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid relationship_type '{}'. Must be one of: {:?}",
            renewal.relationship_type, HUMAN_RELATIONSHIP_TYPES
        )));
    }

    if !INTIMACY_LEVELS.contains(&renewal.intimacy_level.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid intimacy_level '{}'. Must be one of: {:?}",
            renewal.intimacy_level, INTIMACY_LEVELS
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}
