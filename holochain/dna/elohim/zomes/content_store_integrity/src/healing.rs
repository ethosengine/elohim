//! Self-Healing Entry Implementations for Lamad
//!
//! Implements the SelfHealingEntry trait for all core Lamad entry types.
//! These implementations are defined here in the integrity crate to avoid
//! Rust's orphan rule violations (both trait and types must have local ownership).

use hc_rna::{SelfHealingEntry, ValidationStatus};
use crate::*;

// ============================================================================
// Validation Constants - Aligned with lib.rs and data/lamad content
// ============================================================================

/// Content types - extended to support all imported content
pub const CONTENT_TYPES: &[&str] = &[
    // Core types (from lib.rs)
    "epic",           // High-level narrative/vision document
    "concept",        // Atomic knowledge unit
    "lesson",         // Digestible learning session (AI-derived from concepts)
    "scenario",       // Gherkin feature/scenario
    "assessment",     // Quiz or test
    "resource",       // Supporting material
    "reflection",     // Journaling/reflection prompt
    "discussion",     // Discussion topic
    "exercise",       // Practice activity
    "example",        // Illustrative example
    "reference",      // Reference material
    "article",        // Long-form article content
    "feature",        // Gherkin feature (imported from .feature files)
    "practice",       // Practice activity (legacy alias)
    "human",          // Human persona files
    // Extended types (from FCT and other imports)
    "organization",   // Organization reference
    "contributor",    // Contributor profile
    "video",          // Video content reference
    "audio",          // Audio content reference
    "book",           // Book reference
    "book-chapter",   // Book chapter reference
    "documentary",    // Documentary reference
    "bible-verse",    // Biblical scripture reference
    "activity",       // Learning activity
    "narrative",      // Narrative/story content
    "course-module",  // Course module structure
    "module",         // Generic module
    "quiz",           // Quiz content (distinct from assessment)
];

/// Reach levels - must match REACH_LEVELS in lib.rs
pub const REACH_LEVELS: &[&str] = &[
    "private",    // Only self
    "self",       // Only self (alias)
    "intimate",   // Closest relationships
    "trusted",    // Trusted circle
    "familiar",   // Extended network
    "community",  // Community members
    "public",     // Anyone authenticated
    "commons",    // Anyone, including anonymous
];

/// Content formats - all formats used in data/lamad content
pub const CONTENT_FORMATS: &[&str] = &[
    "markdown",        // Markdown format
    "html",            // HTML format
    "plaintext",       // Plain text
    "text",            // Plain text (alias)
    "plain",           // Plain text (alias)
    "video",           // Video media reference
    "audio",           // Audio media reference
    "interactive",     // Interactive content
    "external",        // External URL reference
    "gherkin",         // Gherkin/Cucumber scenario format (imported from .feature files)
    "perseus",           // Perseus quiz/assessment format (Khan Academy derived, canonical)
    "perseus-json",      // Perseus format (alias)
    "perseus-quiz-json", // Perseus quiz format (self-documenting assessment content)
    "video-embed",     // Embedded video (YouTube, Vimeo, etc.)
    "audio-file",      // Audio file reference
    "html5-app",       // HTML5 interactive application
    "human-json",      // Human persona JSON format
    "organization-json", // Organization JSON format
    "json",            // Generic JSON format
];

pub const PATH_VISIBILITIES: &[&str] = &[
    "private",   // Only creator
    "unlisted",  // Accessible by link
    "community", // Community members
    "public",    // Anyone
    "draft",     // Draft in progress
];

pub const STEP_TYPES: &[&str] = &[
    "content",   // Reference content
    "read",      // Read content (seeder default)
    "path",      // Reference another path
    "external",  // External URL
    "practice",  // Practice activity
    "assess",    // Assessment step
    "video",     // Video content step
    "interactive", // Interactive activity
];

/// Mastery levels - must match MASTERY_LEVELS in lib.rs (Bloom's Taxonomy)
pub const MASTERY_LEVELS: &[&str] = &[
    "not_started", // 0 - No engagement
    "seen",        // 1 - Content viewed
    "remember",    // 2 - Basic recall demonstrated
    "understand",  // 3 - Comprehension demonstrated
    "apply",       // 4 - Application in novel contexts (ATTESTATION GATE)
    "analyze",     // 5 - Can break down, connect, contribute analysis
    "evaluate",    // 6 - Can assess, critique, peer review
    "create",      // 7 - Can author, derive, synthesize
    // Legacy aliases
    "recognize",   // Alias for remember
    "recall",      // Alias for remember
    "synthesize",  // Alias for create
];

pub const COMPLETION_CRITERIA: &[&str] = &[
    "all-required",     // All steps required
    "pass-assessment",  // Must pass assessment
    "view-content",     // Just view content
];

// ============================================================================
// Content Self-Healing Implementation
// ============================================================================

impl SelfHealingEntry for Content {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn validation_status(&self) -> ValidationStatus {
        match self.validation_status.as_str() {
            "Valid" => ValidationStatus::Valid,
            "Migrated" => ValidationStatus::Migrated,
            "Degraded" => ValidationStatus::Degraded,
            "Healing" => ValidationStatus::Healing,
            _ => ValidationStatus::Valid,
        }
    }

    fn set_validation_status(&mut self, status: ValidationStatus) {
        self.validation_status = format!("{}", status);
    }

    fn entry_id(&self) -> String {
        self.id.clone()
    }

    fn validate(&self) -> Result<(), String> {
        // Required fields
        if self.id.is_empty() {
            return Err("Content id is required".to_string());
        }
        if self.title.is_empty() {
            return Err("Content title is required".to_string());
        }
        if self.content_type.is_empty() {
            return Err("Content type is required".to_string());
        }

        // Validate against known content types
        if !CONTENT_TYPES.contains(&self.content_type.as_str()) {
            return Err(format!(
                "Invalid content_type '{}'. Must be one of: {:?}",
                self.content_type, CONTENT_TYPES
            ));
        }

        // Validate reach level
        if !REACH_LEVELS.contains(&self.reach.as_str()) {
            return Err(format!(
                "Invalid reach '{}'. Must be one of: {:?}",
                self.reach, REACH_LEVELS
            ));
        }

        // Validate format
        if !CONTENT_FORMATS.contains(&self.content_format.as_str()) {
            return Err(format!(
                "Invalid content_format '{}'. Must be one of: {:?}",
                self.content_format, CONTENT_FORMATS
            ));
        }

        // Reference validation is deferred - will be checked when references are accessed
        // For now, just validate that IDs are not empty
        // This allows entries to be created and marked Degraded if references fail later
        for related_id in &self.related_node_ids {
            if related_id.is_empty() {
                return Err("Related content ID cannot be empty".to_string());
            }
        }

        Ok(())
    }
}

// ============================================================================
// LearningPath Self-Healing Implementation
// ============================================================================

impl SelfHealingEntry for LearningPath {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn validation_status(&self) -> ValidationStatus {
        match self.validation_status.as_str() {
            "Valid" => ValidationStatus::Valid,
            "Migrated" => ValidationStatus::Migrated,
            "Degraded" => ValidationStatus::Degraded,
            "Healing" => ValidationStatus::Healing,
            _ => ValidationStatus::Valid,
        }
    }

    fn set_validation_status(&mut self, status: ValidationStatus) {
        self.validation_status = format!("{}", status);
    }

    fn entry_id(&self) -> String {
        self.id.clone()
    }

    fn validate(&self) -> Result<(), String> {
        // Required fields
        if self.id.is_empty() {
            return Err("LearningPath id is required".to_string());
        }
        if self.title.is_empty() {
            return Err("LearningPath title is required".to_string());
        }
        if self.created_by.is_empty() {
            return Err("LearningPath created_by is required".to_string());
        }

        // Validate visibility
        if !PATH_VISIBILITIES.contains(&self.visibility.as_str()) {
            return Err(format!(
                "Invalid visibility '{}'. Must be one of: {:?}",
                self.visibility, PATH_VISIBILITIES
            ));
        }

        // Validate creator exists (if we can check)
        // This would require querying agents/humans
        // For now, just validate it's not empty (already checked above)

        Ok(())
    }
}

// ============================================================================
// PathStep Self-Healing Implementation
// ============================================================================

impl SelfHealingEntry for PathStep {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn validation_status(&self) -> ValidationStatus {
        match self.validation_status.as_str() {
            "Valid" => ValidationStatus::Valid,
            "Migrated" => ValidationStatus::Migrated,
            "Degraded" => ValidationStatus::Degraded,
            "Healing" => ValidationStatus::Healing,
            _ => ValidationStatus::Valid,
        }
    }

    fn set_validation_status(&mut self, status: ValidationStatus) {
        self.validation_status = format!("{}", status);
    }

    fn entry_id(&self) -> String {
        self.id.clone()
    }

    fn validate(&self) -> Result<(), String> {
        // Required fields
        if self.id.is_empty() {
            return Err("PathStep id is required".to_string());
        }
        if self.path_id.is_empty() {
            return Err("PathStep path_id is required".to_string());
        }
        if self.resource_id.is_empty() {
            return Err("PathStep resource_id is required".to_string());
        }

        // Validate step type
        if !STEP_TYPES.contains(&self.step_type.as_str()) {
            return Err(format!(
                "Invalid step_type '{}'. Must be one of: {:?}",
                self.step_type, STEP_TYPES
            ));
        }

        // Reference validation is deferred - will be checked when references are accessed
        // Just validate that references are not empty
        if self.path_id.is_empty() {
            return Err("Path ID cannot be empty".to_string());
        }

        if self.resource_id.is_empty() {
            return Err("Resource ID cannot be empty".to_string());
        }

        // Validate completion criteria if present
        if let Some(criteria) = &self.completion_criteria {
            if !COMPLETION_CRITERIA.contains(&criteria.as_str()) {
                return Err(format!(
                    "Invalid completion_criteria '{}'. Must be one of: {:?}",
                    criteria, COMPLETION_CRITERIA
                ));
            }
        }

        Ok(())
    }
}

// ============================================================================
// ContentMastery Self-Healing Implementation
// ============================================================================

impl SelfHealingEntry for ContentMastery {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn validation_status(&self) -> ValidationStatus {
        match self.validation_status.as_str() {
            "Valid" => ValidationStatus::Valid,
            "Migrated" => ValidationStatus::Migrated,
            "Degraded" => ValidationStatus::Degraded,
            "Healing" => ValidationStatus::Healing,
            _ => ValidationStatus::Valid,
        }
    }

    fn set_validation_status(&mut self, status: ValidationStatus) {
        self.validation_status = format!("{}", status);
    }

    fn entry_id(&self) -> String {
        self.id.clone()
    }

    fn validate(&self) -> Result<(), String> {
        // Required fields
        if self.id.is_empty() {
            return Err("ContentMastery id is required".to_string());
        }
        if self.human_id.is_empty() {
            return Err("ContentMastery human_id is required".to_string());
        }
        if self.content_id.is_empty() {
            return Err("ContentMastery content_id is required".to_string());
        }

        // Validate mastery level
        if !MASTERY_LEVELS.contains(&self.mastery_level.as_str()) {
            return Err(format!(
                "Invalid mastery_level '{}'. Must be one of: {:?}",
                self.mastery_level, MASTERY_LEVELS
            ));
        }

        // Validate mastery_level_index matches mastery_level
        let expected_index = MASTERY_LEVELS
            .iter()
            .position(|&l| l == self.mastery_level.as_str())
            .unwrap_or(0) as u32;
        if self.mastery_level_index != expected_index {
            return Err(format!(
                "mastery_level_index {} doesn't match mastery_level '{}' (expected {})",
                self.mastery_level_index, self.mastery_level, expected_index
            ));
        }

        // Validate freshness_score is in range
        if self.freshness_score < 0.0 || self.freshness_score > 1.0 {
            return Err(format!(
                "freshness_score {} out of range (0.0-1.0)",
                self.freshness_score
            ));
        }

        // Validate last_engagement_type
        if !ENGAGEMENT_TYPES.contains(&self.last_engagement_type.as_str()) {
            return Err(format!(
                "Invalid last_engagement_type '{}'. Must be one of: {:?}",
                self.last_engagement_type, ENGAGEMENT_TYPES
            ));
        }

        Ok(())
    }
}
