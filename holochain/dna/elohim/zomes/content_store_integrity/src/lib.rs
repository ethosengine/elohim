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
use doorway_client::Cacheable;

// =============================================================================
// Self-Healing Support
// =============================================================================
pub mod healing;

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

// =============================================================================
// Blob Management - Phase 1: Large Media Support (Video, Podcasts)
// =============================================================================

/// Video/audio codec types (track codec obsolescence for adaptive delivery)
pub const CODEC_TYPES: [&str; 15] = [
    // Video codecs (H.264, H.265, VP9, AV1)
    "h264",     // H.264/AVC (backward compatible, widely supported)
    "h265",     // H.265/HEVC (more efficient, newer devices)
    "vp8",      // VP8 (WebRTC, WebM)
    "vp9",      // VP9 (YouTube, WebM)
    "av1",      // AV1 (next-gen, highest compression)
    // Audio codecs (AAC, Opus, FLAC, MP3)
    "aac",      // AAC (standard audio)
    "opus",     // Opus (modern, variable bitrate)
    "flac",     // FLAC (lossless)
    "mp3",      // MP3 (legacy, universal)
    "vorbis",   // Vorbis (open source audio)
    // Container/other
    "mp4",      // MP4 container
    "webm",     // WebM container
    "mkv",      // Matroska container
    "mov",      // QuickTime container
    "unknown",  // Unknown/undeclared
];

/// Video resolution variants for adaptive streaming
pub const VIDEO_VARIANTS: [&str; 6] = [
    "480p",   // 854x480 - mobile minimum
    "720p",   // 1280x720 - HD
    "1080p",  // 1920x1080 - Full HD
    "1440p",  // 2560x1440 - QHD
    "2160p",  // 3840x2160 - 4K
    "4320p",  // 7680x4320 - 8K
];

/// Caption/subtitle formats
pub const CAPTION_FORMATS: [&str; 5] = [
    "webvtt",  // WebVTT (web standard)
    "srt",     // SubRip (simple)
    "vtt",     // VTT (alias for webvtt)
    "ass",     // ASS/SSA (advanced formatting)
    "ssa",     // SSA (older format)
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
    // Self-healing DNA fields (for schema evolution support)
    #[serde(default)]
    pub schema_version: u32,             // Increment when Content schema changes
    #[serde(default)]
    pub validation_status: String,       // Valid, Migrated, Degraded, Healing
}

impl Cacheable for Content {
    fn cache_type() -> &'static str {
        "Content"
    }

    fn cache_id(&self) -> String {
        self.id.clone()
    }

    fn cache_ttl() -> u64 {
        3600 // 1 hour for content
    }

    fn is_public(&self) -> bool {
        self.reach == "commons"
    }

    fn reach(&self) -> Option<&str> {
        Some(&self.reach)
    }
}

// =============================================================================
// Blob (Media) Entries - Phase 1: Large Media Support
// =============================================================================

/// Blob entry - metadata for large media files (video, audio, podcasts)
///
/// Blobs are NOT stored in DHT (too large). This entry stores only metadata:
/// - Cryptographic hash for integrity verification
/// - Size for cache allocation planning
/// - MIME type for rendering
/// - Codec info for adaptive streaming
/// - Fallback URLs for resilience (CDN, custodians, P2P)
///
/// Actual blob data distributed separately via:
/// - HTTP Range requests (resumable downloads)
/// - HLS/DASH adaptive streaming
/// - Custodian P2P replication
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct BlobEntry {
    /// SHA256 hash of blob content (hex string, 64 chars)
    pub hash: String,

    /// Total size in bytes
    pub size_bytes: u64,

    /// MIME type (video/mp4, audio/mpeg, etc.)
    pub mime_type: String,

    /// Fallback URLs for resilient access (primary, secondary, tertiary, custodian URLs)
    /// Tried in order until one succeeds
    pub fallback_urls: Vec<String>,

    /// Bitrate in megabits per second (for quality indication)
    pub bitrate_mbps: Option<f32>,

    /// Duration in seconds (for audio/video)
    pub duration_seconds: Option<u32>,

    /// Codec used (h264, h265, vp9, av1, opus, flac, etc.)
    pub codec: Option<String>,

    /// Visibility level (private, commons, etc.)
    pub reach: String,

    /// Author/uploader agent ID
    pub author_id: Option<String>,

    /// When created
    pub created_at: String,

    /// When last verified/accessed
    pub verified_at: Option<String>,
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
    // Self-healing DNA fields
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
}

impl Cacheable for LearningPath {
    fn cache_type() -> &'static str {
        "LearningPath"
    }

    fn cache_id(&self) -> String {
        self.id.clone()
    }

    fn cache_ttl() -> u64 {
        1800 // 30 minutes for paths
    }

    fn is_public(&self) -> bool {
        self.visibility == "public"
    }
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
    // Self-healing DNA fields
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
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

impl Cacheable for Relationship {
    fn cache_type() -> &'static str {
        "Relationship"
    }

    fn cache_id(&self) -> String {
        self.id.clone()
    }

    fn cache_ttl() -> u64 {
        86400 // 1 day for relationships (rarely change)
    }

    fn is_public(&self) -> bool {
        true // Relationships are always public metadata
    }
}

// =============================================================================
// Human Relationships (Qahal - Social Graph)
// =============================================================================

/// Intimacy levels for human relationships (determines custody and access rights)
pub const INTIMACY_LEVELS: [&str; 5] = [
    "intimate",    // Spouse, immediate family, closest confidants (auto-custody enabled)
    "trusted",     // Extended family, very close friends (custody by consent)
    "familiar",    // Friends, colleagues, regular contacts (no auto-custody)
    "acquainted",  // People you know but don't interact with regularly
    "public",      // Known publicly but no personal relationship
];

/// Human relationship types (social bonds between agents)
pub const HUMAN_RELATIONSHIP_TYPES: [&str; 12] = [
    "spouse",           // Married/life partner (intimate)
    "parent",           // Parent-child (intimate)
    "child",            // Parent-child (intimate)
    "sibling",          // Brother/sister (intimate or trusted)
    "grandparent",      // Grandparent-grandchild (trusted)
    "grandchild",       // Grandparent-grandchild (trusted)
    "extended-family",  // Aunt, uncle, cousin (trusted or familiar)
    "trusted-friend",   // Close friend (trusted)
    "colleague",        // Work colleague (familiar)
    "neighbor",         // Geographic proximity (familiar)
    "community-member", // Same community/church/organization (familiar)
    "acquaintance",     // Known but not close (acquainted)
];

/// Human-to-human relationship for social graph and custody
/// Enables multi-tier replication: family network backup + emergency access
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HumanRelationship {
    pub id: String,

    // === PARTIES ===
    pub party_a_id: String,        // Agent ID of first party
    pub party_b_id: String,        // Agent ID of second party

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
    pub auto_custody_enabled: bool,     // Should intimate/private content auto-replicate?
    pub shared_encryption_key_id: Option<String>, // For family-shared content
    pub emergency_access_enabled: bool, // Can this relationship trigger emergency recovery?

    // === METADATA ===
    pub initiated_by: String,      // Which party initiated (party_a_id or party_b_id)
    pub verified_at: Option<String>, // When both parties confirmed
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>, // Optional expiration (e.g., temporary trust)

    // === CONTEXT ===
    pub context_json: Option<String>, // Additional metadata: how they met, shared interests, etc.
    pub reach: String,                // Visibility of relationship itself (private, local, public)
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
    // Self-healing DNA fields
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
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
// Shefa: Insurance Mutual - Member Risk Profile
// =============================================================================

/// MemberRiskProfile - Behavioral risk assessment for autonomous mutual insurance.
///
/// Core of Elohim Mutual: actual behavioral observation instead of proxies like credit scores.
/// Uses Observer protocol attestations for three factors: care, connectedness, claims history.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MemberRiskProfile {
    pub id: String,
    pub member_id: String,              // Agent ID of member
    pub risk_type: String,              // health, property, casualty, care
    // Behavioral scores (0-100)
    pub care_maintenance_score: f64,    // Preventive care frequency
    pub community_connectedness_score: f64, // Support network quality
    pub historical_claims_rate: f64,    // Claims frequency (0.0-1.0)
    // Calculated risk
    pub risk_score: f64,                // Weighted average (0-100)
    pub risk_tier: String,              // low, standard, high, uninsurable (from RISK_TIERS)
    pub risk_tier_rationale: String,    // Human-readable explanation
    // Evidence trail
    pub evidence_event_ids_json: String, // Observer attestation IDs (Vec<String> as JSON)
    pub evidence_breakdown_json: String, // Count by type: {careEvents, communityEvents, claimsEvents}
    // Trending
    pub risk_trend_direction: String,   // improving, stable, declining (from RISK_TRENDS)
    pub last_risk_score: f64,           // Score from previous assessment
    // Assessment tracking
    pub assessed_at: String,
    pub last_assessment_at: String,
    pub next_assessment_due: String,
    pub assessment_event_ids_json: String, // EconomicEvent IDs (Vec<String> as JSON)
    // Schema & validation
    pub schema_version: u32,
    pub validation_status: String,      // Valid, Migrated, Degraded, Healing
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Risk tiers for insurance mutual
pub const RISK_TIERS: [&str; 4] = [
    "low",          // Excellent preventive care, strong support
    "standard",     // Average care, moderate support
    "high",         // Poor care or weak support
    "uninsurable",  // Too risky for coverage
];

/// Risk trend directions
pub const RISK_TRENDS: [&str; 3] = [
    "improving",
    "stable",
    "declining",
];

// =============================================================================
// Shefa: Insurance Mutual - Coverage Policy
// =============================================================================

/// CoveragePolicy - What risks are covered and under what terms.
///
/// Governs cost-sharing: deductible, coinsurance, out-of-pocket maximum.
/// Created at governance level (individual, household, community, network, constitutional).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CoveragePolicy {
    pub id: String,
    pub member_id: String,
    // Coverage definition
    pub coverage_level: String,         // individual, household, community, network
    pub governed_at: String,            // Which Qahal (or network)
    pub covered_risks_json: String,     // CoveredRisk[] as JSON
    // Cost sharing
    pub deductible_value: Option<f64>,
    pub deductible_unit: Option<String>,
    pub coinsurance: f64,               // Percentage (0-100)
    pub out_of_pocket_maximum_value: Option<f64>,
    pub out_of_pocket_maximum_unit: Option<String>,
    // Effective dates
    pub effective_from: String,
    pub renewal_terms: String,          // annual, semi-annual, etc.
    pub renewal_due_at: String,
    // Constitutional basis
    pub constitutional_basis: String,   // Reference to governance document
    // Premium tracking
    pub last_premium_event_id: Option<String>,
    pub last_premium_paid_at: Option<String>,
    // Schema & validation
    pub schema_version: u32,
    pub validation_status: String,
    pub modification_event_ids_json: String, // EconomicEvent IDs (Vec<String> as JSON)
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// CoveredRisk - A single risk that's covered
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CoveredRisk {
    pub id: String,
    pub risk_name: String,              // "Emergency Medical", "Prescription Medications"
    pub risk_description: String,
    pub risk_category: String,          // health, property, casualty, care
    pub is_covered: bool,
    pub coverage_limit_value: Option<f64>,
    pub coverage_limit_unit: Option<String>,
    pub deductible_applies: bool,
    pub coinsurance_percent: f64,       // 0-100
    pub prevention_incentive_applies: bool,
    pub created_at: String,
}

/// Coverage levels (governance layers for policy decisions)
pub const COVERAGE_LEVELS: [&str; 5] = [
    "individual",
    "household",
    "community",
    "network",
    "constitutional",
];

// =============================================================================
// Shefa: Insurance Mutual - Claims
// =============================================================================

/// InsuranceClaim - A member's claim for coverage.
///
/// Full lifecycle: filed → investigated → adjusted → approved/denied → settled/appealed.
/// Immutable event trail for transparency and governance.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InsuranceClaim {
    pub id: String,
    pub claim_number: String,           // Human-readable: CLM-XXXXXXXXXX
    pub policy_id: String,
    pub member_id: String,
    // Filing
    pub filed_date: String,
    pub filed_by: String,
    // Loss details
    pub loss_type: String,              // Risk name (e.g., "Emergency Medical")
    pub loss_date: String,
    pub description: String,
    pub estimated_amount_value: Option<f64>,
    pub estimated_amount_unit: Option<String>,
    // Evidence
    pub observer_attestation_ids_json: String, // Vec<String> as JSON
    pub member_document_ids_json: String,      // Vec<String> as JSON
    // Status & history
    pub status: String,                 // filed, adjustment-made, approved, denied, settled, appealed (from CLAIM_STATUSES)
    pub status_history_json: String,    // ClaimStatusChange[] as JSON
    // Adjustments
    pub adjustment_event_ids_json: String, // EconomicEvent IDs (Vec<String> as JSON)
    pub appeal_event_ids_json: String,
    pub settlement_event_ids_json: String,
    // Schema & validation
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,          // {coveredRiskId, deductibleApplies, coinsurancePercent, coverageLimit}
    pub created_at: String,
    pub updated_at: String,
}

/// Claim statuses
pub const CLAIM_STATUSES: [&str; 7] = [
    "filed",
    "adjustment-made",
    "approved",
    "denied",
    "settled",
    "appealed",
    "resolved",
];

// =============================================================================
// Shefa: Insurance Mutual - Adjustment Reasoning (Bob Parr Principle)
// =============================================================================

/// AdjustmentReasoning - Adjuster's determination with full constitutional reasoning.
///
/// Core of the Bob Parr Principle: every decision must be explained in plain language
/// and cite the constitutional basis. Auditable by governance.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AdjustmentReasoning {
    pub id: String,
    pub claim_id: String,
    pub adjuster_id: String,
    // Decision
    pub coverage_decision: String,      // approved, denied, partial (from COVERAGE_DECISIONS)
    pub approved_amount_value: Option<f64>,
    pub approved_amount_unit: Option<String>,
    // Reasoning
    pub plain_language_explanation: String, // Member-facing explanation
    pub interpretation_notes: Option<String>,
    pub applied_generosity_principle: bool, // Did we interpret ambiguously in member's favor?
    // Constitutional basis
    pub constitutional_basis_documents_json: String, // Document references (Vec<String> as JSON)
    pub policy_citations_json: String,  // Specific policy sections (Vec<String> as JSON)
    // Governance review
    pub flagged_for_governance: bool,
    pub governance_review_reason: Option<String>,
    // Timestamps
    pub adjustment_date: String,
    pub created_at: String,
}

/// Coverage decision types
pub const COVERAGE_DECISIONS: [&str; 3] = [
    "approved",
    "denied",
    "partial",
];

// =============================================================================
// Shefa: Requests & Offers - Service Request
// =============================================================================

/// ServiceRequest - Someone requesting a service (REA Intent: take action).
///
/// Part of the peer-to-peer marketplace for coordinating work and services.
/// Initially pending admin approval before becoming visible.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ServiceRequest {
    pub id: String,
    pub request_number: String,         // REQ-XXXXXXXXXX
    pub requester_id: String,
    // Content
    pub title: String,
    pub description: String,
    // Contact & timing
    pub contact_preference: String,     // email, phone, message, in-person (from CONTACT_PREFERENCES)
    pub contact_value: String,
    pub time_zone: String,
    pub time_preference: String,        // morning, afternoon, evening, any (from TIME_PREFERENCES)
    pub interaction_type: String,       // virtual, in-person, hybrid (from INTERACTION_TYPES)
    pub date_range_start: String,
    pub date_range_end: Option<String>,
    // Service details
    pub service_type_ids_json: String,  // Vec<String> as JSON
    pub required_skills_json: String,   // Vec<String> as JSON
    pub budget_value: Option<f64>,
    pub budget_unit: Option<String>,
    pub medium_of_exchange_ids_json: String, // Vec<String> as JSON
    // Visibility
    pub status: String,                 // pending, active, archived, deleted (from REQUEST_STATUSES)
    pub is_public: bool,                // Hidden until admin approval
    pub links_json: String,             // Vec<String> as JSON - portfolio, docs, etc.
    // Schema & validation
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// ServiceOffer - Someone offering a service (REA Intent: give action).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ServiceOffer {
    pub id: String,
    pub offer_number: String,           // OFR-XXXXXXXXXX
    pub offeror_id: String,
    // Content
    pub title: String,
    pub description: String,
    // Contact & availability
    pub contact_preference: String,
    pub contact_value: String,
    pub time_zone: String,
    pub time_preference: String,
    pub interaction_type: String,
    pub hours_per_week: f64,
    pub date_range_start: String,
    pub date_range_end: Option<String>,
    // Service details
    pub service_type_ids_json: String,  // Vec<String> as JSON
    pub offered_skills_json: String,    // Vec<String> as JSON
    pub rate_value: f64,
    pub rate_unit: String,              // unit token
    pub rate_per: String,               // hour, day, week, month, project
    pub medium_of_exchange_ids_json: String, // Vec<String> as JSON
    pub accepts_alternative_payment: bool,
    // Visibility
    pub status: String,                 // pending, active, archived, deleted
    pub is_public: bool,
    pub links_json: String,             // Vec<String> as JSON
    // Schema & validation
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Request/Offer statuses
pub const REQUEST_STATUSES: [&str; 4] = [
    "pending",
    "active",
    "archived",
    "deleted",
];

/// Contact preferences
pub const CONTACT_PREFERENCES: [&str; 4] = [
    "email",
    "phone",
    "message",
    "in-person",
];

/// Time preferences
pub const TIME_PREFERENCES: [&str; 4] = [
    "morning",
    "afternoon",
    "evening",
    "any",
];

/// Interaction types
pub const INTERACTION_TYPES: [&str; 3] = [
    "virtual",
    "in-person",
    "hybrid",
];

// =============================================================================
// Shefa: Requests & Offers - Service Match
// =============================================================================

/// ServiceMatch - A suggested or confirmed match between request and offer.
///
/// Lifecycle: suggested → contacted → negotiating → agreed → completed.
/// Algorithmic or manual matching based on compatibility.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ServiceMatch {
    pub id: String,
    pub request_id: String,
    pub offer_id: String,
    // Matching details
    pub match_reason: String,
    pub match_quality: u32,             // 0-100 compatibility score
    pub shared_service_types_json: String, // Vec<String> as JSON
    pub time_compatible: bool,
    pub interaction_compatible: bool,
    pub exchange_compatible: bool,
    // Status
    pub status: String,                 // suggested, contacted, negotiating, agreed, completed (from MATCH_STATUSES)
    // Coordination
    pub proposal_id: Option<String>,    // REA Proposal ID if exists
    pub commitment_id: Option<String>,  // REA Commitment ID if agreed
    // Schema & validation
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Match statuses (lifecycle)
pub const MATCH_STATUSES: [&str; 5] = [
    "suggested",
    "contacted",
    "negotiating",
    "agreed",
    "completed",
];

// =============================================================================
// Shefa: Stewarded Resources (Resource Stewardship & Accountability)
// =============================================================================

/// StewardedResource - A resource tracked and managed by a human
/// Tracks capacity, allocation, usage, and governance
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StewardedResource {
    pub id: String,
    pub resource_number: String,          // RES-XXXXXXXXXX
    pub steward_id: String,               // Agent ID who stewards
    pub category: String,                 // energy, compute, water, food, shelter, etc.
    pub subcategory: String,
    pub name: String,
    pub description: Option<String>,

    // Capacity & Measurement
    pub dimension_unit: String,           // kWh, GB, liters, etc.
    pub dimension_label: String,
    pub total_capacity_value: f64,
    pub total_capacity_unit: String,
    pub allocatable_capacity_value: f64,
    pub allocatable_capacity_unit: String,

    // Current State
    pub total_allocated_value: f64,
    pub total_used_value: f64,
    pub available_value: f64,

    // Governance
    pub governance_level: String,         // individual, household, community, network, constitutional
    pub governed_by: Option<String>,
    pub can_modify_allocations: bool,

    // Allocations & Usage
    pub allocations_json: String,         // AllocationBlock[] as JSON
    pub allocation_strategy: String,      // manual, automatic, hybrid
    pub recent_usage_json: String,        // UsageRecord[] as JSON
    pub trends_json: String,              // ResourceTrend[] as JSON

    // Tracking
    pub observer_enabled: bool,
    pub observer_agent_id: Option<String>,
    pub is_shared: bool,
    pub visibility: String,               // private, household, community, public
    pub data_quality: String,             // measured, estimated, manual, mixed
    pub last_verified_at: Option<String>,

    // Economic Integration
    pub resource_spec_id: Option<String>, // hREA ResourceSpecification
    pub commons_pool_id: Option<String>,
    pub allocation_event_ids_json: String, // EconomicEvent IDs
    pub usage_event_ids_json: String,

    // Metadata
    pub schema_version: u32,
    pub validation_status: String,        // Valid, Migrated, Degraded, Healing
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// FinancialAsset - Tracking of money, investments, obligations
/// Supports transparency about financial situation and UBA/UBI eligibility
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FinancialAsset {
    pub id: String,
    pub steward_id: String,
    pub asset_type: String,               // fiat-currency, mutual-credit, crypto, stock, bond, etc.
    pub currency_code: Option<String>,    // USD, EUR, BTC
    pub account_number_hash: Option<String>,  // Hashed for privacy
    pub account_institution: Option<String>,

    // Account State
    pub account_balance: f64,
    pub available_balance: f64,
    pub account_status: String,           // active, frozen, closed
    pub pending_transactions_json: String, // Transaction[] as JSON

    // Income Streams
    pub income_streams_json: String,      // IncomeStream[] as JSON
    pub monthly_income: f64,              // Guaranteed income
    pub expected_monthly_income: f64,     // With uncertain streams

    // Obligations & Liabilities
    pub obligations_json: String,         // FinancialObligation[] as JSON
    pub total_liability: f64,
    pub monthly_obligations: f64,

    // UBA/UBI Status
    pub uba_eligible: bool,
    pub uba_status: String,               // active, pending, paused, inactive
    pub uba_monthly_amount: Option<f64>,
    pub uba_last_payment: Option<String>,

    // Financial Health
    pub burn_rate: Option<f64>,           // Per day
    pub runway_days: Option<u32>,         // Days until depleted
    pub debt_to_income_ratio: Option<f64>,
    pub credit_score: Option<u32>,
    pub confidence_score: u32,            // Data confidence 0-100

    // Verification
    pub data_source: String,              // bank-api, manual, blockchain, mixed
    pub last_verified_at: Option<String>,
    pub verification_method: Option<String>,

    // Metadata
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// UBAEligibility - Tracks if human qualifies for Universal Basic Assets/Income
/// Constitutional floor of dignity while transitioning to Elohim economy
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct UBAEligibility {
    pub id: String,
    pub human_id: String,
    pub eligible: bool,
    pub eligibility_reason: Option<String>,

    // Entitlements
    pub basic_assets_json: String,        // BasicAssetEntitlement[] as JSON
    pub basic_income_json: Option<String>, // BasicIncomeEntitlement as JSON
    pub dignity_floor_json: String,       // DignityFloor as JSON

    // Verification
    pub verified_at: Option<String>,
    pub verified_by: Option<String>,      // Governance entity
    pub documentation_links_json: String, // String[] as JSON

    // Metadata
    pub schema_version: u32,
    pub validation_status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// AccountingBridge - Bridge between legacy accounting and hREA/ValueFlows
///
/// This entry type acts as the "AccountingBridge module" - mapping legacy
/// financial assets (stocks, bonds, CoDs, AR/AP) to hREA ResourceSpecification
/// for transparent economic coordination.
///
/// Enables:
/// - Transparent portfolio tracking
/// - Equity asset stewardship
/// - Accounts receivable/payable in hREA
/// - Financial statements through hREA primitives
/// - Full economic integration of all assets
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AccountingBridge {
    pub id: String,
    pub agent_id: String,                 // Who owns this asset
    pub asset_name: String,               // "Apple Stock", "US Treasury Bond", etc.
    pub asset_type: String,               // Maps to FINANCIAL_ASSET_TYPES
    pub asset_category: String,           // equity, debt, receivable, payable, real-estate

    // Core Asset Data
    pub quantity: f64,                    // Number of shares, par value, etc.
    pub unit: String,                     // "shares", "dollars", "units"
    pub current_value: f64,               // Current market/book value
    pub cost_basis: f64,                  // Original purchase price
    pub currency_code: String,            // USD, EUR, etc.

    // Equity Assets (stocks, bonds, mutual funds, ETFs)
    pub ticker_symbol: Option<String>,    // AAPL, MSFT, etc.
    pub exchange: Option<String>,         // NASDAQ, NYSE, etc.
    pub industry: Option<String>,         // Sector/industry classification

    // Debt Assets (bonds, CoDs, notes)
    pub maturity_date: Option<String>,    // ISO 8601 date
    pub coupon_rate: Option<f64>,         // Interest rate
    pub par_value: Option<f64>,           // Face value
    pub credit_rating: Option<String>,    // AAA, A, BBB, etc.

    // Receivables & Payables
    pub debtor_id: Option<String>,        // For AR: who owes money
    pub creditor_id: Option<String>,      // For AP: who we owe
    pub due_date: Option<String>,
    pub terms: Option<String>,            // Net-30, Net-60, etc.

    // Real Estate
    pub property_address: Option<String>,
    pub property_type: Option<String>,    // residential, commercial, land
    pub square_feet: Option<f64>,
    pub mortgage_balance: Option<f64>,    // If mortgaged
    pub loan_terms: Option<String>,

    // hREA Integration
    pub resource_spec_id: String,         // Maps to hREA ResourceSpecification
    pub commitment_ids_json: String,      // hREA Commitments
    pub event_ids_json: String,           // Economic events tracking changes

    // Accounting Records
    pub acquisition_date: String,
    pub acquisition_event_id: Option<String>, // EconomicEvent when acquired
    pub last_valuation_date: String,
    pub last_valuation_event_id: Option<String>, // EconomicEvent for revaluation
    pub dividend_or_interest_events_json: String, // Vec<String> - dividend/interest payments

    // Governance & Transparency
    pub governance_level: String,         // individual, household, community
    pub visibility: String,               // private, household, public
    pub account_number_hash: Option<String>, // Hashed for privacy
    pub custodian_id: Option<String>,     // Who holds/manages (brokerage, etc.)

    // Verification & Quality
    pub data_source: String,              // manual, api, blockchain, third-party
    pub last_verified_at: Option<String>,
    pub verification_source: Option<String>, // Where data came from

    // Accounting Calculations
    pub unrealized_gain_loss: f64,        // Current value - cost basis
    pub unrealized_gain_loss_percent: f64,
    pub tax_lot_details_json: String,     // For complex tax calculations

    // Metadata
    pub schema_version: u32,
    pub validation_status: String,        // Valid, Migrated, Degraded, Healing
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

// Constants for Financial/Resource Status
/// FinancialAsset types - supporting equity, debt, and hREA bridge
/// These map to hREA ResourceSpecification for economic coordination
pub const FINANCIAL_ASSET_TYPES: [&str; 15] = [
    // Liquid assets
    "fiat-currency",
    "mutual-credit",
    "cryptocurrency",

    // Equity assets (stocks, bonds)
    "stock",
    "bond",
    "cod",                    // Certificate of Deposit
    "mutual-fund",
    "etf",

    // Real estate & property
    "property-equity",
    "real-estate-investment-trust",

    // Receivables & obligations
    "account-receivable",     // Money owed TO you
    "account-payable",        // Money owed BY you (liability)
    "promissory-note",

    // Other
    "debt",
    "other",
];

pub const ACCOUNT_STATUSES: [&str; 3] = ["active", "frozen", "closed"];

pub const UBA_STATUSES: [&str; 4] = ["active", "pending", "paused", "inactive"];

pub const RESOURCE_CATEGORIES: [&str; 13] = [
    "energy",
    "compute",
    "water",
    "food",
    "shelter",
    "transportation",
    "property",
    "equipment",
    "inventory",
    "knowledge",
    "reputation",
    "financial-asset",
    "uba",
];

pub const GOVERNANCE_LEVELS: [&str; 5] = [
    "individual",
    "household",
    "community",
    "network",
    "constitutional",
];

pub const ENFORCEMENT_METHODS: [&str; 3] = [
    "voluntary",   // Voluntary compliance, no enforcement
    "progressive", // Incentive-based, gentle nudges
    "hard",        // Mandatory enforcement with hard stops
];

pub const TRANSITION_STATUSES: [&str; 6] = [
    "proposal",      // Initial proposal
    "negotiating",   // In negotiation phase
    "agreed",        // Agreement reached
    "executing",     // Execution in progress
    "completed",     // Transition complete
    "blocked",       // Blocked or disputed
];

pub const PHASE_STATUSES: [&str; 4] = [
    "pending",      // Waiting to start
    "in_progress",  // Currently executing
    "completed",    // Successfully completed
    "blocked",      // Failed or blocked
];

pub const ACTION_TYPES: [&str; 7] = [
    "sell",       // Sell asset
    "transfer",   // Transfer to another holder
    "liquidate",  // Convert to liquid currency
    "convert",    // Convert to different asset type
    "register",   // Register with governance
    "authorize",  // Get authorization
    "other",      // Other action type
];

pub const POSITION_TYPES: [&str; 5] = [
    "below-floor",
    "at-floor",
    "in-safe-zone",
    "above-ceiling",
    "far-above-ceiling",
];

// =============================================================================
// Shefa: Constitutional Limits & Resource Position
// =============================================================================

/// ConstitutionalLimit - Defines floor and ceiling bounds for resources
///
/// Implements donut economy and limitarianism principles:
/// - FLOOR: Dignity minimum (constitutional entitlement)
/// - CEILING: Constitutional maximum (beyond which extractive)
/// - SAFE ZONE: Healthy operating space between floor and ceiling
///
/// Enables constitutional compliance and transition to community stewardship.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ConstitutionalLimit {
    pub id: String,
    pub category: String,                    // Maps to RESOURCE_CATEGORIES
    pub dimension: String,                   // e.g., "dollars", "hours", "cpu-cores"

    // Floor and Ceiling Values
    pub floor_value: f64,                    // Dignity minimum
    pub floor_rationale: String,             // Why this floor?
    pub ceiling_value: f64,                  // Constitutional maximum
    pub ceiling_rationale: String,           // Why this ceiling?

    // Safe Operating Space
    pub safe_min_value: f64,                 // Recommended minimum
    pub safe_max_value: f64,                 // Recommended maximum

    // Enforcement
    pub enforcement_method: String,          // voluntary, progressive, hard
    pub transition_deadline: String,         // ISO 8601 date for voluntary → progressive
    pub hard_stop_date: Option<String>,      // ISO 8601 date for hard enforcement

    // Constitutional Basis
    pub constitutional_basis_key: String,    // Reference to constitutional principle
    pub governance_level: String,            // Which governance level sets this?
    pub governed_by: String,                 // Entity ID of governing body

    // Metadata
    pub schema_version: u32,
    pub validation_status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// ResourcePosition - Assessment of where a resource stands relative to constitutional bounds
///
/// Answers: "Is this resource within constitutional limits?"
/// Identifies excess holdings and enables transition planning.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ResourcePosition {
    pub id: String,
    pub resource_id: String,                 // StewardedResource this assesses
    pub limit_id: String,                    // ConstitutionalLimit applied
    pub assessment_date: String,             // When assessed (ISO 8601)

    // Position Assessment
    pub position_type: String,               // below-floor, at-floor, in-safe-zone, above-ceiling, far-above-ceiling
    pub current_value: f64,                  // Current amount of resource
    pub distance_from_floor: f64,            // How far below/above floor (negative = below)
    pub distance_from_ceiling: f64,          // How far below/above ceiling (negative = below)

    // Excess Calculation
    pub excess_above_ceiling: f64,           // Amount exceeding ceiling (0 if below)
    pub excess_percentage: f64,              // Percentage above ceiling (0 if below)
    pub surplus_available_for_transition: f64, // Recommended transition amount

    // Compliance Status
    pub compliant: bool,                     // Is this position compliant?
    pub warning_level: String,               // "none", "caution", "warning", "critical"
    pub days_to_hard_stop: Option<i32>,      // Days until hard enforcement (if applicable)

    // Transition Info
    pub has_active_transition: bool,         // Is there an active transition path?
    pub transition_path_id: Option<String>,  // If yes, which one?
    pub transition_status: Option<String>,   // Current transition status

    // Metadata
    pub schema_version: u32,
    pub validation_status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// TransitionPath - Structured process for moving excess assets to community stewardship
///
/// Enables constitutional compliance through:
/// - Transparent negotiation of splits
/// - Phased execution with governance oversight
/// - Immutable event recording
/// - Reputation/governance credit tracking
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TransitionPath {
    pub id: String,
    pub steward_id: String,                  // Who's transitioning assets
    pub resource_id: String,                 // Which resource/asset
    pub limit_id: String,                    // Which constitutional limit applies

    // Asset Details
    pub current_value: f64,                  // Total asset value
    pub constitutional_ceiling: f64,         // Ceiling this asset should follow
    pub excess_amount: f64,                  // Amount to transition

    // Proposed Splits (how to divide the excess)
    pub proposed_splits_json: String,        // Vec<AssetSplit> as JSON

    // Status & Timeline
    pub status: String,                      // proposal, negotiating, agreed, executing, completed, blocked
    pub initiated_at: String,                // ISO 8601 timestamp
    pub negotiation_deadline: String,        // 90 days from initiation by default
    pub execution_start_date: Option<String>,
    pub target_completion_date: Option<String>,
    pub actual_completion_date: Option<String>,

    // Governance & Oversight
    pub governance_level: String,            // Which level has oversight?
    pub governing_body: String,              // Entity ID (e.g., household council, community court)
    pub approval_status: String,             // pending, approved, rejected
    pub approval_date: Option<String>,
    pub approved_by: Option<String>,         // Who approved?

    // Transparency
    pub visibility: String,                  // private, household, community, public
    pub rationale: String,                   // Why this transition path?

    // Execution Phases
    pub phases_json: String,                 // Vec<TransitionPhase> as JSON
    pub current_phase: u32,                  // Which phase are we executing?

    // Immutable Records
    pub transition_event_ids_json: String,   // Vec<String> - EconomicEvent IDs
    pub governance_proposal_event_id: Option<String>,

    // Block/Dispute Info
    pub block_reason: Option<String>,
    pub blocked_at: Option<String>,
    pub disputed: bool,

    // Metadata
    pub schema_version: u32,
    pub validation_status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// AssetSplit - How excess asset is divided among destinations
///
/// Each split represents one recipient of the excess asset.
/// Examples:
/// - 60% to Commons Pool
/// - 30% to Community Benefit Corp
/// - 10% to Land Trust
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AssetSplit {
    pub id: String,
    pub transition_path_id: String,
    pub split_name: String,                  // e.g., "Commons Pool", "Community Benefit Corp"
    pub destination_type: String,            // individual, organization, commons-pool, trust, coop, etc.
    pub destination_id: String,              // Recipient entity ID
    pub amount: f64,                         // Amount for this split
    pub percentage: f64,                     // Percentage of excess (0-100)

    // Governance Role
    pub legacy_role: Option<String>,         // If steward keeps a role (e.g., board member)
    pub legacy_role_details: Option<String>, // Details about the maintained role

    // Status & Timeline
    pub status: String,                      // pending, agreed, executing, completed, blocked
    pub agreed_at: Option<String>,
    pub completed_at: Option<String>,

    // Rationale & Transparency
    pub rationale: String,                   // Why this split?
    pub terms: Option<String>,               // Any special terms?
    pub conditions: Option<String>,          // Any conditions for this split?

    // Metadata
    pub schema_version: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// TransitionPhase - Sequential phase in the execution of a transition
///
/// Each phase represents a milestone in converting excess assets to community stewardship.
/// Example phases:
/// 1. "Get appraisals" → determine actual value
/// 2. "Receive board approvals" → governance sign-off
/// 3. "Execute transfers" → move assets
/// 4. "Update registrations" → legal/official updates
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TransitionPhase {
    pub id: String,
    pub transition_path_id: String,
    pub sequence_number: u32,                // Execution order
    pub name: String,                        // e.g., "Get Appraisals", "Board Approval", "Execute Transfer"
    pub description: String,                 // What happens in this phase?

    // Timeline
    pub target_start_date: String,           // ISO 8601
    pub target_end_date: String,
    pub actual_start_date: Option<String>,
    pub actual_end_date: Option<String>,

    // Amount in This Phase
    pub amount_in_phase: f64,                // How much transitions in this phase

    // Actions to Execute
    pub actions_json: String,                // Vec<TransitionAction> as JSON
    pub total_actions: u32,
    pub completed_actions: u32,
    pub failed_actions: u32,

    // Status
    pub status: String,                      // pending, in_progress, completed, blocked
    pub block_reason: Option<String>,        // If blocked, why?
    pub block_date: Option<String>,

    // Metadata
    pub schema_version: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// TransitionAction - Specific action within a phase
///
/// Examples:
/// - "Sell 100 shares of AAPL"
/// - "Transfer to Community Benefit Corp"
/// - "Register with SEC"
/// - "Update broker account"
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TransitionAction {
    pub id: String,
    pub phase_id: String,
    pub action_type: String,                 // sell, transfer, liquidate, convert, register, authorize, other
    pub action_description: String,          // What specifically to do

    // Responsibility
    pub responsible_party: String,           // Who executes? (agent ID or role)
    pub assigned_to: Option<String>,         // Person/entity assigned

    // Timeline
    pub target_date: String,                 // When should this happen?
    pub actual_date: Option<String>,         // When did it happen?
    pub deadline_critical: bool,             // Is this a hard deadline?

    // Status & Results
    pub status: String,                      // pending, completed, failed
    pub completion_notes: Option<String>,    // How did it go?
    pub failure_reason: Option<String>,      // If failed, why?

    // Immutable Record
    pub economic_event_id: Option<String>,   // EconomicEvent recording this action

    // Metadata
    pub schema_version: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// CommonsContribution - Recognition when asset transitions to community stewardship
///
/// Tracks the steward's contribution to commons and enables:
/// - Governance credit attribution
/// - Public recognition (if desired)
/// - Historical record of community building
/// - Future claim to benefits from commons
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CommonsContribution {
    pub id: String,
    pub steward_id: String,                  // Who made the contribution
    pub transition_path_id: String,          // Which transition path
    pub asset_split_id: String,              // Which split (if to commons)

    // Contribution Details
    pub original_holding: f64,                // What they originally held
    pub contributed_amount: f64,             // What they contributed
    pub contribution_date: String,           // ISO 8601 timestamp

    // Commons Pool
    pub destination_commons_pool: String,    // Which commons pool received this
    pub commons_pool_id: String,             // Entity ID

    // Governance Credit
    pub governance_credit_amount: f64,       // How much governance credit earned
    pub governance_credit_category: String,  // Type of credit (reputation, voting power, etc.)

    // Legacy Role
    pub legacy_role: Option<String>,         // If they maintain a role
    pub legacy_role_details: Option<String>, // Details of maintained role

    // Recognition
    pub public_recognition: bool,            // May we publicly recognize this?
    pub recognition_statement: Option<String>, // How to describe it publicly
    pub recognition_date: Option<String>,    // When published

    // Immutable Record
    pub economic_event_id: String,           // EconomicEvent recording contribution
    pub governance_proposal_id: Option<String>,

    // Metadata
    pub schema_version: u32,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Shefa: Flow Planning, Simulation, and Budgeting
// =============================================================================

/// FlowPlan - Top-level planning entity
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FlowPlan {
    pub id: String,
    pub plan_number: String,                 // FP-XXXXXXXXXX
    pub steward_id: AgentPubKey,
    pub name: String,
    pub description: Option<String>,
    pub time_horizon: String,                // TimeHorizon as string
    pub plan_period_start: Timestamp,
    pub plan_period_end: Timestamp,
    pub resource_scopes: Vec<String>,        // ResourceCategory[]
    pub included_resource_ids: Vec<String>,
    pub goals: Vec<String>,                  // FlowGoal IDs
    pub milestones: Vec<String>,             // FlowMilestone IDs
    pub budgets: Vec<String>,                // FlowBudget IDs
    pub status: String,                      // PlanStatus
    pub confidence_score: u8,                // 0-100
    pub completion_percent: u8,              // 0-100
    pub created_at: Timestamp,
    pub activated_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    pub last_reviewed_at: Option<Timestamp>,
    pub next_review_due: Timestamp,
    pub plan_event_ids_json: String,         // Vec<String> as JSON
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
}

/// FlowBudget - Prescriptive allocation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FlowBudget {
    pub id: String,
    pub budget_number: String,               // FB-XXXXXXXXXX
    pub plan_id: String,
    pub steward_id: AgentPubKey,
    pub name: String,
    pub description: Option<String>,
    pub budget_period: String,               // weekly|monthly|quarterly|annual
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub categories_json: String,             // BudgetCategory[] as JSON
    pub total_planned: f64,
    pub total_actual: f64,
    pub variance: f64,
    pub variance_percent: f64,
    pub status: String,                      // BudgetStatus
    pub health_status: String,               // healthy|warning|critical
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub last_reconciled: Timestamp,
    pub budget_event_ids_json: String,       // Vec<String> as JSON
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowGoal - Specific target to achieve
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FlowGoal {
    pub id: String,
    pub goal_number: String,                 // FG-XXXXXXXXXX
    pub plan_id: String,
    pub steward_id: AgentPubKey,
    pub name: String,
    pub description: Option<String>,
    pub goal_type: String,                   // savings|debt-reduction|income-increase|allocation-shift|milestone|custom
    pub target_metric: String,
    pub target_value: f64,
    pub target_unit: String,
    pub current_value: f64,
    pub starting_value: f64,
    pub deadline: Timestamp,
    pub progress_percent: u8,                // 0-100
    pub on_track: bool,
    pub estimated_completion_date: Option<Timestamp>,
    pub linked_resource_ids_json: String,    // Vec<String> as JSON
    pub linked_budget_ids_json: String,      // Vec<String> as JSON
    pub blocked_by_json: String,             // Vec<String> as JSON
    pub status: String,                      // GoalStatus
    pub created_at: Timestamp,
    pub started_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    pub goal_event_ids_json: String,         // Vec<String> as JSON
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowMilestone - Key checkpoint in plan
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FlowMilestone {
    pub id: String,
    pub milestone_number: String,            // FM-XXXXXXXXXX
    pub plan_id: String,
    pub name: String,
    pub description: Option<String>,
    pub target_date: Timestamp,
    pub actual_date: Option<Timestamp>,
    pub success_criteria_json: String,       // MilestoneSuccessCriterion[] as JSON
    pub all_criteria_met: bool,
    pub depends_on_goals_json: String,       // Vec<String> as JSON
    pub depends_on_milestones_json: String,  // Vec<String> as JSON
    pub blocks_goals_json: String,           // Vec<String> as JSON
    pub status: String,                      // MilestoneStatus
    pub achieved_at: Option<Timestamp>,
    pub milestone_event_ids_json: String,    // Vec<String> as JSON
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowScenario - What-if simulation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FlowScenario {
    pub id: String,
    pub scenario_number: String,             // FS-XXXXXXXXXX
    pub plan_id: String,
    pub steward_id: AgentPubKey,
    pub name: String,
    pub description: Option<String>,
    pub scenario_type: String,               // optimistic|pessimistic|baseline|target|what-if
    pub changes_json: String,                // ScenarioChange[] as JSON
    pub projections_json: String,            // Vec<String> as JSON (FlowProjection IDs)
    pub baseline_scenario_id: Option<String>,
    pub delta_metrics_json: String,          // Record<String, f64> as JSON
    pub status: String,                      // ScenarioStatus
    pub simulated_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub scenario_event_ids_json: String,     // Vec<String> as JSON
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowProjection - Time series forecast
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FlowProjection {
    pub id: String,
    pub projection_number: String,           // FP-XXXXXXXXXX
    pub plan_id: Option<String>,
    pub scenario_id: Option<String>,
    pub steward_id: AgentPubKey,
    pub resource_category: String,
    pub resource_id: Option<String>,
    pub projection_start: Timestamp,
    pub projection_end: Timestamp,
    pub projection_horizon: String,          // TimeHorizon
    pub data_points_json: String,            // ProjectionDataPoint[] as JSON
    pub confidence_level: String,            // low|medium|high
    pub confidence_percent: u8,              // 0-100
    pub projection_method: String,           // trend-extrapolation|pattern-based|scenario-driven|constraint-optimized
    pub assumptions_json: String,            // Vec<String> as JSON
    pub breakpoints_json: String,            // ProjectionBreakpoint[] as JSON
    pub created_at: Timestamp,
    pub schema_version: u32,
    pub validation_status: String,
}

/// RecurringPattern - Life cadence modeling
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecurringPattern {
    pub id: String,
    pub pattern_number: String,              // RP-XXXXXXXXXX
    pub steward_id: AgentPubKey,
    pub label: String,
    pub description: Option<String>,
    pub frequency: String,                   // daily|weekly|biweekly|monthly|quarterly|semi-annual|annual|irregular|one-time
    pub frequency_value: Option<u32>,        // For "every N" patterns
    pub expected_amount: f64,
    pub expected_unit: String,
    pub variance_expected: u8,               // 0-100 percentage
    pub start_date: Timestamp,
    pub end_date: Option<Timestamp>,
    pub next_due_date: Timestamp,
    pub resource_category: String,
    pub pattern_type: String,                // income|expense|allocation|event
    pub auto_generate: bool,
    pub historical_occurrences_json: String, // Vec<String> as JSON (event IDs)
    pub missed_occurrences: u32,
    pub average_actual_amount: Option<f64>,
    pub reliability: u8,                     // 0-100 consistency score
    pub status: String,                      // active|paused|ended
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub schema_version: u32,
    pub validation_status: String,
}

// =============================================================================
// NOTE: Plaid/Banking Integration REMOVED from Holochain
// =============================================================================
// PlaidConnection, ImportBatch, StagedTransaction, TransactionRule have been
// moved to the banking-bridge module (elohim-app/src/app/shefa/banking-bridge).
// These are local-only (IndexedDB) because:
// 1. Bank credentials are personal convenience, not network signals
// 2. Staging data is ephemeral - only approved transactions become EconomicEvents
// 3. Separation prevents cluttering the next-gen economy domain
//
// The EconomicEventBridgeService in banking-bridge commits approved transactions
// to Holochain as EconomicEvents (the ONLY network signal from banking).
// =============================================================================

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
// Qahal: Governance Signals - Contextual Feedback & Consensus
// =============================================================================
//
// Governance signals enable constitutional feedback mechanisms:
// - Low friction: Emotional reactions (moved, grateful, challenged, concerned)
// - Medium friction: Graduated feedback (Loomio-style scales)
// - High friction: Formal proposals (binding decisions)
//
// Inspired by:
// - Loomio: 4-position voting (Agree/Abstain/Disagree/Block)
// - Forby: ARCH intensity-based voting
// - Polis: 2D opinion clustering and consensus discovery

/// GovernanceReaction - Low friction emotional feedback on content.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GovernanceReaction {
    pub id: String,
    pub content_id: String,
    pub content_type: String,
    pub reactor_id: String,
    pub reaction: String,
    pub intensity: u8,
    pub mediated: bool,
    pub mediation_accepted: bool,
    pub context_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const REACTION_TYPES: [&str; 6] = ["moved", "grateful", "challenged", "concerned", "surprised", "illuminated"];

/// GraduatedFeedback - Medium friction scaled feedback (Loomio/Forby style).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GraduatedFeedback {
    pub id: String,
    pub content_id: String,
    pub content_type: String,
    pub responder_id: String,
    pub feedback_context: String,
    pub position: i8,
    pub intensity: u8,
    pub reasoning: Option<String>,
    pub updated_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const FEEDBACK_CONTEXTS: [&str; 5] = ["accuracy", "usefulness", "proposal", "clarity", "relevance"];

/// ProposalVote - Loomio-style 4-position voting on proposals.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProposalVote {
    pub id: String,
    pub proposal_id: String,
    pub voter_id: String,
    pub voter_name: String,
    pub position: String,
    pub reasoning: Option<String>,
    pub version: u32,
    pub previous_position: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

pub const VOTE_POSITIONS: [&str; 4] = ["agree", "abstain", "disagree", "block"];

/// OpinionStatement - Polis-style statement for clustering.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct OpinionStatement {
    pub id: String,
    pub context_id: String,
    pub author_id: String,
    pub text: String,
    pub status: String,
    pub vote_count: u32,
    pub agree_count: u32,
    pub disagree_count: u32,
    pub pass_count: u32,
    pub consensus_score: i32,
    pub cluster_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// StatementVote - Individual vote on an OpinionStatement.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StatementVote {
    pub id: String,
    pub statement_id: String,
    pub voter_id: String,
    pub vote: String,
    pub created_at: String,
    pub metadata_json: String,
}

pub const STATEMENT_VOTES: [&str; 3] = ["agree", "disagree", "pass"];

/// LearningSignal - Learning progress and interaction signals.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct LearningSignal {
    pub id: String,
    pub content_id: String,
    pub learner_id: String,
    pub signal_type: String,
    pub payload_json: String,
    pub session_id: Option<String>,
    pub path_id: Option<String>,
    pub step_index: Option<u32>,
    pub created_at: String,
    pub metadata_json: String,
}

pub const LEARNING_SIGNAL_TYPES: [&str; 5] = ["content_viewed", "progress_update", "quiz_attempt", "mastery_achieved", "interactive_completion"];

/// MediationLog - Record of mediation interactions.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MediationLog {
    pub id: String,
    pub reaction_id: String,
    pub user_id: String,
    pub content_id: String,
    pub trigger_reaction: String,
    pub teaching_shown: String,
    pub user_choice: String,
    pub reflection: Option<String>,
    pub created_at: String,
    pub metadata_json: String,
}

pub const MEDIATION_CHOICES: [&str; 3] = ["proceed", "reconsider", "escalate"];

// =============================================================================
// Lamad: Content Attestation Entry - Trust Claims About Content
// =============================================================================
//
// ContentAttestation is DIFFERENT from Attestation (agent credentials):
// - Attestation = credentials granted to AGENTS (path completion, role credentials)
// - ContentAttestation = trust claims about CONTENT (author verified, peer reviewed)
//
// ContentAttestation enables the bidirectional trust model:
// - Agents need attestations to ACCESS content (via ContentReach)
// - Content needs attestations to REACH audiences (via reachGranted)
//
// See: elohim-app/src/app/lamad/models/content-attestation.model.ts

/// ContentAttestation - Trust credential granted to CONTENT.
///
/// Enables reach-based access control:
/// - Content starts with minimal reach (private)
/// - Attestations grant additional reach levels
/// - Reach levels: private < invited < local < community < commons
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContentAttestation {
    pub id: String,
    pub content_id: String,
    pub attestation_type: String,        // CONTENT_ATTESTATION_TYPES
    pub reach_granted: String,           // CONTENT_REACH_LEVELS
    pub granted_by_json: String,         // AttestationGrantor serialized
    pub granted_at: String,
    pub expires_at: Option<String>,
    pub status: String,                  // CONTENT_ATTESTATION_STATUS
    pub revocation_json: Option<String>, // AttestationRevocation if revoked
    pub evidence_json: Option<String>,   // AttestationEvidence
    pub scope_json: Option<String>,      // AttestationScope (optional)
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
    // Self-healing DNA fields
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
}

/// Content attestation types (different from agent attestation categories)
pub const CONTENT_ATTESTATION_TYPES: [&str; 10] = [
    "author-verified",       // Author identity confirmed
    "steward-approved",      // Domain steward reviewed and approved
    "community-endorsed",    // Received N endorsements from community
    "peer-reviewed",         // Formal review by qualified peers
    "governance-ratified",   // Approved through governance process
    "curriculum-canonical",  // Official learning content for a path
    "safety-reviewed",       // Checked for harmful content
    "accuracy-verified",     // Factual accuracy validated
    "accessibility-checked", // Meets accessibility standards
    "license-cleared",       // IP/licensing verified
];

/// Content attestation statuses
pub const CONTENT_ATTESTATION_STATUS: [&str; 4] = [
    "active",
    "expired",
    "revoked",
    "superseded",
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
// Doorway Infrastructure (Self-Validating Network Nodes)
// =============================================================================

/// Trust tier for doorways - computed from uptime history and attestations
/// Displayed on login screens so users know who they're signing in through
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DoorwayTier {
    Emerging,    // New doorway, < 7 days uptime
    Established, // 7+ days, 95%+ uptime
    Trusted,     // 30+ days, 99%+ uptime, peer attestations
    Anchor,      // 90+ days, 99.9%+ uptime, significant content served
}

impl Default for DoorwayTier {
    fn default() -> Self {
        DoorwayTier::Emerging
    }
}

/// DoorwayCapabilities - what services this doorway provides
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoorwayCapabilities {
    pub bootstrap: bool,    // Can bootstrap new nodes
    pub signal: bool,       // Provides WebRTC signaling
    pub gateway: bool,      // HTTP/REST gateway
    pub projection: bool,   // Maintains MongoDB projections
    pub custodian: bool,    // Can store blobs
}

impl Default for DoorwayCapabilities {
    fn default() -> Self {
        Self {
            bootstrap: false,
            signal: false,
            gateway: true,  // Most doorways are at least gateways
            projection: true,
            custodian: false,
        }
    }
}

/// DoorwayRegistration - A doorway node registered in the DHT
///
/// Doorways are the Web2 bridge to Holochain infrastructure. Unlike traditional
/// fediverse instances, doorways don't own user data - they project it from the DHT.
/// Users can switch doorways freely; their identity and data remain in the DHT.
///
/// Self-registration only: operator_agent must be the author (validation rule).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DoorwayRegistration {
    pub id: String,                       // "alpha-elohim-host"
    pub url: String,                      // "https://alpha.elohim.host"
    pub operator_agent: String,           // uhCAk... (who runs this doorway)
    pub operator_human: Option<String>,   // Link to Human entry for reputation
    pub capabilities_json: String,        // DoorwayCapabilities as JSON
    pub reach: String,                    // What reach levels served (from REACH_LEVELS)
    pub region: Option<String>,           // Geographic locality for routing
    pub bandwidth_mbps: Option<u32>,      // Self-reported bandwidth capacity
    pub version: String,                  // Doorway software version
    pub tier: String,                     // DoorwayTier as string
    pub registered_at: String,
    pub updated_at: String,
}

/// DoorwayHeartbeat - Lightweight status update (60s interval)
///
/// Detailed heartbeats are kept for 24 hours only, then summarized.
/// This enables real-time health monitoring without unbounded storage.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DoorwayHeartbeat {
    pub doorway_id: String,               // References DoorwayRegistration.id
    pub status: String,                   // online, degraded, maintenance, offline
    pub uptime_ratio: f32,                // Rolling uptime since last summary
    pub active_connections: u32,          // Current active connections
    pub content_served: u64,              // Bytes served since last heartbeat
    pub timestamp: String,
}

/// DoorwayHeartbeatSummary - Daily aggregate kept forever for reputation
///
/// At midnight UTC, doorways summarize their previous day's heartbeats
/// and prune the detailed records. Summaries build the long-term trust history.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DoorwayHeartbeatSummary {
    pub doorway_id: String,               // References DoorwayRegistration.id
    pub date: String,                     // "2024-01-15" (UTC date)
    pub uptime_ratio: f32,                // Average uptime for the day
    pub total_content_served: u64,        // Total bytes served
    pub peak_connections: u32,            // Maximum concurrent connections
    pub heartbeat_count: u32,             // How many heartbeats received (expect ~1440)
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
    BlobEntry(BlobEntry),              // NEW: Large media (video, audio, podcasts)
    LearningPath(LearningPath),
    PathChapter(PathChapter),
    PathStep(PathStep),
    ContentMastery(ContentMastery),
    PracticePool(PracticePool),
    MasteryChallenge(MasteryChallenge),
    KnowledgeMap(KnowledgeMap),
    PathExtension(PathExtension),
    ContentAttestation(ContentAttestation),

    // Qahal: Community & Relationships
    Relationship(Relationship),
    HumanRelationship(HumanRelationship),

    // Governance
    Challenge(Challenge),
    Proposal(Proposal),
    Precedent(Precedent),
    Discussion(Discussion),
    GovernanceState(GovernanceState),

    // Governance Signals (Loomio/Forby/Polis patterns)
    GovernanceReaction(GovernanceReaction),
    GraduatedFeedback(GraduatedFeedback),
    ProposalVote(ProposalVote),
    OpinionStatement(OpinionStatement),
    StatementVote(StatementVote),
    LearningSignal(LearningSignal),
    MediationLog(MediationLog),

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

    // Shefa: Insurance Mutual (Autonomous Mutual Insurance)
    MemberRiskProfile(MemberRiskProfile),
    CoveragePolicy(CoveragePolicy),
    CoveredRisk(CoveredRisk),
    InsuranceClaim(InsuranceClaim),
    AdjustmentReasoning(AdjustmentReasoning),

    // Shefa: Requests & Offers (Peer-to-Peer Service Coordination)
    ServiceRequest(ServiceRequest),
    ServiceOffer(ServiceOffer),
    ServiceMatch(ServiceMatch),

    // Shefa: Stewarded Resources (Resource Stewardship & Transparency)
    StewardedResource(StewardedResource),
    FinancialAsset(FinancialAsset),
    UBAEligibility(UBAEligibility),
    AccountingBridge(AccountingBridge),

    // Shefa: Constitutional Limits & Transitions (Donut Economy & Limitarianism)
    ConstitutionalLimit(ConstitutionalLimit),
    ResourcePosition(ResourcePosition),
    TransitionPath(TransitionPath),
    AssetSplit(AssetSplit),
    TransitionPhase(TransitionPhase),
    TransitionAction(TransitionAction),
    CommonsContribution(CommonsContribution),

    // Shefa: Flow Planning, Simulation, and Budgeting
    FlowPlan(FlowPlan),
    FlowBudget(FlowBudget),
    FlowGoal(FlowGoal),
    FlowMilestone(FlowMilestone),
    FlowScenario(FlowScenario),
    FlowProjection(FlowProjection),
    RecurringPattern(RecurringPattern),

    // NOTE: Plaid types (PlaidConnection, ImportBatch, StagedTransaction, TransactionRule)
    // removed - now in banking-bridge module (local IndexedDB only)

    // Lamad: Steward Economy
    StewardCredential(StewardCredential),
    PremiumGate(PremiumGate),
    AccessGrant(AccessGrant),
    StewardRevenue(StewardRevenue),

    // Infrastructure: Doorway Federation (Self-Validating Network Nodes)
    DoorwayRegistration(DoorwayRegistration),
    DoorwayHeartbeat(DoorwayHeartbeat),
    DoorwayHeartbeatSummary(DoorwayHeartbeatSummary),

    // Infrastructure: Anchors
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
    // Lamad: Blob (Media) links - Phase 1
    // =========================================================================
    ContentToBlobs,                     // Content -> BlobEntry (one-to-many)
    IdToBlob,                          // Anchor(blob_hash) -> BlobEntry (for lookup)
    BlobToVariants,                    // BlobEntry -> BlobVariant entries (quality options)
    BlobToCaptions,                    // BlobEntry -> BlobCaption entries (subtitles)
    BlobToReplicas,                    // BlobEntry -> CustodianCommitment (replication)
    AuthorToBlobs,                     // Anchor(author_id) -> BlobEntry (author's blobs)

    // =========================================================================
    // Lamad: Learning path links
    // =========================================================================
    IdToPath,
    IdToStep,                   // Anchor(step_id) -> PathStep
    PathToStep,
    StepToContent,
    // PathByCreator, PathByDifficulty, PathByType, PathByTag removed - use queries

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
    IdToMastery,                // Anchor(mastery_id) -> ContentMastery (for healing/migration)
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
    ContentRelationshipByType,  // Anchor(rel_type) -> Relationship

    // =========================================================================
    // Qahal: Governance Signal links (Loomio/Forby/Polis patterns)
    // =========================================================================
    ContentToReactions,         // Content -> GovernanceReaction
    AgentToReactions,           // Anchor(agent_id) -> GovernanceReaction
    ReactionByType,             // Anchor(reaction_type) -> GovernanceReaction
    ContentToFeedback,          // Content -> GraduatedFeedback
    AgentToFeedback,            // Anchor(agent_id) -> GraduatedFeedback
    FeedbackByContext,          // Anchor(feedback_context) -> GraduatedFeedback
    ProposalToVotes,            // Proposal -> ProposalVote
    AgentToVotes,               // Anchor(agent_id) -> ProposalVote
    VoteByPosition,             // Anchor(position) -> ProposalVote
    ContextToStatements,        // Anchor(context_id) -> OpinionStatement
    AgentToStatements,          // Anchor(agent_id) -> OpinionStatement
    StatementToVotes,           // OpinionStatement -> StatementVote
    AgentToStatementVotes,      // Anchor(agent_id) -> StatementVote
    ContentToLearningSignals,   // Content -> LearningSignal
    AgentToLearningSignals,     // Anchor(agent_id) -> LearningSignal
    PathToLearningSignals,      // Anchor(path_id) -> LearningSignal
    LearningSignalByType,       // Anchor(signal_type) -> LearningSignal
    ReactionToMediation,        // GovernanceReaction -> MediationLog
    AgentToMediations,          // Anchor(agent_id) -> MediationLog

    // =========================================================================
    // REMOVED: Identity links now in imagodei DNA
    // - HumanRelationship links: IdToHumanRelationship, AgentToRelationship, etc.
    // - Human/Agent links: IdToHuman, IdToAgent, AgentKeyToHuman, etc.
    // - Attestation links: AgentToAttestation, AttestationByCategory, etc.
    // See: holochain/dna/imagodei/zomes/imagodei_integrity
    // =========================================================================

    // =========================================================================
    // Lamad: Content Attestation links (Content trust claims)
    // =========================================================================
    IdToContentAttestation,         // Anchor(attestation_id) -> ContentAttestation
    ContentToContentAttestation,    // Anchor(content_id) -> ContentAttestation
    ContentAttestationByType,       // Anchor(attestation_type) -> ContentAttestation
    ContentAttestationByReach,      // Anchor(reach_granted) -> ContentAttestation

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
    // Shefa: Insurance Mutual links
    // =========================================================================
    IdToMemberRiskProfile,      // Anchor(profile_id) -> MemberRiskProfile
    MemberToRiskProfile,        // Anchor(member_id) -> MemberRiskProfile
    // RiskProfileByTier removed - query via projection
    IdToCoveragePolicy,         // Anchor(policy_id) -> CoveragePolicy
    MemberToCoveragePolicy,     // Anchor(member_id) -> CoveragePolicy
    // CoveragePolicyByLevel removed - query via projection
    PolicyToCoveredRisk,        // CoveragePolicy -> CoveredRisk
    IdToInsuranceClaim,         // Anchor(claim_id) -> InsuranceClaim
    MemberToClaim,              // Anchor(member_id) -> InsuranceClaim
    ClaimByStatus,              // Anchor(status) -> InsuranceClaim
    PolicyToClaim,              // CoveragePolicy -> InsuranceClaim
    IdToAdjustmentReasoning,    // Anchor(adjustment_id) -> AdjustmentReasoning
    ClaimToAdjustment,          // InsuranceClaim -> AdjustmentReasoning
    AdjustmentToEvent,          // AdjustmentReasoning -> EconomicEvent

    // =========================================================================
    // Shefa: Requests & Offers links
    // =========================================================================
    IdToServiceRequest,         // Anchor(request_id) -> ServiceRequest
    RequesterToRequest,         // Anchor(requester_id) -> ServiceRequest
    RequestByStatus,            // Anchor(status) -> ServiceRequest
    // RequestByServiceType removed - query via projection
    IdToServiceOffer,           // Anchor(offer_id) -> ServiceOffer
    OfferorToOffer,             // Anchor(offeror_id) -> ServiceOffer
    OfferByStatus,              // Anchor(status) -> ServiceOffer
    // OfferByServiceType removed - query via projection
    IdToServiceMatch,           // Anchor(match_id) -> ServiceMatch
    RequestToMatch,             // ServiceRequest -> ServiceMatch
    OfferToMatch,               // ServiceOffer -> ServiceMatch
    MatchByStatus,              // Anchor(status) -> ServiceMatch

    // =========================================================================
    // Shefa: Stewarded Resources links
    // =========================================================================
    // StewardedResource
    IdToStewardedResource,      // Anchor(resource_id) -> StewardedResource
    StewardToResource,          // Anchor(steward_id) -> StewardedResource
    ResourceByCategory,         // Anchor(category) -> StewardedResource
    ResourceByGovernance,       // Anchor(governance_level) -> StewardedResource

    // FinancialAsset
    IdToFinancialAsset,         // Anchor(asset_id) -> FinancialAsset
    StewardToFinancialAsset,    // Anchor(steward_id) -> FinancialAsset
    FinancialAssetByType,       // Anchor(asset_type) -> FinancialAsset
    FinancialAssetByStatus,     // Anchor(account_status) -> FinancialAsset

    // UBAEligibility
    IdToUBAEligibility,         // Anchor(eligibility_id) -> UBAEligibility
    HumanToUBA,                 // Anchor(human_id) -> UBAEligibility
    UBAByStatus,                // Anchor(eligible) -> UBAEligibility

    // AccountingBridge
    IdToAccountingBridge,       // Anchor(bridge_id) -> AccountingBridge
    AgentToAccountingBridge,    // Anchor(agent_id) -> AccountingBridge
    BridgeByAssetType,          // Anchor(asset_type) -> AccountingBridge
    BridgeByCategory,           // Anchor(asset_category) -> AccountingBridge

    // =========================================================================
    // Shefa: Constitutional Limits & Transitions links
    // =========================================================================
    // ConstitutionalLimit
    IdToConstitutionalLimit,    // Anchor(limit_id) -> ConstitutionalLimit
    LimitByCategory,            // Anchor(category) -> ConstitutionalLimit
    LimitByGovernance,          // Anchor(governance_level) -> ConstitutionalLimit
    LimitByEnforcement,         // Anchor(enforcement_method) -> ConstitutionalLimit

    // ResourcePosition
    IdToResourcePosition,       // Anchor(position_id) -> ResourcePosition
    ResourceToPosition,         // Anchor(resource_id) -> ResourcePosition
    PositionByType,             // Anchor(position_type) -> ResourcePosition
    PositionByCompliance,       // Anchor(compliant:warning_level) -> ResourcePosition

    // TransitionPath
    IdToTransitionPath,         // Anchor(path_id) -> TransitionPath
    StewardToTransitionPath,    // Anchor(steward_id) -> TransitionPath
    ResourceToTransitionPath,   // Anchor(resource_id) -> TransitionPath
    TransitionByStatus,         // Anchor(status) -> TransitionPath
    TransitionByGovernance,     // Anchor(governance_level) -> TransitionPath

    // AssetSplit
    IdToAssetSplit,             // Anchor(split_id) -> AssetSplit
    TransitionPathToSplit,      // Anchor(transition_path_id) -> AssetSplit
    SplitByDestination,         // Anchor(destination_type) -> AssetSplit
    SplitByStatus,              // Anchor(status) -> AssetSplit

    // TransitionPhase
    IdToTransitionPhase,        // Anchor(phase_id) -> TransitionPhase
    TransitionPathToPhase,      // Anchor(transition_path_id) -> TransitionPhase
    PhaseBySequence,            // Anchor(sequence_number) -> TransitionPhase
    PhaseByStatus,              // Anchor(status) -> TransitionPhase

    // TransitionAction
    IdToTransitionAction,       // Anchor(action_id) -> TransitionAction
    PhaseToAction,              // Anchor(phase_id) -> TransitionAction
    ActionByType,               // Anchor(action_type) -> TransitionAction
    ActionByStatus,             // Anchor(status) -> TransitionAction

    // CommonsContribution
    IdToCommonsContribution,    // Anchor(contribution_id) -> CommonsContribution
    StewardToContribution,      // Anchor(steward_id) -> CommonsContribution
    TransitionToContribution,   // Anchor(transition_path_id) -> CommonsContribution
    PoolToContribution,         // Anchor(commons_pool_id) -> CommonsContribution
    ContributionByRecognition,  // Anchor(public_recognition) -> CommonsContribution

    // =========================================================================
    // Shefa: Flow Planning links (Consolidated - essential navigation only)
    // =========================================================================
    IdToFlowPlan,               // Anchor(plan_id) -> FlowPlan
    StewardToFlowPlan,          // Anchor(steward_id) -> FlowPlan
    PlanToBudget,               // Plan -> FlowBudget (structural)
    PlanToGoal,                 // Plan -> FlowGoal (structural)
    PlanToMilestone,            // Plan -> FlowMilestone (structural)
    PlanToScenario,             // Plan -> FlowScenario (structural)
    IdToFlowBudget,             // Anchor(budget_id) -> FlowBudget
    IdToFlowGoal,               // Anchor(goal_id) -> FlowGoal
    IdToFlowMilestone,          // Anchor(milestone_id) -> FlowMilestone
    IdToFlowScenario,           // Anchor(scenario_id) -> FlowScenario
    ScenarioToProjection,       // Scenario -> FlowProjection (structural)
    IdToRecurringPattern,       // Anchor(pattern_id) -> RecurringPattern
    StewardToRecurringPattern,  // Anchor(steward_id) -> RecurringPattern

    // NOTE: Plaid link types removed - now in banking-bridge (local IndexedDB)

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

    // =========================================================================
    // REMOVED: Doorway links now in infrastructure DNA
    // - IdToDoorway, OperatorToDoorway, DoorwayToHeartbeat, DoorwayToSummary
    // See: holochain/dna/infrastructure/zomes/infrastructure_integrity
    // =========================================================================
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
