//! Self-Healing Entry Implementations for Lamad
//!
//! Implements the SelfHealingEntry trait for all core Lamad entry types.
//! These implementations are defined here in the integrity crate to avoid
//! Rust's orphan rule violations (both trait and types must have local ownership).

use hc_rna::{SelfHealingEntry, ValidationStatus};
use crate::*;

// ============================================================================
// Validation Constants
// ============================================================================

pub const CONTENT_TYPES: &[&str] = &[
    "concept",       // Atomic knowledge concept
    "lesson",        // Structured learning unit
    "practice",      // Hands-on practice activity
    "assessment",    // Knowledge check
    "reference",     // External reference material
];

pub const REACH_LEVELS: &[&str] = &[
    "public",   // Open to everyone
    "commons",  // Shared commons (curated)
    "private",  // Private/restricted
];

pub const CONTENT_FORMATS: &[&str] = &[
    "markdown",   // Markdown format
    "html",       // HTML format
    "plaintext",  // Plain text
    "video",      // Video media
];

pub const PATH_VISIBILITIES: &[&str] = &[
    "public",  // Published path
    "private", // Private path
    "draft",   // Draft in progress
];

pub const STEP_TYPES: &[&str] = &[
    "content",   // Reference content
    "path",      // Reference another path
    "external",  // External URL
    "practice",  // Practice activity
];

pub const MASTERY_LEVELS: &[&str] = &[
    "recognize",    // Can identify
    "recall",       // Can recall
    "understand",   // Understands concepts
    "apply",        // Can apply knowledge
    "synthesize",   // Can combine and create
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
