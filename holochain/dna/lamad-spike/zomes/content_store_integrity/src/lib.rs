//! Content Store Integrity Zome
//!
//! Defines the entry types and validation rules for the Lamad content store.
//! Supports full ContentNode schema from elohim-service plus learning paths.

use hdi::prelude::*;

// =============================================================================
// Content Entry - Full ContentNode model
// =============================================================================

/// Content entry - maps to elohim-service ContentNode
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Content {
    // === Core identity ===
    /// Unique identifier (kebab-case, matches ContentNode.id)
    pub id: String,
    /// Content type: epic, scenario, role, feature, concept, video, etc.
    pub content_type: String,
    /// Display title
    pub title: String,
    /// Brief description/summary
    pub description: String,
    /// Full content body (markdown, gherkin, html, etc.)
    pub content: String,
    /// Content format for rendering: markdown, gherkin, html, video-embed, etc.
    pub content_format: String,

    // === Organization ===
    /// Tags for categorization and search
    pub tags: Vec<String>,
    /// Source file path (for provenance/debugging)
    pub source_path: Option<String>,
    /// Related node IDs (bidirectional relationships)
    pub related_node_ids: Vec<String>,

    // === Trust & Reach ===
    /// Author/creator ID (AgentPubKey as string, or external ID)
    pub author_id: Option<String>,
    /// Reach level: private, invited, local, community, federated, commons
    pub reach: String,
    /// Trust score (0.0 - 1.0)
    pub trust_score: f64,

    // === Flexible metadata ===
    /// Serialized JSON for domain-specific metadata (ContentMetadata)
    pub metadata_json: String,

    // === Timestamps ===
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last updated timestamp (ISO 8601)
    pub updated_at: String,
}

// =============================================================================
// ContentRelationship Entry - Graph edges between content
// =============================================================================

/// Relationship between content nodes (graph edge)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContentRelationship {
    /// Unique identifier for this relationship
    pub id: String,
    /// Source node ID
    pub source_node_id: String,
    /// Target node ID
    pub target_node_id: String,
    /// Relationship type: CONTAINS, RELATES_TO, IMPLEMENTS, VALIDATES, etc.
    pub relationship_type: String,
    /// Confidence score for inferred relationships (0.0 - 1.0)
    pub confidence: f64,
    /// Optional metadata as JSON string
    pub metadata_json: Option<String>,
}

// =============================================================================
// LearningPath Entry - Curated learning sequences
// =============================================================================

/// Learning path - curated sequence of content
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct LearningPath {
    /// Unique identifier (kebab-case)
    pub id: String,
    /// Version string (semver)
    pub version: String,
    /// Display title
    pub title: String,
    /// Path description
    pub description: String,
    /// Purpose/learning objectives
    pub purpose: Option<String>,
    /// Creator ID
    pub created_by: String,
    /// Difficulty level: beginner, intermediate, advanced
    pub difficulty: String,
    /// Estimated duration (e.g., "2 hours", "1 week")
    pub estimated_duration: Option<String>,
    /// Visibility: draft, published, archived
    pub visibility: String,
    /// Path type: linear, branching, adaptive
    pub path_type: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last updated timestamp (ISO 8601)
    pub updated_at: String,
}

// =============================================================================
// PathStep Entry - Steps within a learning path
// =============================================================================

/// Step in a learning path
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PathStep {
    /// Unique identifier for this step
    pub id: String,
    /// Parent path ID
    pub path_id: String,
    /// Order index within path (0-based)
    pub order_index: u32,
    /// Step type: content, assessment, checkpoint, etc.
    pub step_type: String,
    /// Resource ID (links to Content.id)
    pub resource_id: String,
    /// Optional step title (overrides content title)
    pub step_title: Option<String>,
    /// Optional narrative/context for this step
    pub step_narrative: Option<String>,
    /// Whether this step is optional
    pub is_optional: bool,
}

// =============================================================================
// StringAnchor Entry - For creating deterministic link bases from strings
// =============================================================================

/// String anchor entry for creating deterministic link bases from strings.
/// Used to create links like IdToContent, TypeToContent, TagToContent, etc.
/// Named StringAnchor to avoid conflict with hdk::prelude::Anchor
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StringAnchor {
    /// Anchor type (e.g., "content_id", "content_type", "tag")
    pub anchor_type: String,
    /// Anchor value (e.g., the actual ID, type, or tag string)
    pub anchor_text: String,
}

impl StringAnchor {
    pub fn new(anchor_type: &str, anchor_text: &str) -> Self {
        Self {
            anchor_type: anchor_type.to_string(),
            anchor_text: anchor_text.to_string(),
        }
    }
}

// =============================================================================
// Entry Types Enum
// =============================================================================

/// All entry types in this DNA
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Content(Content),
    ContentRelationship(ContentRelationship),
    LearningPath(LearningPath),
    PathStep(PathStep),
    StringAnchor(StringAnchor),
}

// =============================================================================
// Link Types
// =============================================================================

/// Link types for indexing and queries
#[hdk_link_types]
pub enum LinkTypes {
    // Content indexing
    /// AgentPubKey → Content (author's content)
    AuthorToContent,
    /// Hash(id) → Content (lookup by string ID)
    IdToContent,
    /// Hash(content_type) → Content (filter by type)
    TypeToContent,
    /// Hash(tag) → Content (filter by tag)
    TagToContent,
    /// Hash(import_id) → Content (bulk import tracking)
    ImportBatchToContent,

    // Learning path structure
    /// LearningPath → PathStep
    PathToStep,
    /// PathStep → Content
    StepToContent,
    /// Hash(path_id) → LearningPath (lookup by string ID)
    IdToPath,
}

// =============================================================================
// Validation
// =============================================================================

/// Validation callback - runs when entries are created/updated
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Content(content) => validate_content(content),
                EntryTypes::LearningPath(path) => validate_learning_path(path),
                EntryTypes::PathStep(step) => validate_path_step(step),
                EntryTypes::ContentRelationship(rel) => validate_relationship(rel),
                EntryTypes::StringAnchor(_) => Ok(ValidateCallbackResult::Valid), // Anchors are always valid
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

/// Validate Content entry
fn validate_content(content: Content) -> ExternResult<ValidateCallbackResult> {
    // ID must not be empty
    if content.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Content ID cannot be empty".to_string(),
        ));
    }

    // Content type validation
    let valid_types = [
        "source",
        "epic",
        "feature",
        "scenario",
        "concept",
        "role",
        "video",
        "organization",
        "book-chapter",
        "tool",
        "path",
        "assessment",
        "reference",
        "example",
    ];
    if !valid_types.contains(&content.content_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid content type: {}. Must be one of: {}",
            content.content_type,
            valid_types.join(", ")
        )));
    }

    // Content format validation
    let valid_formats = [
        "markdown",
        "gherkin",
        "html",
        "plaintext",
        "video-embed",
        "external-link",
        "quiz-json",
        "assessment-json",
    ];
    if !valid_formats.contains(&content.content_format.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid content format: {}. Must be one of: {}",
            content.content_format,
            valid_formats.join(", ")
        )));
    }

    // Reach validation
    let valid_reach = [
        "private",
        "invited",
        "local",
        "community",
        "federated",
        "commons",
    ];
    if !valid_reach.contains(&content.reach.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid reach: {}. Must be one of: {}",
            content.reach,
            valid_reach.join(", ")
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate LearningPath entry
fn validate_learning_path(path: LearningPath) -> ExternResult<ValidateCallbackResult> {
    if path.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Path ID cannot be empty".to_string(),
        ));
    }

    let valid_difficulties = ["beginner", "intermediate", "advanced"];
    if !valid_difficulties.contains(&path.difficulty.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid difficulty: {}. Must be one of: {}",
            path.difficulty,
            valid_difficulties.join(", ")
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate PathStep entry
fn validate_path_step(step: PathStep) -> ExternResult<ValidateCallbackResult> {
    if step.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Step ID cannot be empty".to_string(),
        ));
    }

    if step.path_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Step must have a path_id".to_string(),
        ));
    }

    if step.resource_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Step must have a resource_id".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate ContentRelationship entry
fn validate_relationship(rel: ContentRelationship) -> ExternResult<ValidateCallbackResult> {
    if rel.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Relationship ID cannot be empty".to_string(),
        ));
    }

    if rel.source_node_id.is_empty() || rel.target_node_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Relationship must have source and target node IDs".to_string(),
        ));
    }

    let valid_types = [
        "CONTAINS",
        "BELONGS_TO",
        "DESCRIBES",
        "IMPLEMENTS",
        "VALIDATES",
        "RELATES_TO",
        "REFERENCES",
        "DEPENDS_ON",
        "REQUIRES",
        "FOLLOWS",
        "DERIVED_FROM",
        "SOURCE_OF",
    ];
    if !valid_types.contains(&rel.relationship_type.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid relationship type: {}. Must be one of: {}",
            rel.relationship_type,
            valid_types.join(", ")
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}
