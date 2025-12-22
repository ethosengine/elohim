//! Content Store Integrity Zome
//!
//! Defines entry and link types for content storage, learning paths, and relationships.
//! This is the single source of truth for all Elohim domain models.
//!
//! Domain Pillars:
//! - Protocol: Core constants (ReachLevel, MasteryLevel, etc.)
//! - Lamad: Content and learning (Content, LearningPath, ContentMastery)
//! - Imago Dei: Identity (Agent, AgentProgress, Attestation)
//! - Qahal: Community (Relationship, Human relationships)
//! - Shefa: Economy (EconomicEvent, EconomicResource, ContributorPresence, Process, etc.)

use hdi::prelude::*;

// =============================================================================
// Protocol Constants (enums as string arrays for msgpack compatibility)
// =============================================================================

/// Reach levels - graduated visibility in the network
/// Determines who can see content/profiles
pub const REACH_LEVELS: [&str; 8] = [
    "private",    // Only self
    "self",       // Only self (alias)
    "intimate",   // Closest relationships
    "trusted",    // Trusted circle
    "familiar",   // Extended network
    "community",  // Community members
    "public",     // Anyone authenticated
    "commons",    // Anyone, including anonymous
];

/// Mastery levels - Bloom's Taxonomy based progression
/// Maps to 0-7 numeric values for comparison
pub const MASTERY_LEVELS: [&str; 8] = [
    "not_started", // 0 - No engagement
    "seen",        // 1 - Content viewed
    "remember",    // 2 - Basic recall demonstrated
    "understand",  // 3 - Comprehension demonstrated
    "apply",       // 4 - Application in novel contexts (ATTESTATION GATE)
    "analyze",     // 5 - Can break down, connect, contribute analysis
    "evaluate",    // 6 - Can assess, critique, peer review
    "create",      // 7 - Can author, derive, synthesize
];

/// The level index at which participation privileges unlock (apply = 4)
pub const ATTESTATION_GATE_LEVEL: usize = 4;

/// Agent types in the network
pub const AGENT_TYPES: [&str; 4] = [
    "human",        // Human individual
    "organization", // Organization or group
    "ai-agent",     // AI assistant
    "elohim",       // Constitutional AI agent
];

/// Content types supported in Lamad
pub const CONTENT_TYPES: [&str; 12] = [
    "epic",        // High-level narrative/vision document
    "concept",     // Atomic knowledge unit
    "lesson",      // Digestible learning session (AI-derived from concepts)
    "scenario",    // Gherkin feature/scenario
    "assessment",  // Quiz or test
    "resource",    // Supporting material
    "reflection",  // Journaling/reflection prompt
    "discussion",  // Discussion topic
    "exercise",    // Practice activity
    "example",     // Illustrative example
    "reference",   // Reference material
    "article",     // Long-form article content
];

/// Content format types
pub const CONTENT_FORMATS: [&str; 6] = [
    "markdown",
    "html",
    "video",
    "audio",
    "interactive",
    "external",
];

/// Path visibility types
pub const PATH_VISIBILITIES: [&str; 4] = [
    "private",   // Only creator
    "unlisted",  // Accessible by link
    "community", // Community members
    "public",    // Anyone
];

/// Engagement types for mastery tracking
pub const ENGAGEMENT_TYPES: [&str; 8] = [
    "view",       // Passive viewing
    "quiz",       // Took assessment
    "practice",   // Practice exercise
    "comment",    // Added comment/discussion
    "review",     // Peer reviewed content
    "contribute", // Made contribution
    "path_step",  // Encountered in learning path
    "refresh",    // Explicit refresh engagement
];

/// Attestation categories
pub const ATTESTATION_CATEGORIES: [&str; 4] = [
    "domain-mastery",   // Earned via sustained concept mastery
    "path-completion",  // All concepts at impression+ level
    "role-credential",  // Granted by governance process
    "achievement",      // One-time participation recognition
];

// =============================================================================
// hREA Point System - Value Flow Demonstration
// =============================================================================

/// Point triggers - actions that earn points (hREA EconomicEvent triggers)
pub const POINT_TRIGGERS: [&str; 10] = [
    "engagement_view",       // Viewing content
    "engagement_practice",   // Practicing content
    "challenge_correct",     // Correct answer in mastery challenge
    "challenge_complete",    // Completing a mastery challenge
    "level_up",              // Leveling up mastery
    "level_down",            // Losing mastery (negative points)
    "discovery",             // Discovering serendipitous content
    "path_step_complete",    // Completing a learning path step
    "path_complete",         // Completing entire learning path
    "contribution",          // Contributing to content
];

/// Point amounts for each trigger (can be configured)
/// Demonstrates hREA resourceQuantity
pub const DEFAULT_POINT_AMOUNTS: [(& str, i32); 10] = [
    ("engagement_view", 1),
    ("engagement_practice", 2),
    ("challenge_correct", 5),
    ("challenge_complete", 10),
    ("level_up", 20),
    ("level_down", -10),      // Negative - losing points
    ("discovery", 15),
    ("path_step_complete", 5),
    ("path_complete", 100),
    ("contribution", 50),
];

/// Resource specifications for the point system (hREA ResourceSpecification)
pub const POINT_RESOURCE_SPECS: [&str; 3] = [
    "learning-points",      // Points earned by learners
    "recognition-points",   // Points accumulated by contributors
    "impact-points",        // Aggregate impact across content
];

/// Recognition flow types - how value flows to contributors (hREA Appreciation)
pub const RECOGNITION_FLOW_TYPES: [&str; 4] = [
    "content_engagement",   // Someone engaged with your content
    "content_mastery",      // Someone mastered your content
    "path_completion",      // Someone completed a path with your content
    "discovery_spark",      // Your content sparked a discovery
];

// =============================================================================
// Shefa Protocol Constants
// =============================================================================

/// REA Action vocabulary - what happened in an economic event
/// Aligned with ValueFlows specification: https://www.valueflo.ws/concepts/actions/
pub const REA_ACTIONS: [&str; 20] = [
    // Input actions (consume/use resources)
    "use",              // Use without consuming (view content, attend session)
    "consume",          // Use up completely (one-time access tokens)
    "cite",             // Reference another's work (creates recognition flow)
    // Output actions (create/produce resources)
    "produce",          // Create new resource (author content, synthesize map)
    "raise",            // Increase quantity (accumulate recognition)
    "lower",            // Decrease quantity (reduce holdings)
    // Transfer actions (move between agents)
    "transfer",         // Move resource to another agent (ownership transfer)
    "transfer-custody", // Move custody without ownership change
    "transfer-all-rights", // Transfer all rights to resource
    "move",             // Move resource between locations (same agent)
    // Modification actions
    "modify",           // Change resource properties
    "combine",          // Merge resources (path extensions into base path)
    "separate",         // Split resource (fork a learning path)
    // Work actions
    "work",             // Contribute labor (stewardship, review, curation)
    "deliver-service",  // Provide service (Elohim synthesis, tutoring)
    // Logistics actions
    "pickup",           // Take custody of a resource at a location
    "dropoff",          // Release custody of a resource at a location
    // Exchange actions
    "give",             // Give a resource (one side of exchange)
    "take",             // Take a resource (other side of exchange)
    // Acceptance
    "accept",           // Accept a transfer or commitment (claim presence)
];

/// Resource classifications for Shefa economy
pub const RESOURCE_CLASSIFICATIONS: [&str; 16] = [
    // Lamad content classifications
    "content",          // Learning content (epics, features, scenarios)
    "attention",        // Human attention/engagement
    "recognition",      // Attestation of value/contribution
    "credential",       // Earned capabilities/attestations
    "curation",         // Curated collections/paths
    "synthesis",        // AI-generated maps/analysis
    "stewardship",      // Care/maintenance of presences
    "membership",       // Network participation rights
    "compute",          // Computational resources (for Elohim)
    "currency",         // Mutual credit (Unyt/HoloFuel integration)
    // Shefa token classifications
    "care-token",       // Witnessed caregiving acts
    "time-token",       // Hours contributed to community
    "learning-token",   // Skills developed and taught
    "steward-token",    // Environmental/resource protection
    "creator-token",    // Content that helps others
    "infrastructure-token", // Network maintenance contribution
];

/// Lamad-specific event types for contextual tracking
pub const LAMAD_EVENT_TYPES: [&str; 27] = [
    // Attention Events
    "content-view",       // Human viewed content
    "path-step-complete", // Human completed a step
    "session-start",      // Human began a session
    "session-end",        // Human ended a session
    // Recognition Events
    "affinity-mark",      // Human marked affinity
    "endorsement",        // Formal endorsement
    "citation",           // Content cited another
    // Achievement Events
    "path-complete",      // Human completed a path
    "attestation-grant",  // Attestation granted
    "capability-earn",    // Capability developed
    // Creation Events
    "content-create",     // Content created
    "path-create",        // Path created
    "extension-create",   // Path extension created
    // Synthesis Events
    "map-synthesis",      // Elohim synthesized map
    "analysis-complete",  // Elohim completed analysis
    // Stewardship Events
    "stewardship-begin",  // Elohim began stewardship
    "invitation-send",    // Invitation sent
    "presence-claim",     // Contributor claimed presence
    "recognition-transfer", // Recognition transferred
    // Governance Events
    "attestation-revoke", // Attestation revoked
    "content-flag",       // Content flagged
    "governance-vote",    // Governance vote cast
    // Currency Events (Unyt integration)
    "credit-issue",       // Mutual credit issued
    "credit-transfer",    // Credit transferred
    "credit-retire",      // Credit retired
    // Process Events
    "process-start",      // Process begun
    "process-complete",   // Process finished
];

/// Economic event states
pub const EVENT_STATES: [&str; 5] = [
    "pending",       // Created but not yet validated
    "validated",     // Passed validation rules
    "countersigned", // Both parties have signed (for transfers)
    "disputed",      // Under dispute
    "corrected",     // Superseded by correction event
];

/// Economic resource states
pub const RESOURCE_STATES: [&str; 6] = [
    "available",     // Ready to use/transfer
    "committed",     // Promised but not yet transferred
    "in-use",        // Currently being used
    "consumed",      // Fully consumed
    "archived",      // No longer active
    "disputed",      // Under review/appeal
];

/// Contributor presence lifecycle states
pub const PRESENCE_STATES: [&str; 3] = [
    "unclaimed",     // Presence exists, no steward assigned
    "stewarded",     // Elohim actively stewarding, recognition accumulating
    "claimed",       // Contributor has verified identity and claimed presence
];

/// Process states for value-creating transformations
pub const PROCESS_STATES: [&str; 4] = [
    "not-started",   // Defined but not begun
    "in-progress",   // Currently active
    "finished",      // Successfully completed
    "abandoned",     // Stopped without completion
];

/// Commitment states for future economic activity
pub const COMMITMENT_STATES: [&str; 6] = [
    "proposed",      // Offered but not accepted
    "accepted",      // Accepted by receiver
    "in-progress",   // Being fulfilled
    "fulfilled",     // Fully satisfied
    "cancelled",     // Cancelled before fulfillment
    "breached",      // Failed to fulfill
];

/// Claim verification methods for contributor presence claiming
pub const CLAIM_VERIFICATION_METHODS: [&str; 8] = [
    "domain-verification",   // Prove control of website domain
    "social-verification",   // Prove control of social account
    "email-verification",    // Prove control of email
    "orcid-verification",    // ORCID authentication
    "github-verification",   // GitHub authentication
    "publisher-attestation", // Publisher vouches for identity
    "community-vouching",    // Community members vouch
    "cryptographic-proof",   // Existing cryptographic identity
];

/// Invitation channels for presence outreach
pub const INVITATION_CHANNELS: [&str; 7] = [
    "email",           // Direct email
    "twitter",         // Twitter DM or mention
    "github",          // GitHub issue/discussion
    "website-contact", // Contact form on contributor's website
    "orcid",           // ORCID messaging
    "publisher",       // Through publisher
    "community",       // Through mutual community connection
];

/// Value flow governance layers
pub const VALUE_FLOW_LAYERS: [&str; 4] = [
    "dignity_floor",  // Layer 1: Existential minimums
    "attribution",    // Layer 2: Contribution recognition
    "circulation",    // Layer 3: Community velocity
    "sustainability", // Layer 4: Network development
];

// =============================================================================
// CustodianCommitment Constants (Imago Dei - Digital Presence Stewardship)
// =============================================================================

/// Commitment types - why this commitment exists
pub const COMMITMENT_TYPES: [&str; 4] = [
    "relationship",     // Based on HumanRelationship (primary path)
    "category",         // Category override (medical, high-bandwidth, etc.)
    "community",        // M-of-N community custody
    "steward",          // ContributorPresence steward custody
];

/// Commitment basis - selection criteria
pub const COMMITMENT_BASIS: [&str; 6] = [
    "intimate_relationship",    // IntimacyLevel >= trusted (family, close friends)
    "trusted_relationship",     // IntimacyLevel == trusted
    "community_member",         // Same community/neighborhood reach
    "category_specialist",      // Has credential for content category
    "bandwidth_capacity",       // High bandwidth for large files
    "geographic_proximity",     // Close geographic location
];

/// Shard strategy - how content is split for resilience
pub const SHARD_STRATEGIES: [&str; 3] = [
    "full_replica",      // Each custodian holds complete copy (small content)
    "threshold_split",   // M-of-N Shamir's Secret Sharing
    "erasure_coded",     // Reed-Solomon erasure coding (efficient for large files)
];

/// Emergency trigger types - what can activate emergency protocol
pub const EMERGENCY_TRIGGERS: [&str; 5] = [
    "manual_signal",           // Beneficiary manually activates via passphrase
    "trusted_party",           // Designated trusted person activates
    "m_of_n_consensus",        // M custodians vote to activate
    "dead_mans_switch",        // Beneficiary hasn't checked in for N days
    "beneficiary_incapacity",  // Medical/legal declaration of incapacity
];

/// Bandwidth classes for performance hints
pub const BANDWIDTH_CLASSES: [&str; 4] = [
    "low",      // <5 Mbps
    "medium",   // 5-50 Mbps
    "high",     // 50-500 Mbps
    "ultra",    // >500 Mbps
];

/// Category override types (specialists who custody content outside relationship reach)
pub const CATEGORY_OVERRIDE_TYPES: [&str; 5] = [
    "medical",           // Healthcare providers (doctors, clinicians)
    "emergency",         // Emergency services (fire, police, rescue)
    "disaster_relief",   // Disaster response (FEMA, Red Cross, etc.)
    "high_bandwidth",    // Content delivery specialists (video, media)
    "archive",           // Long-term preservation (librarians, archivists)
];

/// Category override access levels
pub const CATEGORY_ACCESS_LEVELS: [&str; 4] = [
    "professional",      // Licensed/credentialed access only
    "trusted",          // Pre-vetted specialists
    "verified",         // Verified identity + background check
    "emergency_only",   // Access granted only during declared emergency
];

// =============================================================================
// Content Entry
// =============================================================================

/// Content entry representing a content node (matches ContentNode from elohim-service)
/// This is the atomic unit of knowledge in the Lamad system.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Content {
    pub id: String,
    pub content_type: String,            // epic, concept, lesson, scenario, assessment, etc.
    pub title: String,
    pub description: String,
    pub summary: Option<String>,         // Short preview text for cards/lists (AI-generated)
    pub content: String,
    pub content_format: String,          // markdown, html, video, audio, interactive, external
    pub tags: Vec<String>,
    pub source_path: Option<String>,     // Original source file path if imported
    pub related_node_ids: Vec<String>,   // Quick graph references (denormalized)
    pub author_id: Option<String>,
    pub reach: String,                   // Visibility: private, community, public, commons
    pub trust_score: f64,
    // Attention metadata for digestible learning sessions
    pub estimated_minutes: Option<u32>,  // Reading/viewing time
    pub thumbnail_url: Option<String>,   // Preview image for visual cards
    // Extensible metadata
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Learning Path Entries
// =============================================================================

/// Learning path entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct LearningPath {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub purpose: Option<String>,
    pub created_by: String,
    pub difficulty: String,
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    pub tags: Vec<String>,
    /// Extensible metadata (stores chapters JSON for hierarchical paths)
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Path step entry - represents a single learning activity
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PathStep {
    pub id: String,
    pub path_id: String,
    pub chapter_id: Option<String>,      // If part of a chapter
    pub order_index: u32,
    pub step_type: String,               // content, path, external, checkpoint, reflection
    pub resource_id: String,             // Content.id, nested LearningPath.id, or external URL
    pub step_title: Option<String>,
    pub step_narrative: Option<String>,
    pub is_optional: bool,
    // Learning objectives and engagement
    pub learning_objectives_json: String, // String[] as JSON
    pub reflection_prompts_json: String,  // String[] as JSON
    pub practice_exercises_json: String,  // String[] as JSON
    // Completion and gating
    pub estimated_minutes: Option<u32>,
    pub completion_criteria: Option<String>, // view, quiz_pass, practice_complete, reflection_submit
    pub attestation_required: Option<String>, // Required attestation to access this step
    pub attestation_granted: Option<String>,  // Attestation granted on completing this step
    pub mastery_threshold: Option<u32>,       // Minimum mastery level (0-7) to proceed
    // Metadata
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Step types for PathStep.step_type
pub const STEP_TYPES: [&str; 5] = [
    "content",      // Regular content node
    "path",         // Nested learning path (composition)
    "external",     // External resource (URL)
    "checkpoint",   // Assessment/quiz checkpoint
    "reflection",   // Reflection/journaling prompt
];

/// Completion criteria for PathStep.completion_criteria
pub const COMPLETION_CRITERIA: [&str; 5] = [
    "view",             // Just view the content
    "quiz_pass",        // Pass associated quiz
    "practice_complete", // Complete practice exercises
    "reflection_submit", // Submit reflection
    "time_spent",       // Spend minimum time
];

/// Path chapter entry - thematic grouping of steps
/// Named "chapter" to evoke narrative journey rather than institutional "module"
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PathChapter {
    pub id: String,
    pub path_id: String,
    pub order_index: u32,
    pub title: String,
    pub description: Option<String>,
    pub learning_objectives_json: String,  // String[] as JSON
    pub estimated_minutes: Option<u32>,
    pub is_optional: bool,
    // Completion tracking
    pub attestation_granted: Option<String>, // Milestone attestation on chapter completion
    pub mastery_threshold: Option<u32>,      // Required mastery level to complete chapter
    // Metadata
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Content Relationship Entry
// =============================================================================

/// Relationship types for content graph edges
pub const RELATIONSHIP_TYPES: [&str; 6] = [
    "RELATES_TO",   // General association between concepts
    "CONTAINS",     // Parent-child hierarchical relationship
    "DEPENDS_ON",   // Prerequisite dependency (must understand first)
    "IMPLEMENTS",   // Implementation of a concept
    "REFERENCES",   // Citation or reference to another concept
    "DERIVED_FROM", // This content was derived from source content
];

/// Inference sources for relationships
pub const INFERENCE_SOURCES: [&str; 4] = [
    "explicit",   // Manually created by author/curator
    "path",       // Inferred from learning path structure
    "tag",        // Inferred from shared tags
    "semantic",   // Inferred by AI from content similarity
];

/// Relationship between two content nodes (stored in DHT, replaces Kuzu)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Relationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,  // See RELATIONSHIP_TYPES
    pub confidence: f64,            // 0.0 - 1.0
    pub inference_source: String,   // See INFERENCE_SOURCES
    pub metadata_json: Option<String>,
    pub created_at: String,
}

// =============================================================================
// Human/Agent Entry (for Elohim network)
// =============================================================================

/// Human agent in the Elohim network
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Human {
    pub id: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub affinities: Vec<String>,      // Topics/areas of interest
    pub profile_reach: String,        // public, community, private
    pub location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Progress tracking for a human on a learning path
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

// =============================================================================
// Imago Dei: Agent Entry (expanded identity model)
// =============================================================================

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
    pub agent_type: String,           // human, organization, ai-agent, elohim
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar: Option<String>,
    pub affinities: Vec<String>,      // Topics/areas of interest
    pub visibility: String,           // public, connections, private
    pub location: Option<String>,
    pub holochain_agent_key: Option<String>, // AgentPubKey as base64
    pub did: Option<String>,          // W3C DID (did:web:elohim.host:agents:{id})
    pub activity_pub_type: Option<String>, // Person, Organization, Service, etc.
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
// Lamad: Content Mastery Entry
// =============================================================================

/// ContentMastery - A human's mastery state for a specific content node.
///
/// Implements Bloom's Taxonomy progression from passive consumption
/// to active contribution. This powers:
/// - Khan Academy-style cross-path completion views
/// - Participation privilege gating (apply level = attestation gate)
/// - Expertise discovery queries
/// - Freshness/decay tracking
///
/// Storage: Lives on agent's private source chain.
/// The full model with history lives locally; this is the DHT-storable summary.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContentMastery {
    pub id: String,
    pub human_id: String,
    pub content_id: String,
    pub mastery_level: String,        // MasteryLevel: not_started → create
    pub mastery_level_index: u32,     // 0-7 for comparison
    pub freshness_score: f64,         // 0.0-1.0, decays over time
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
}

/// Attestation - Permanent achievement record.
///
/// Attestations are PERMANENT ACHIEVEMENTS, not competency tracking.
/// ContentMastery handles graduated competency.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Attestation {
    pub id: String,
    pub agent_id: String,
    pub category: String,            // AttestationCategory
    pub attestation_type: String,    // Specific type within category
    pub display_name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub tier: Option<String>,        // bronze, silver, gold, platinum
    pub earned_via_json: String,     // EarnedVia details as JSON
    pub issued_at: String,
    pub issued_by: String,           // System, steward, governance, or peer
    pub expires_at: Option<String>,
    pub proof: Option<String>,       // Cryptographic signature
}

// =============================================================================
// Shefa: Economic Event Entry
// =============================================================================

/// EconomicEvent - An observed economic occurrence.
///
/// This is the core building block of REA accounting. Events are immutable
/// records of value flows between agents. They form the audit trail on source chains.
///
/// From the Economic Epic:
/// "REA doesn't ask 'how much money?' It asks 'what actually happened?'
/// Every event is recorded from what we call the 'independent view'—as
/// a transaction between parties rather than separate ledger entries."
///
/// Holochain mapping:
/// - Lives on agent's source chain (provider or receiver)
/// - Countersigned by both parties for transfers
/// - Immutable - corrections create new events, don't modify old ones
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EconomicEvent {
    pub id: String,
    pub action: String,              // REAAction from REA_ACTIONS
    pub provider: String,            // Agent ID - who provided/gave
    pub receiver: String,            // Agent ID - who received
    // Resource identification
    pub resource_conforms_to: Option<String>,  // ResourceSpecification.id
    pub resource_inventoried_as: Option<String>, // EconomicResource.id (provider side)
    pub to_resource_inventoried_as: Option<String>, // EconomicResource.id (receiver side for transfers)
    pub resource_classified_as_json: String,   // ResourceClassification[] as JSON
    // Quantities
    pub resource_quantity_value: Option<f64>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f64>,
    pub effort_quantity_unit: Option<String>,
    // Timing
    pub has_point_in_time: String,
    pub has_duration: Option<String>,
    // Process context
    pub input_of: Option<String>,    // Process.id
    pub output_of: Option<String>,   // Process.id
    // Commitment/Agreement context
    pub fulfills_json: String,       // Commitment.id[] as JSON
    pub realization_of: Option<String>, // Agreement.id
    pub satisfies_json: String,      // Intent.id[] as JSON
    pub in_scope_of_json: String,    // Accounting scopes as JSON
    // Metadata
    pub note: Option<String>,
    pub state: String,               // EventState from EVENT_STATES
    pub triggered_by: Option<String>, // EconomicEvent.id for corrections/chains
    pub at_location: Option<String>,
    pub image: Option<String>,
    pub lamad_event_type: Option<String>, // LamadEventType for domain-specific tracking
    pub metadata_json: String,       // Additional metadata as JSON
    pub created_at: String,
}

// =============================================================================
// Shefa: Economic Resource Entry
// =============================================================================

/// EconomicResource - A resource that can flow through the network.
///
/// In ValueFlows, resources are created and modified only through events.
/// This is the current state derived from event history.
///
/// Examples in Lamad:
/// - Content nodes as resources (substitutable: false)
/// - Recognition/affinity points (substitutable: true)
/// - Credentials/attestations (substitutable: false)
/// - Compute credits (substitutable: true)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EconomicResource {
    pub id: String,
    pub conforms_to: String,         // ResourceSpecification.id
    pub name: String,
    pub note: Option<String>,
    // Quantities
    pub accounting_quantity_value: Option<f64>,
    pub accounting_quantity_unit: Option<String>,
    pub onhand_quantity_value: Option<f64>,
    pub onhand_quantity_unit: Option<String>,
    // Ownership
    pub primary_accountable: String, // Agent ID - who owns this
    pub custodian: Option<String>,   // Agent ID - who has custody (may differ)
    // State
    pub state: Option<String>,       // ResourceState from RESOURCE_STATES
    pub classified_as_json: String,  // ResourceClassification[] as JSON
    // References
    pub tracking_identifier: Option<String>,
    pub image: Option<String>,
    pub content_node_id: Option<String>,  // For content resources
    pub attestation_id: Option<String>,   // For credential resources
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Shefa: Contributor Presence Entry
// =============================================================================

/// ContributorPresence - Stewardship lifecycle for absent contributors.
///
/// From the Manifesto (Part IV-B):
/// "When someone's work is referenced in Lamad—whether a book author, a video
/// contributor, or a researcher—a presence is established for them. This presence
/// is not an account they control (yet), but a place where recognition can accumulate."
///
/// Lifecycle: UNCLAIMED → STEWARDED → CLAIMED
///
/// This inverts traditional systems: recognition flows FIRST, invitation follows.
/// The Elohim steward these presences until contributors claim them.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContributorPresence {
    pub id: String,
    pub display_name: String,
    pub presence_state: String,      // PresenceState from PRESENCE_STATES
    // External identifiers for verification
    pub external_identifiers_json: String, // ExternalIdentifier[] as JSON
    // Establishing content
    pub establishing_content_ids_json: String, // String[] as JSON
    pub established_at: String,
    // Accumulated recognition
    pub affinity_total: f64,
    pub unique_engagers: u32,
    pub citation_count: u32,
    pub endorsements_json: String,   // PresenceEndorsement[] as JSON
    pub recognition_score: f64,
    pub recognition_by_content_json: String, // ContentRecognition[] as JSON
    pub accumulating_since: String,
    pub last_recognition_at: String,
    // Stewardship (if stewarded)
    pub steward_id: Option<String>,  // Elohim agent ID
    pub stewardship_started_at: Option<String>,
    pub stewardship_commitment_id: Option<String>, // Commitment.id
    pub stewardship_quality_score: Option<f64>,
    // Claim details (if claimed)
    pub claim_initiated_at: Option<String>,
    pub claim_verified_at: Option<String>,
    pub claim_verification_method: Option<String>, // ClaimVerificationMethod
    pub claim_evidence_json: Option<String>,       // ClaimEvidence[] as JSON
    pub claimed_agent_id: Option<String>,          // Verified Agent.id
    pub claim_recognition_transferred_value: Option<f64>,
    pub claim_recognition_transferred_unit: Option<String>,
    pub claim_facilitated_by: Option<String>,
    // Invitations
    pub invitations_json: String,    // PresenceInvitation[] as JSON
    // Metadata
    pub note: Option<String>,
    pub image: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Shefa: Process Entry
// =============================================================================

/// Process - A transformation that creates value.
///
/// In Lamad, learning paths are processes that transform
/// attention and effort into capabilities and recognition.
///
/// Examples:
/// - Learning journey: attention + effort → credential
/// - Content creation: effort → content resource
/// - Synthesis: content + compute → knowledge map
/// - Stewardship: effort → maintained presence
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Process {
    pub id: String,
    pub based_on: Option<String>,    // ProcessSpecification.id
    pub name: String,
    pub note: Option<String>,
    // Timing
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub finished: bool,
    pub state: String,               // ProcessState from PROCESS_STATES
    // Events
    pub inputs_json: String,         // EconomicEvent.id[] as JSON
    pub outputs_json: String,        // EconomicEvent.id[] as JSON
    // Scope
    pub in_scope_of_json: String,    // Accounting scopes as JSON
    // References
    pub path_id: Option<String>,     // For learning paths
    pub performed_by: Option<String>, // Agent ID (e.g., Elohim for synthesis)
    pub classified_as_json: String,  // ProcessClassification[] as JSON
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Shefa: Intent Entry
// =============================================================================

/// Intent - Expression of desired economic activity.
///
/// Intents are like requests/offers before they become commitments.
/// In Lamad: "I want to learn X", "I offer to review Y"
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Intent {
    pub id: String,
    pub name: Option<String>,
    pub action: String,              // REAAction
    pub provider: Option<String>,    // Agent ID (who will give - implies offer)
    pub receiver: Option<String>,    // Agent ID (who will receive - implies request)
    // Resource
    pub resource_conforms_to: Option<String>, // ResourceSpecification.id
    pub resource_inventoried_as: Option<String>, // EconomicResource.id
    pub resource_quantity_value: Option<f64>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f64>,
    pub effort_quantity_unit: Option<String>,
    pub available_quantity_value: Option<f64>,
    pub available_quantity_unit: Option<String>,
    // Timing
    pub has_point_in_time: Option<String>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub due: Option<String>,
    // Process
    pub input_of: Option<String>,    // Process.id
    pub output_of: Option<String>,   // Process.id
    // Scope
    pub in_scope_of_json: String,
    pub classified_as_json: String,
    // State
    pub finished: bool,
    pub note: Option<String>,
    pub image: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Shefa: Commitment Entry
// =============================================================================

/// Commitment - A promise of future economic activity.
///
/// Commitments are binding promises, stronger than intents.
/// In Lamad: "I commit to stewarding this presence"
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Commitment {
    pub id: String,
    pub action: String,              // REAAction
    pub provider: String,            // Agent ID
    pub receiver: String,            // Agent ID
    // Resource
    pub resource_conforms_to: Option<String>,
    pub resource_inventoried_as: Option<String>,
    pub resource_classified_as_json: String,
    pub resource_quantity_value: Option<f64>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f64>,
    pub effort_quantity_unit: Option<String>,
    // Timing
    pub has_point_in_time: Option<String>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub due: Option<String>,
    // Context
    pub clause_of: Option<String>,   // Agreement.id
    pub agreed_in: Option<String>,   // URI reference
    pub input_of: Option<String>,    // Process.id
    pub output_of: Option<String>,   // Process.id
    pub satisfies: Option<String>,   // Intent.id
    pub in_scope_of_json: String,
    // State
    pub finished: bool,
    pub state: String,               // CommitmentState from COMMITMENT_STATES
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Shefa: Appreciation Entry
// =============================================================================

/// Appreciation - Recognition of value created by another.
///
/// This is the core of Lamad's recognition economics. Appreciations flow
/// to creators (or their presences) when their work is used, cited, or endorsed.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Appreciation {
    pub id: String,
    pub appreciation_of: String,     // EconomicEvent.id that triggered this
    pub appreciated_by: String,      // Agent ID - who is giving appreciation
    pub appreciation_to: String,     // Agent ID - recipient (may be presence)
    pub quantity_value: f64,
    pub quantity_unit: String,
    pub note: Option<String>,
    pub created_at: String,
}

// =============================================================================
// Shefa: Claim Entry
// =============================================================================

/// Claim - A claim for a future economic event.
///
/// In hREA, Claims represent entitlements that arise from economic activity,
/// which can be settled by future economic events.
///
/// In Lamad: claims to recognition that has accumulated at a presence.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Claim {
    pub id: String,
    pub action: String,              // REAAction
    pub triggered_by: String,        // EconomicEvent.id
    pub provider: Option<String>,    // Agent ID - who has the claim
    pub receiver: Option<String>,    // Agent ID - against whom
    // Resource
    pub resource_conforms_to: Option<String>,
    pub resource_classified_as_json: String,
    pub resource_quantity_value: Option<f64>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f64>,
    pub effort_quantity_unit: Option<String>,
    // Context
    pub due: Option<String>,
    pub agreed_in: Option<String>,   // URI reference
    pub in_scope_of_json: String,
    // State
    pub finished: bool,
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Shefa: Settlement Entry
// =============================================================================

/// Settlement - Records the settlement of a claim.
///
/// In hREA, Settlements link Claims to the EconomicEvents that fulfill them.
/// A Claim can be partially or fully settled by multiple Settlements.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Settlement {
    pub id: String,
    pub settles: String,             // Claim.id
    pub settled_by: String,          // EconomicEvent.id
    pub resource_quantity_value: Option<f64>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f64>,
    pub effort_quantity_unit: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

// =============================================================================
// Lamad: Practice Pool & Mastery Challenges
// =============================================================================

/// Practice Pool - agent's current learning rotation with graph-aware discovery
/// Supports Khan Academy-style organic learning with serendipitous discoveries
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PracticePool {
    pub id: String,
    pub agent_id: String,
    /// Content currently in active rotation (from paths)
    pub active_content_ids_json: String,          // Vec<String> as JSON
    /// Content that's mastered but needs refresh (freshness dropped)
    pub refresh_queue_ids_json: String,           // Vec<String> as JSON
    /// Graph-discovered content for serendipity/surprise learning
    /// Format: [{content_id, source_content_id, relationship_type, discovery_reason}]
    pub discovery_candidates_json: String,
    /// Paths contributing to this pool
    pub contributing_path_ids_json: String,       // Vec<String> as JSON
    /// Pool settings
    pub max_active_size: u32,                     // Max items in active rotation
    pub refresh_threshold: f64,                   // Freshness below this → needs refresh (0.0-1.0)
    pub discovery_probability: f64,               // Chance of including surprise content (0.0-1.0)
    pub regression_enabled: bool,                 // Can mastery level go DOWN on failures?
    /// Challenge cooldown
    pub challenge_cooldown_hours: u32,            // Hours between mastery challenges
    pub last_challenge_at: Option<String>,        // When was last mastery challenge?
    pub last_challenge_id: Option<String>,        // ID of last challenge
    /// Statistics
    pub total_challenges_taken: u32,
    pub total_level_ups: u32,
    pub total_level_downs: u32,
    pub discoveries_unlocked: u32,                // Serendipitous content discovered
    /// Timestamps
    pub created_at: String,
    pub updated_at: String,
}

/// Mastery Challenge - a mixed assessment pulling from the practice pool
/// Supports level UP and DOWN, enforces cooldown, tracks discoveries
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MasteryChallenge {
    pub id: String,
    pub agent_id: String,
    pub pool_id: String,                          // PracticePool this challenge draws from
    pub path_id: Option<String>,                  // Optional - can be path-independent
    /// Challenge composition
    /// Format: [{content_id, source: "active"|"refresh"|"discovery", question_count}]
    pub content_mix_json: String,
    pub total_questions: u32,
    pub discovery_questions: u32,                 // How many were "surprise" content
    /// Challenge state
    pub state: String,                            // "in_progress", "completed", "abandoned"
    pub started_at: String,
    pub completed_at: Option<String>,
    pub time_limit_seconds: Option<u32>,          // Optional time limit
    pub actual_time_seconds: Option<u32>,
    /// Questions and responses
    /// Format: [{content_id, question_type, question_data, options}]
    pub questions_json: String,
    /// Format: [{content_id, response, correct, time_taken_ms}]
    pub responses_json: String,
    /// Results
    pub score: Option<f64>,                       // 0.0-1.0 overall score
    pub score_by_content_json: String,            // {content_id: score} per-content scores
    /// Level changes applied after completion
    /// Format: [{content_id, from_level, to_level, from_index, to_index, change: "up"|"down"|"same"}]
    pub level_changes_json: String,
    pub net_level_change: i32,                    // Sum of all level index changes
    /// Discoveries - content unlocked through serendipity
    /// Format: [{content_id, discovered_via, relationship_type}]
    pub discoveries_json: String,
    /// Metadata
    pub created_at: String,
}

/// Pool content source - where content came from
pub const POOL_SOURCES: [&str; 4] = [
    "path_active",    // Currently active in a learning path
    "refresh_queue",  // Mastered but needs refresh
    "graph_neighbor", // Related via knowledge graph
    "serendipity",    // Random discovery opportunity
];

/// Challenge states
pub const CHALLENGE_STATES: [&str; 3] = [
    "in_progress",
    "completed",
    "abandoned",
];

// =============================================================================
// Lamad: Knowledge Map Entry
// =============================================================================

/// KnowledgeMap - A personalized view of learnable territory.
///
/// Four relational dimensions:
/// 1. domain - Relationship with knowledge (Khan Academy style)
/// 2. self - Relationship with self ("know thyself")
/// 3. person - Relationship with others (Gottman Love Maps)
/// 4. collective - Relationship with communities
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct KnowledgeMap {
    pub id: String,
    pub map_type: String,           // domain, self, person, collective
    pub owner_id: String,
    pub title: String,
    pub description: Option<String>,
    // Subject being mapped
    pub subject_type: String,       // content-graph, agent, organization
    pub subject_id: String,
    pub subject_name: String,
    // Visibility
    pub visibility: String,         // private, mutual, shared, public
    pub shared_with_json: String,   // String[] as JSON
    // Map content
    pub nodes_json: String,         // KnowledgeNode[] as JSON
    pub path_ids_json: String,      // String[] as JSON
    pub overall_affinity: f64,
    // For domain maps
    pub content_graph_id: Option<String>,
    pub mastery_levels_json: String, // Record<string, string> as JSON
    pub goals_json: String,          // DomainGoal[] as JSON
    // Timestamps
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Knowledge map types
pub const KNOWLEDGE_MAP_TYPES: [&str; 4] = [
    "domain",
    "self",
    "person",
    "collective",
];

/// Knowledge map visibility levels
pub const KNOWLEDGE_MAP_VISIBILITY: [&str; 4] = [
    "private",
    "mutual",
    "shared",
    "public",
];

// =============================================================================
// Lamad: Path Extension Entry
// =============================================================================

/// PathExtension - Learner-owned mutations to curated paths.
///
/// Enables personal learning customization without fragmenting curation.
/// Extensions can insert steps, add annotations, reorder, or exclude steps.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PathExtension {
    pub id: String,
    pub base_path_id: String,
    pub base_path_version: String,
    pub extended_by: String,
    pub title: String,
    pub description: Option<String>,
    // Modifications (all as JSON for flexibility)
    pub insertions_json: String,      // PathStepInsertion[] as JSON
    pub annotations_json: String,     // PathStepAnnotation[] as JSON
    pub reorderings_json: String,     // PathStepReorder[] as JSON
    pub exclusions_json: String,      // PathStepExclusion[] as JSON
    // Visibility
    pub visibility: String,           // private, shared, public
    pub shared_with_json: String,     // String[] as JSON
    // Fork tracking
    pub forked_from: Option<String>,
    pub forks_json: String,           // String[] as JSON
    // Upstream proposal (propose changes back to maintainer)
    pub upstream_proposal_json: Option<String>,
    // Stats
    pub stats_json: String,           // ExtensionStats as JSON
    // Timestamps
    pub created_at: String,
    pub updated_at: String,
}

/// Extension visibility levels
pub const EXTENSION_VISIBILITY: [&str; 3] = [
    "private",
    "shared",
    "public",
];

// =============================================================================
// Governance Entry Types
// =============================================================================

/// Challenge - A formal challenge to content or decisions.
///
/// Enables community members with standing to challenge content quality,
/// accuracy, safety, or constitutional alignment.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Challenge {
    pub id: String,
    pub entity_type: String,          // content, path, extension, attestation, decision
    pub entity_id: String,
    pub challenger_id: String,
    pub challenger_name: String,
    pub challenger_standing: String,  // Attestation level granting standing
    pub grounds: String,              // factual-error, safety, policy, constitutional
    pub description: String,
    pub evidence_json: String,        // Evidence[] as JSON
    pub status: String,               // filed, acknowledged, under-review, resolved, dismissed
    pub filed_at: String,
    pub acknowledged_at: Option<String>,
    pub sla_deadline: Option<String>,
    pub assigned_elohim: Option<String>,
    pub priority: String,             // normal, high, critical
    pub resolution_json: Option<String>, // ChallengeResolution as JSON
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Challenge grounds
pub const CHALLENGE_GROUNDS: [&str; 5] = [
    "factual-error",
    "new-evidence",
    "safety",
    "policy",
    "constitutional",
];

/// Challenge status states
pub const CHALLENGE_STATUS: [&str; 5] = [
    "filed",
    "acknowledged",
    "under-review",
    "resolved",
    "dismissed",
];

/// Proposal - A formal proposal for changes.
///
/// Supports various governance decision-making mechanisms.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub proposal_type: String,        // sense-check, consent, consensus, supermajority
    pub description: String,
    pub proposer_id: String,
    pub proposer_name: String,
    pub rationale: String,
    pub status: String,               // draft, discussion, voting, decided, dismissed
    pub phase: String,                // Current phase
    pub amendments_json: String,      // Amendment[] as JSON
    pub voting_config_json: String,   // VotingConfig as JSON
    pub current_votes_json: String,   // VoteCount as JSON
    pub outcome_json: Option<String>, // ProposalOutcome as JSON
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Proposal types
pub const PROPOSAL_TYPES: [&str; 4] = [
    "sense-check",
    "consent",
    "consensus",
    "supermajority",
];

/// Proposal status states
pub const PROPOSAL_STATUS: [&str; 5] = [
    "draft",
    "discussion",
    "voting",
    "decided",
    "dismissed",
];

/// Precedent - A binding decision that guides future decisions.
///
/// Precedents form the case law of the governance system.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Precedent {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub full_reasoning: String,
    pub binding: String,              // constitutional, binding-network, binding-local, persuasive
    pub scope_json: String,           // { entityTypes, categories, roles } as JSON
    pub citations: u32,               // How often this precedent is cited
    pub status: String,               // active, superseded, under-review
    pub established_by: String,       // Proposal ID or governance body
    pub established_at: String,
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Precedent binding levels
pub const PRECEDENT_BINDING: [&str; 4] = [
    "constitutional",
    "binding-network",
    "binding-local",
    "persuasive",
];

/// Discussion - A threaded discussion on an entity.
///
/// Enables structured deliberation on content, proposals, or challenges.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Discussion {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub category: String,             // general, proposal, challenge, feedback
    pub title: String,
    pub messages_json: String,        // DiscussionMessage[] as JSON
    pub status: String,               // open, closed, archived
    pub message_count: u32,
    pub last_activity_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Discussion categories
pub const DISCUSSION_CATEGORIES: [&str; 4] = [
    "general",
    "proposal",
    "challenge",
    "feedback",
];

/// GovernanceState - Current governance status of an entity.
///
/// Tracks the governance posture of content, paths, etc.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GovernanceState {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,               // approved, pending, challenged, suspended
    pub status_basis_json: String,    // StatusBasis as JSON
    pub labels_json: String,          // Label[] as JSON
    pub active_challenges_json: String, // String[] as JSON
    pub active_proposals_json: String,  // String[] as JSON
    pub precedent_ids_json: String,     // String[] as JSON
    pub last_updated: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Governance status states
pub const GOVERNANCE_STATUS: [&str; 4] = [
    "approved",
    "pending",
    "challenged",
    "suspended",
];

// =============================================================================
// hREA Point System Entries - Value Flow Demonstration
// =============================================================================

/// Learner Point Balance - tracks points earned by a learner
/// Demonstrates hREA EconomicResource (accountingQuantity)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct LearnerPointBalance {
    pub id: String,
    pub agent_id: String,
    /// Current point balance
    pub total_points: i64,
    /// Breakdown by trigger type
    pub points_by_trigger_json: String,   // {trigger: points}
    /// Lifetime statistics
    pub total_earned: i64,
    pub total_spent: i64,                 // If points can be "spent"
    /// Tracking
    pub last_point_event_id: Option<String>,
    pub last_point_event_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Point Event - an individual point earning/spending event
/// Demonstrates hREA EconomicEvent
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PointEvent {
    pub id: String,
    pub agent_id: String,
    /// hREA action: "produce" (earn), "consume" (spend), "transfer"
    pub action: String,
    /// The trigger that caused this event
    pub trigger: String,
    /// Points involved (positive for earn, negative for spend/lose)
    pub points: i32,
    /// Context - what content/challenge/path triggered this
    pub content_id: Option<String>,
    pub challenge_id: Option<String>,
    pub path_id: Option<String>,
    /// For challenge events - was this correct/incorrect
    pub was_correct: Option<bool>,
    /// Metadata
    pub note: Option<String>,
    pub metadata_json: String,
    /// Timestamp (hREA hasPointInTime)
    pub occurred_at: String,
}

/// Contributor Recognition - tracks recognition flowing to content contributors
/// Demonstrates hREA Appreciation - "economic event given in appreciation for another"
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContributorRecognition {
    pub id: String,
    /// The contributor receiving recognition
    pub contributor_id: String,
    /// The content that triggered this recognition
    pub content_id: String,
    /// The learner whose action triggered recognition
    pub learner_id: String,
    /// The learner's point event that triggered this (hREA appreciationOf)
    pub appreciation_of_event_id: String,
    /// Recognition flow type
    pub flow_type: String,
    /// Recognition points awarded to contributor
    pub recognition_points: i32,
    /// Context
    pub path_id: Option<String>,
    pub challenge_id: Option<String>,
    /// Metadata
    pub note: Option<String>,
    /// Timestamp
    pub occurred_at: String,
}

/// Contributor Impact Summary - aggregate impact stats for a contributor
/// Demonstrates hREA resource accounting over time
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContributorImpact {
    pub id: String,
    pub contributor_id: String,
    /// Aggregate recognition
    pub total_recognition_points: i64,
    pub total_learners_reached: u32,
    pub total_content_mastered: u32,    // Learners who mastered contributor's content
    pub total_discoveries_sparked: u32,  // Content discovered via their content
    /// Breakdown by content
    pub impact_by_content_json: String, // {content_id: {points, learners, mastered}}
    /// Breakdown by recognition type
    pub impact_by_flow_type_json: String, // {flow_type: points}
    /// Time series for visualization
    pub recent_events_json: String,     // Last N recognition events
    /// Timestamps
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Lamad: Steward Economy - Sustainable Income for Knowledge Stewards
// =============================================================================
//
// Enables stewards to earn sustainable income from their care of the knowledge graph:
// 1. StewardCredential - Proof of qualification to steward premium content
// 2. PremiumGate - Access control for curated content/paths
// 3. AccessGrant - Record of learner gaining access
// 4. StewardRevenue - Value flowing to stewards (via Shefa EconomicEvents)
//
// Key insight: Stewards may or may not be original creators. A steward:
// - Takes responsibility for content quality and curation
// - May hold a ContributorPresence in trust (like Lynn Foster's content)
// - Earns from their stewardship work, not just creation
// - Can be the creator, a maintainer, or a commons steward
//
// Revenue model: Steward keeps ~85%, Commons (Elohim) gets ~15% for infrastructure

/// Steward credential tiers - qualification levels for stewarding content
pub const STEWARD_TIERS: [&str; 4] = [
    "caretaker",    // Basic stewardship - maintains existing content
    "curator",      // Active curation - organizes and improves paths
    "expert",       // Domain expertise + peer attestations
    "pioneer",      // Original researcher/synthesizer, primary steward
];

/// Access requirement types for premium gates
pub const ACCESS_REQUIREMENT_TYPES: [&str; 5] = [
    "attestation",   // Must have specific attestation
    "mastery",       // Must have mastery level on prerequisite content
    "payment",       // Must pay (credits, tokens, fiat via bridge)
    "peer_vouch",    // Must have peer vouches from qualified members
    "scholarship",   // Elohim-funded access (commons pool)
];

/// Pricing models for premium gates
pub const PRICING_MODELS: [&str; 5] = [
    "one_time",            // Single payment, lifetime access
    "subscription",        // Recurring payment
    "pay_what_you_can",    // Minimum + optional additional
    "free_with_attribution", // Free but creator gets recognition flow
    "commons_sponsored",   // Elohim commons pool covers cost
];

/// Access grant types
pub const ACCESS_GRANT_TYPES: [&str; 4] = [
    "lifetime",      // Permanent access
    "subscription",  // Time-limited, renewable
    "trial",         // Time-limited, non-renewable
    "revocable",     // Can be revoked (e.g., scholarship terms)
];

/// StewardCredential - Proof of qualification to steward premium content
///
/// Before a steward can gate content, they must demonstrate qualification:
/// 1. Domain knowledge - Mastery level on the content domain
/// 2. Peer attestations - Other stewards/experts vouch for them
/// 3. Stewardship track record - Demonstrated care for the knowledge graph
///
/// Note: Stewards may hold content in trust for absent contributors (like
/// importing Lynn Foster's course while she's not on the network yet).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StewardCredential {
    pub id: String,
    /// The steward's own presence (their Human or ContributorPresence)
    pub steward_presence_id: String,
    /// Agent who holds this credential
    pub agent_id: String,
    /// Credential tier (STEWARD_TIERS)
    pub tier: String,

    // Stewardship scope - what they can steward
    /// ContributorPresence IDs they steward (may include others' presences held in trust)
    pub stewarded_presence_ids_json: String,  // String[] as JSON
    /// Content IDs under their stewardship
    pub stewarded_content_ids_json: String,  // String[] as JSON
    /// Path IDs under their stewardship
    pub stewarded_path_ids_json: String,  // String[] as JSON

    // Domain qualification
    /// Content IDs where steward has demonstrated mastery
    pub mastery_content_ids_json: String,  // String[] as JSON
    /// Mastery level achieved (minimum required varies by tier)
    pub mastery_level_achieved: String,
    /// When qualification was verified
    pub qualification_verified_at: String,

    // Peer attestation
    /// Attestation IDs from peers vouching for stewardship quality
    pub peer_attestation_ids_json: String,  // String[] as JSON
    /// Number of unique attesters
    pub unique_attester_count: u32,
    /// Aggregate reputation of attesters
    pub attester_reputation_sum: f64,

    // Stewardship track record
    /// Quality metrics for their stewardship work
    pub stewardship_quality_score: f64,
    /// Total learners served through their stewarded content
    pub total_learners_served: u32,
    /// Content improvements/updates made
    pub total_content_improvements: u32,

    // Domain scope
    /// Tags/domains this credential applies to
    pub domain_tags_json: String,  // String[] as JSON

    // Status
    /// Is this credential currently valid?
    pub is_active: bool,
    /// Reason if deactivated
    pub deactivation_reason: Option<String>,

    // Metadata
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// PremiumGate - Access control for curated content/paths
///
/// Defines requirements learners must meet to access premium content.
/// Revenue flows to steward with commons fee to Elohim for infrastructure.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PremiumGate {
    pub id: String,
    /// Steward who controls this gate
    pub steward_credential_id: String,
    /// The steward's presence
    pub steward_presence_id: String,
    /// Original contributor presence (may differ from steward if holding in trust)
    pub contributor_presence_id: Option<String>,

    // What's being gated
    /// Gated resource type: "path", "chapter", "content", "content_bundle"
    pub gated_resource_type: String,
    /// IDs of gated resources
    pub gated_resource_ids_json: String,  // String[] as JSON
    /// Display info
    pub gate_title: String,
    pub gate_description: String,
    pub gate_image: Option<String>,

    // Access requirements (learner must meet ALL)
    /// Required attestations (JSON array of {attestation_type, attestation_id?})
    pub required_attestations_json: String,
    /// Required mastery (JSON array of {content_id, min_level})
    pub required_mastery_json: String,
    /// Required peer vouches (JSON: {min_count, from_tier?})
    pub required_vouches_json: String,

    // Pricing
    /// Pricing model (PRICING_MODELS)
    pub pricing_model: String,
    /// Price amount (null for free/attribution models)
    pub price_amount: Option<f64>,
    /// Price unit (e.g., "elohim-credit", "usd", "eth")
    pub price_unit: Option<String>,
    /// For subscription: period in days
    pub subscription_period_days: Option<u32>,
    /// For pay_what_you_can: minimum amount
    pub min_amount: Option<f64>,

    // Revenue share
    /// Percentage to steward (e.g., 85.0)
    pub steward_share_percent: f64,
    /// Percentage to commons (e.g., 15.0) - covers infrastructure
    pub commons_share_percent: f64,
    /// If contributor_presence_id differs from steward, portion for original contributor
    pub contributor_share_percent: Option<f64>,

    // Scholarship support
    /// Can Elohim commons fund access for qualifying learners?
    pub scholarship_eligible: bool,
    /// Max scholarships per period
    pub max_scholarships_per_period: Option<u32>,
    /// Scholarship criteria (JSON: requirements for scholarship)
    pub scholarship_criteria_json: Option<String>,

    // Status
    pub is_active: bool,
    pub deactivation_reason: Option<String>,

    // Stats
    pub total_access_grants: u32,
    pub total_revenue_generated: f64,
    pub total_to_steward: f64,
    pub total_to_contributor: f64,  // If different from steward
    pub total_to_commons: f64,
    pub total_scholarships_granted: u32,

    // Metadata
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// AccessGrant - Record of learner gaining access to gated content
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AccessGrant {
    pub id: String,
    /// The gate this grants access through
    pub gate_id: String,
    /// The learner agent
    pub learner_agent_id: String,

    // How access was granted
    /// Grant type (ACCESS_GRANT_TYPES)
    pub grant_type: String,
    /// How it was obtained: "payment", "attestation_met", "scholarship", "creator_gift"
    pub granted_via: String,

    // Payment details (if payment-based)
    /// Payment event ID (links to Shefa EconomicEvent)
    pub payment_event_id: Option<String>,
    /// Amount paid
    pub payment_amount: Option<f64>,
    pub payment_unit: Option<String>,

    // Scholarship details (if scholarship-based)
    pub scholarship_sponsor_id: Option<String>,  // Usually "elohim-commons"
    pub scholarship_reason: Option<String>,

    // Access window
    pub granted_at: String,
    /// Null for lifetime access
    pub valid_until: Option<String>,
    /// For subscriptions: when renewal is due
    pub renewal_due_at: Option<String>,

    // Status
    pub is_active: bool,
    pub revoked_at: Option<String>,
    pub revoke_reason: Option<String>,

    // Metadata
    pub metadata_json: String,
    pub created_at: String,
}

/// StewardRevenue - Value flowing from gate access to stewards and contributors
/// This creates underlying Shefa EconomicEvents for the value transfers.
///
/// Supports three-way split when steward holds content in trust:
/// - Steward gets their share for curation/maintenance work
/// - Contributor gets their share (accumulates at their presence)
/// - Commons gets infrastructure fee
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StewardRevenue {
    pub id: String,
    /// The access grant that triggered this revenue
    pub access_grant_id: String,
    pub gate_id: String,

    // Parties
    pub from_learner_id: String,
    /// Steward receiving their share
    pub to_steward_presence_id: String,
    /// Original contributor (may be same as steward, or held in trust)
    pub to_contributor_presence_id: Option<String>,

    // Amounts
    /// Total payment received
    pub gross_amount: f64,
    pub payment_unit: String,
    /// Amount to steward for their work
    pub steward_amount: f64,
    /// Amount to contributor (accumulates even if unclaimed)
    pub contributor_amount: f64,
    /// Amount to Elohim commons (infrastructure)
    pub commons_amount: f64,

    // Shefa linkage - creates EconomicEvents for each flow
    /// The EconomicEvent for learner→steward transfer
    pub steward_economic_event_id: String,
    /// The EconomicEvent for learner→contributor transfer (if applicable)
    pub contributor_economic_event_id: Option<String>,
    /// The EconomicEvent for learner→commons transfer
    pub commons_economic_event_id: String,

    // Status
    pub status: String,  // "pending", "completed", "failed", "refunded"
    pub completed_at: Option<String>,
    pub failure_reason: Option<String>,

    // Metadata
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
}

// =============================================================================
// CustodianCommitment - Digital Presence Stewardship
// =============================================================================

/// CustodianCommitment - Promise to custody content for resilience
///
/// Enables organic account protection through social relationships:
/// - Family automatically custodies family-reach content (intimate relationships)
/// - Communities custody community-reach content
/// - Specialists custody category-specific content (medical, video, etc.)
/// - Emergency protocols (manual, trusted party, M-of-N) can reconstruct presence
///
/// Lifecycle: PROPOSED → ACCEPTED → ACTIVE → FULFILLED | BREACHED
///
/// Example commitments:
/// 1. Alice (spouse) → Bob's "intimate" reach content (relationship-based, 60-80%)
/// 2. Carol (nurse) → Medical content (category override, 20-40%)
/// 3. Dave (fiber ISP) → Video content (bandwidth override)
/// 4. Community (100 of 100k followers) → Sheila's commons-reach content (democratic resilience)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CustodianCommitment {
    pub id: String,

    // =========================================================================
    // Parties
    // =========================================================================
    pub custodian_agent_id: String,       // Who is custodying
    pub beneficiary_agent_id: String,     // Whose content/presence is being custodied
    pub commitment_type: String,          // From COMMITMENT_TYPES: relationship|category|community|steward

    // =========================================================================
    // Basis (WHY this custodian was selected)
    // =========================================================================
    pub basis: String,                    // From COMMITMENT_BASIS: intimate_relationship|trusted_relationship|etc.
    pub relationship_id: Option<String>,  // If relationship-based, link to HumanRelationship
    pub category_override_json: String,   // CategoryOverride[] if category-based (empty string if not)

    // =========================================================================
    // Scope (WHAT is being custodied)
    // =========================================================================
    /// ContentFilter[] - which content matches this commitment (reach levels, categories, tags, etc.)
    pub content_filters_json: String,
    pub estimated_content_count: u32,     // Estimated number of items matching filters
    pub estimated_size_mb: f64,           // Estimated total size

    // =========================================================================
    // Shard Topology
    // =========================================================================
    pub shard_strategy: String,           // From SHARD_STRATEGIES: full_replica|threshold_split|erasure_coded
    pub redundancy_factor: u32,           // M (threshold for recovery)
    /// ShardAssignment[] - which shards this custodian holds (encrypted on DHT)
    pub shard_assignments_json: String,

    // =========================================================================
    // Emergency Protocol (all three trigger types can be enabled)
    // =========================================================================
    /// EmergencyTrigger[] - what can activate emergency mode
    pub emergency_triggers_json: String,
    /// EmergencyContact[] - who to notify when activated
    pub emergency_contacts_json: String,
    /// RecoveryPlan - instructions for reconstruction
    pub recovery_instructions_json: String,

    // =========================================================================
    // Performance Hints (for cache layer, NOT custody requirements)
    // =========================================================================
    pub cache_priority: u32,              // Higher = prefer when serving from cache
    pub bandwidth_class: String,          // From BANDWIDTH_CLASSES: low|medium|high|ultra
    pub geographic_affinity: Option<String>, // Hint: prefer serving to this region

    // =========================================================================
    // State Tracking
    // =========================================================================
    pub state: String,                    // From COMMITMENT_STATES: proposed|accepted|in-progress|fulfilled|cancelled|breached
    pub proposed_at: String,
    pub accepted_at: Option<String>,
    pub activated_at: Option<String>,     // When emergency protocol was activated
    pub last_verification_at: Option<String>, // Last time shards were verified
    /// VerificationFailure[] - any shard integrity checks that failed
    pub verification_failures_json: String,

    // =========================================================================
    // Fulfillment Tracking
    // =========================================================================
    pub shards_stored_count: u32,         // How many shards are currently stored
    pub last_shard_update_at: Option<String>, // When shards were last updated
    pub total_restores_performed: u32,    // How many times content was reconstructed

    // =========================================================================
    // Economic Context
    // =========================================================================
    pub shefa_commitment_id: Option<String>, // Links to Shefa Commitment if compensation is involved

    // =========================================================================
    // Metadata
    // =========================================================================
    pub note: Option<String>,
    pub metadata_json: String,            // Extensible metadata
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Anchor Entries (for link indexing)
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
    // Lamad: Content & Learning
    Content(Content),
    LearningPath(LearningPath),
    PathChapter(PathChapter),
    PathStep(PathStep),
    ContentMastery(ContentMastery),
    PracticePool(PracticePool),
    MasteryChallenge(MasteryChallenge),
    KnowledgeMap(KnowledgeMap),
    PathExtension(PathExtension),

    // Qahal: Community & Relationships
    Relationship(Relationship),

    // Governance
    Challenge(Challenge),
    Proposal(Proposal),
    Precedent(Precedent),
    Discussion(Discussion),
    GovernanceState(GovernanceState),

    // Imago Dei: Identity
    Human(Human),           // Legacy, kept for backward compatibility
    HumanProgress(HumanProgress), // Legacy
    Agent(Agent),           // Expanded identity model
    AgentProgress(AgentProgress), // Expanded progress model
    Attestation(Attestation),
    CustodianCommitment(CustodianCommitment), // Digital presence stewardship

    // Shefa: Economy (REA/ValueFlows)
    EconomicEvent(EconomicEvent),
    EconomicResource(EconomicResource),

    // Shefa: Point System (hREA demonstration)
    LearnerPointBalance(LearnerPointBalance),
    PointEvent(PointEvent),
    ContributorRecognition(ContributorRecognition),
    ContributorImpact(ContributorImpact),
    ContributorPresence(ContributorPresence),
    Process(Process),
    Intent(Intent),
    Commitment(Commitment),
    Appreciation(Appreciation),
    Claim(Claim),
    Settlement(Settlement),

    // Lamad: Steward Economy
    StewardCredential(StewardCredential),
    PremiumGate(PremiumGate),
    AccessGrant(AccessGrant),
    StewardRevenue(StewardRevenue),

    // Infrastructure
    StringAnchor(StringAnchor),
}

// =============================================================================
// Link Types
// =============================================================================

#[hdk_link_types]
pub enum LinkTypes {
    // =========================================================================
    // Lamad: Content indexing links
    // =========================================================================
    IdToContent,
    TypeToContent,
    TagToContent,
    AuthorToContent,
    ImportBatchToContent,

    // =========================================================================
    // Lamad: Learning path links
    // =========================================================================
    IdToPath,
    IdToStep,                   // Anchor(step_id) -> PathStep
    PathToStep,
    StepToContent,
    PathByCreator,              // Anchor(created_by) -> LearningPath
    PathByDifficulty,           // Anchor(difficulty) -> LearningPath
    PathByType,                 // Anchor(path_type) -> LearningPath
    PathByTag,                  // Anchor(tag) -> LearningPath

    // =========================================================================
    // Lamad: Chapter links
    // =========================================================================
    IdToChapter,                // Anchor(chapter_id) -> PathChapter
    PathToChapter,              // LearningPath -> PathChapter
    ChapterToStep,              // PathChapter -> PathStep
    StepToChapter,              // PathStep -> PathChapter (reverse lookup)

    // =========================================================================
    // Lamad: Progress & Mastery links
    // =========================================================================
    AgentToPathProgress,        // Anchor(agent_id) -> AgentProgress (for path queries)
    PathToProgress,             // Anchor(path_id) -> AgentProgress (for completion stats)
    ProgressByStatus,           // Anchor(status: in_progress|completed|abandoned) -> AgentProgress

    // =========================================================================
    // Lamad: Content Mastery links
    // =========================================================================
    HumanToMastery,             // Anchor(human_id) -> ContentMastery
    ContentToMastery,           // Anchor(content_id) -> ContentMastery
    MasteryByLevel,             // Anchor(mastery_level) -> ContentMastery

    // =========================================================================
    // Lamad: Practice Pool & Mastery Challenge links
    // =========================================================================
    AgentToPool,                // Anchor(agent_id) -> PracticePool
    PoolToContent,              // PracticePool -> Content (active content)
    PoolToPath,                 // PracticePool -> LearningPath (contributing paths)
    AgentToChallenge,           // Anchor(agent_id) -> MasteryChallenge
    PoolToChallenge,            // PracticePool -> MasteryChallenge
    ChallengeByState,           // Anchor(state) -> MasteryChallenge

    // =========================================================================
    // Shefa: Point System links (hREA demonstration)
    // =========================================================================
    AgentToPointBalance,        // Anchor(agent_id) -> LearnerPointBalance
    AgentToPointEvents,         // Anchor(agent_id) -> PointEvent
    PointEventByTrigger,        // Anchor(trigger) -> PointEvent
    ContentToPointEvents,       // Anchor(content_id) -> PointEvent (events for this content)
    ContributorToRecognition,   // Anchor(contributor_id) -> ContributorRecognition
    ContentToRecognition,       // Anchor(content_id) -> ContributorRecognition
    ContributorToImpact,        // Anchor(contributor_id) -> ContributorImpact
    RecognitionByFlowType,      // Anchor(flow_type) -> ContributorRecognition

    // =========================================================================
    // Qahal: Relationship links (replaces Kuzu)
    // =========================================================================
    ContentToRelated,           // Content -> Relationship entry
    RelationshipBySource,       // Anchor(source_id) -> Relationship
    RelationshipByTarget,       // Anchor(target_id) -> Relationship
    RelationshipByType,         // Anchor(rel_type) -> Relationship

    // =========================================================================
    // Imago Dei: Human/Agent links (Legacy)
    // =========================================================================
    IdToHuman,                  // Anchor(human_id) -> Human
    HumanByAffinity,            // Anchor(affinity) -> Human
    HumanToProgress,            // Human -> HumanProgress
    ProgressToPath,             // HumanProgress -> LearningPath

    // =========================================================================
    // Imago Dei: Human Presence links (Secure Login)
    // =========================================================================
    AgentKeyToHuman,            // AgentPubKey -> Human (one-to-one binding for auth)
    HumanByExternalId,          // Anchor(provider:credential_hash) -> Human

    // =========================================================================
    // Imago Dei: Agent links (Expanded)
    // =========================================================================
    IdToAgent,                  // Anchor(agent_id) -> Agent
    AgentByType,                // Anchor(agent_type) -> Agent
    AgentByAffinity,            // Anchor(affinity) -> Agent
    AgentToProgress,            // Agent -> AgentProgress
    AgentProgressToPath,        // AgentProgress -> LearningPath
    AgentKeyToAgent,            // AgentPubKey -> Agent (one-to-one binding for auth)
    ElohimByScope,              // Anchor(scope: family|community|global) -> Agent (Elohim only)

    // =========================================================================
    // Imago Dei: Attestation links
    // =========================================================================
    AgentToAttestation,         // Anchor(agent_id) -> Attestation
    AttestationByCategory,      // Anchor(category) -> Attestation
    AttestationByType,          // Anchor(attestation_type) -> Attestation

    // =========================================================================
    // Imago Dei: CustodianCommitment links (Digital Presence Stewardship)
    // =========================================================================
    IdToCommitmentCustodian,        // Anchor(commitment_id) -> CustodianCommitment
    CustodianToCommitment,          // Anchor(custodian_agent_id) -> CustodianCommitment
    BeneficiaryToCommitment,        // Anchor(beneficiary_agent_id) -> CustodianCommitment
    CustodianCommitmentByType,      // Anchor(commitment_type) -> CustodianCommitment
    CustodianCommitmentByBasis,     // Anchor(basis) -> CustodianCommitment
    CustodianCommitmentByState,     // Anchor(state) -> CustodianCommitment
    RelationshipToCommitment,       // HumanRelationship -> CustodianCommitment (automatic)
    ContentToCommitmentCustodian,   // Content -> CustodianCommitment (which commitments cover)

    // =========================================================================
    // Shefa: Economic Event links
    // =========================================================================
    IdToEvent,                  // Anchor(event_id) -> EconomicEvent
    ProviderToEvent,            // Anchor(provider_id) -> EconomicEvent
    ReceiverToEvent,            // Anchor(receiver_id) -> EconomicEvent
    EventByAction,              // Anchor(action) -> EconomicEvent
    EventByLamadType,           // Anchor(lamad_event_type) -> EconomicEvent
    ProcessInputEvent,          // Process -> EconomicEvent (input)
    ProcessOutputEvent,         // Process -> EconomicEvent (output)
    EventFulfillsCommitment,    // EconomicEvent -> Commitment
    EventSatisfiesIntent,       // EconomicEvent -> Intent

    // =========================================================================
    // Shefa: Economic Resource links
    // =========================================================================
    IdToResource,               // Anchor(resource_id) -> EconomicResource
    ResourceBySpec,             // Anchor(conforms_to) -> EconomicResource
    ResourceByOwner,            // Anchor(primary_accountable) -> EconomicResource
    ResourceByCustodian,        // Anchor(custodian) -> EconomicResource
    ResourceByClassification,   // Anchor(classification) -> EconomicResource
    ContentToResource,          // Content -> EconomicResource (for content resources)

    // =========================================================================
    // Shefa: Contributor Presence links
    // =========================================================================
    IdToPresence,               // Anchor(presence_id) -> ContributorPresence
    PresenceByState,            // Anchor(presence_state) -> ContributorPresence
    StewardToPresence,          // Anchor(steward_id) -> ContributorPresence
    ContentToPresence,          // Content -> ContributorPresence (establishing content)
    ClaimedAgentToPresence,     // Anchor(claimed_agent_id) -> ContributorPresence
    PresenceToRecognition,      // ContributorPresence -> EconomicResource (recognition)

    // =========================================================================
    // Shefa: Process links
    // =========================================================================
    IdToProcess,                // Anchor(process_id) -> Process
    ProcessByClassification,    // Anchor(classification) -> Process
    PathToProcess,              // LearningPath -> Process
    AgentPerformsProcess,       // Anchor(performed_by) -> Process

    // =========================================================================
    // Shefa: Intent links
    // =========================================================================
    IdToIntent,                 // Anchor(intent_id) -> Intent
    IntentByAction,             // Anchor(action) -> Intent
    IntentByProvider,           // Anchor(provider) -> Intent (offers)
    IntentByReceiver,           // Anchor(receiver) -> Intent (requests)
    ProcessToIntent,            // Process -> Intent

    // =========================================================================
    // Shefa: Commitment links
    // =========================================================================
    IdToCommitment,             // Anchor(commitment_id) -> Commitment
    CommitmentByProvider,       // Anchor(provider) -> Commitment
    CommitmentByReceiver,       // Anchor(receiver) -> Commitment
    CommitmentByState,          // Anchor(state) -> Commitment
    CommitmentSatisfiesIntent,  // Commitment -> Intent
    ProcessToCommitment,        // Process -> Commitment

    // =========================================================================
    // Shefa: Appreciation links
    // =========================================================================
    IdToAppreciation,           // Anchor(appreciation_id) -> Appreciation
    AppreciationByGiver,        // Anchor(appreciated_by) -> Appreciation
    AppreciationByReceiver,     // Anchor(appreciation_to) -> Appreciation
    EventToAppreciation,        // EconomicEvent -> Appreciation

    // =========================================================================
    // Shefa: Claim & Settlement links
    // =========================================================================
    IdToClaim,                  // Anchor(claim_id) -> Claim
    ClaimByProvider,            // Anchor(provider) -> Claim
    ClaimByReceiver,            // Anchor(receiver) -> Claim
    EventTriggersClaim,         // EconomicEvent -> Claim
    IdToSettlement,             // Anchor(settlement_id) -> Settlement
    ClaimToSettlement,          // Claim -> Settlement
    EventSettlesClaim,          // EconomicEvent -> Settlement

    // =========================================================================
    // Lamad: Steward Economy links
    // =========================================================================
    // Steward Credentials
    IdToStewardCredential,      // Anchor(credential_id) -> StewardCredential
    HumanToCredential,          // Anchor(human_presence_id) -> StewardCredential
    CredentialByTier,           // Anchor(steward_tier) -> StewardCredential
    CredentialByDomain,         // Anchor(domain_scope) -> StewardCredential
    CredentialToPresence,       // StewardCredential -> ContributorPresence (stewarded)

    // Premium Gates
    IdToGate,                   // Anchor(gate_id) -> PremiumGate
    ResourceToGate,             // Anchor(gated_resource_id) -> PremiumGate
    GateByPricingModel,         // Anchor(pricing_model) -> PremiumGate
    GateBySteward,              // Anchor(steward_credential_id) -> PremiumGate
    GateByContributor,          // Anchor(contributor_presence_id) -> PremiumGate

    // Access Grants
    IdToAccessGrant,            // Anchor(grant_id) -> AccessGrant
    LearnerToGrant,             // Anchor(learner_id) -> AccessGrant
    GateToGrant,                // Anchor(gate_id) -> AccessGrant
    GrantByType,                // Anchor(access_type) -> AccessGrant

    // Steward Revenue
    IdToStewardRevenue,         // Anchor(revenue_id) -> StewardRevenue
    GrantToRevenue,             // AccessGrant -> StewardRevenue
    StewardToRevenue,           // Anchor(steward_credential_id) -> StewardRevenue
    ContributorToRevenue,       // Anchor(contributor_presence_id) -> StewardRevenue
    RevenueByStatus,            // Anchor(status) -> StewardRevenue

    // =========================================================================
    // Lamad: KnowledgeMap links
    // =========================================================================
    IdToKnowledgeMap,           // Anchor(knowledge_map_id) -> KnowledgeMap
    OwnerToKnowledgeMap,        // Anchor(owner_id) -> KnowledgeMap
    KnowledgeMapByType,         // Anchor(map_type) -> KnowledgeMap

    // =========================================================================
    // Lamad: PathExtension links
    // =========================================================================
    IdToPathExtension,          // Anchor(extension_id) -> PathExtension
    ExtenderToExtension,        // Anchor(extended_by) -> PathExtension
    BasePathToExtension,        // Anchor(base_path_id) -> PathExtension

    // =========================================================================
    // Lamad: Governance links
    // =========================================================================
    // Challenge
    IdToChallenge,              // Anchor(challenge_id) -> Challenge
    EntityToChallenge,          // Anchor(entity_type:entity_id) -> Challenge
    ChallengerToChallenge,      // Anchor(challenger_id) -> Challenge
    ChallengeByStatus,          // Anchor(status) -> Challenge

    // Proposal
    IdToProposal,               // Anchor(proposal_id) -> Proposal
    ProposalByType,             // Anchor(proposal_type) -> Proposal
    ProposerToProposal,         // Anchor(proposer_id) -> Proposal
    ProposalByStatus,           // Anchor(status) -> Proposal

    // Precedent
    IdToPrecedent,              // Anchor(precedent_id) -> Precedent
    PrecedentByScope,           // Anchor(scope) -> Precedent
    PrecedentByStatus,          // Anchor(status) -> Precedent

    // Discussion
    IdToDiscussion,             // Anchor(discussion_id) -> Discussion
    EntityToDiscussion,         // Anchor(entity_type:entity_id) -> Discussion
    DiscussionByCategory,       // Anchor(category) -> Discussion
    DiscussionByStatus,         // Anchor(status) -> Discussion

    // GovernanceState
    IdToGovernanceState,        // Anchor(entity_type:entity_id) -> GovernanceState
    GovernanceStateByStatus,    // Anchor(status) -> GovernanceState
}

// =============================================================================
// Validation
// =============================================================================

#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

#[hdk_extern]
pub fn validate(_op: Op) -> ExternResult<ValidateCallbackResult> {
    // TODO: Add proper validation for each entry type
    Ok(ValidateCallbackResult::Valid)
}
