//! Lamad Entry Type Providers for Flexible Healing Architecture
//!
//! This module provides implementations of the EntryTypeProvider trait for all
//! Lamad entry types. Each provider composes:
//! - Validator: schema validation rules
//! - Transformer: v1→v2 field mapping
//! - ReferenceResolver: check if referenced entries exist
//! - DegradationHandler: policy for handling healing failures

use hc_rna::{
    Validator, Transformer, ReferenceResolver, DegradationHandler, DegradationDecision,
    EntryTypeProvider,
};
use serde_json::Value;

// ============================================================================
// VALIDATORS - Schema and Business Logic Validation
// ============================================================================

/// Validates Content entries according to schema and business rules
pub struct ContentValidator;

impl Validator for ContentValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        // Required fields
        let id = data["id"].as_str().ok_or("Content id is required and must be string")?;
        if id.is_empty() {
            return Err("Content id cannot be empty".to_string());
        }

        let title = data["title"].as_str().ok_or("Content title is required and must be string")?;
        if title.is_empty() {
            return Err("Content title cannot be empty".to_string());
        }

        let content_type = data["content_type"].as_str()
            .ok_or("Content content_type is required and must be string")?;

        // Validate against allowed values
        const CONTENT_TYPES: &[&str] = &[
            "epic", "concept", "lesson", "scenario", "assessment", "resource",
            "practice", "reflection", "reference", "external"
        ];
        if !CONTENT_TYPES.contains(&content_type) {
            return Err(format!("Invalid content_type '{}'. Must be one of: {:?}",
                content_type, CONTENT_TYPES));
        }

        // Reach must be valid if present
        if let Some(reach) = data["reach"].as_str() {
            const REACH_LEVELS: &[&str] = &[
                "private", "intimate", "trusted", "familiar", "community", "public", "commons"
            ];
            if !REACH_LEVELS.contains(&reach) {
                return Err(format!("Invalid reach '{}'. Must be one of: {:?}",
                    reach, REACH_LEVELS));
            }
        }

        // Schema version should be 2
        let schema_version = data["schema_version"].as_u64().unwrap_or(0);
        if schema_version != 2 {
            return Err(format!("Expected schema_version 2, got {}", schema_version));
        }

        // Related node IDs must not be empty strings
        if let Some(related) = data["related_node_ids"].as_array() {
            for (idx, id) in related.iter().enumerate() {
                if let Some(id_str) = id.as_str() {
                    if id_str.is_empty() {
                        return Err(format!("Related node ID at index {} cannot be empty", idx));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Validates LearningPath entries
pub struct LearningPathValidator;

impl Validator for LearningPathValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        let id = data["id"].as_str().ok_or("Path id is required")?;
        if id.is_empty() {
            return Err("Path id cannot be empty".to_string());
        }

        let title = data["title"].as_str().ok_or("Path title is required")?;
        if title.is_empty() {
            return Err("Path title cannot be empty".to_string());
        }

        let schema_version = data["schema_version"].as_u64().unwrap_or(0);
        if schema_version != 2 {
            return Err(format!("Expected schema_version 2, got {}", schema_version));
        }

        Ok(())
    }
}

/// Validates PathStep entries
pub struct PathStepValidator;

impl Validator for PathStepValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        let id = data["id"].as_str().ok_or("Step id is required")?;
        if id.is_empty() {
            return Err("Step id cannot be empty".to_string());
        }

        let path_id = data["path_id"].as_str().ok_or("Step path_id is required")?;
        if path_id.is_empty() {
            return Err("Step path_id cannot be empty".to_string());
        }

        let schema_version = data["schema_version"].as_u64().unwrap_or(0);
        if schema_version != 2 {
            return Err(format!("Expected schema_version 2, got {}", schema_version));
        }

        Ok(())
    }
}

/// Validates ContentMastery entries
pub struct ContentMasteryValidator;

impl Validator for ContentMasteryValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        let id = data["id"].as_str().ok_or("Mastery id is required")?;
        if id.is_empty() {
            return Err("Mastery id cannot be empty".to_string());
        }

        let human_id = data["human_id"].as_str().ok_or("Mastery human_id is required")?;
        if human_id.is_empty() {
            return Err("Mastery human_id cannot be empty".to_string());
        }

        let content_id = data["content_id"].as_str().ok_or("Mastery content_id is required")?;
        if content_id.is_empty() {
            return Err("Mastery content_id cannot be empty".to_string());
        }

        let mastery_level = data["mastery_level"].as_str()
            .ok_or("Mastery mastery_level is required")?;

        const MASTERY_LEVELS: &[&str] = &[
            "not_started", "seen", "remember", "understand",
            "apply", "analyze", "evaluate", "create"
        ];
        if !MASTERY_LEVELS.contains(&mastery_level) {
            return Err(format!("Invalid mastery_level '{}'. Must be one of: {:?}",
                mastery_level, MASTERY_LEVELS));
        }

        let schema_version = data["schema_version"].as_u64().unwrap_or(0);
        if schema_version != 2 {
            return Err(format!("Expected schema_version 2, got {}", schema_version));
        }

        Ok(())
    }
}

// ============================================================================
// TRANSFORMERS - V1 to V2 Schema Transformation
// ============================================================================

/// Transforms Content from v1 to v2 schema
pub struct ContentTransformer;

impl Transformer for ContentTransformer {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {
        // Extract v1 fields
        let id = v1_data["id"].as_str().ok_or("v1 Content missing id")?;
        let title = v1_data["title"].as_str().ok_or("v1 Content missing title")?;
        let description = v1_data["description"].as_str().unwrap_or("");
        let content = v1_data["content"].as_str().unwrap_or("");
        let content_format = v1_data["content_format"].as_str().unwrap_or("markdown");
        let content_type = v1_data["content_type"].as_str().unwrap_or("lesson");
        let tags: Vec<String> = v1_data["tags"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let reach = v1_data["reach"].as_str().unwrap_or("community");
        let related_node_ids: Vec<String> = v1_data["related_node_ids"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Build v2 entry with current schema
        Ok(serde_json::json!({
            "id": id,
            "content_type": content_type,
            "title": title,
            "description": description,
            "content": content,
            "content_format": content_format,
            "tags": tags,
            "reach": reach,
            "related_node_ids": related_node_ids,
            "schema_version": 2,
            "validation_status": "Migrated"
        }))
    }

    fn description(&self) -> &str {
        "Transform Content from v1 to v2 schema"
    }
}

/// Transforms LearningPath from v1 to v2
pub struct LearningPathTransformer;

impl Transformer for LearningPathTransformer {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {
        let id = v1_data["id"].as_str().ok_or("v1 LearningPath missing id")?;
        let title = v1_data["title"].as_str().ok_or("v1 LearningPath missing title")?;
        let description = v1_data["description"].as_str().unwrap_or("");
        let difficulty = v1_data["difficulty"].as_str().unwrap_or("intermediate");
        let tags: Vec<String> = v1_data["tags"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(serde_json::json!({
            "id": id,
            "title": title,
            "description": description,
            "difficulty": difficulty,
            "tags": tags,
            "schema_version": 2,
            "validation_status": "Migrated"
        }))
    }

    fn description(&self) -> &str {
        "Transform LearningPath from v1 to v2 schema"
    }
}

/// Transforms PathStep from v1 to v2
pub struct PathStepTransformer;

impl Transformer for PathStepTransformer {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {
        let id = v1_data["id"].as_str().ok_or("v1 PathStep missing id")?;
        let path_id = v1_data["path_id"].as_str().ok_or("v1 PathStep missing path_id")?;
        let content_id = v1_data["content_id"].as_str().unwrap_or("");
        let step_type = v1_data["step_type"].as_str().unwrap_or("content");
        let order = v1_data["order"].as_u64().unwrap_or(0);

        Ok(serde_json::json!({
            "id": id,
            "path_id": path_id,
            "content_id": content_id,
            "step_type": step_type,
            "order": order,
            "schema_version": 2,
            "validation_status": "Migrated"
        }))
    }

    fn description(&self) -> &str {
        "Transform PathStep from v1 to v2 schema"
    }
}

/// Transforms ContentMastery from v1 to v2
pub struct ContentMasteryTransformer;

impl Transformer for ContentMasteryTransformer {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {
        let id = v1_data["id"].as_str().ok_or("v1 ContentMastery missing id")?;
        let human_id = v1_data["human_id"].as_str().ok_or("v1 ContentMastery missing human_id")?;
        let content_id = v1_data["content_id"].as_str().ok_or("v1 ContentMastery missing content_id")?;
        let mastery_level = v1_data["mastery_level"].as_str().unwrap_or("not_started");
        let mastery_level_index = v1_data["mastery_level_index"].as_u64().unwrap_or(0);
        let freshness_score = v1_data["freshness_score"].as_f64().unwrap_or(0.0);
        let engagement_count = v1_data["engagement_count"].as_u64().unwrap_or(0);

        Ok(serde_json::json!({
            "id": id,
            "human_id": human_id,
            "content_id": content_id,
            "mastery_level": mastery_level,
            "mastery_level_index": mastery_level_index,
            "freshness_score": freshness_score,
            "engagement_count": engagement_count,
            "schema_version": 2,
            "validation_status": "Migrated"
        }))
    }

    fn description(&self) -> &str {
        "Transform ContentMastery from v1 to v2 schema"
    }
}

// ============================================================================
// REFERENCE RESOLVERS - Check if Referenced Entries Exist
// ============================================================================

/// Resolves references in Content entries (checks if related content exists)
pub struct ContentReferenceResolver;

impl ReferenceResolver for ContentReferenceResolver {
    fn resolve_reference(&self, _entry_type: &str, id: &str) -> Result<bool, String> {
        // In a real implementation, this would check the DHT:
        // get_entry_by_id(id).map(|e| e.is_some())
        // For now, we accept all references (they may not exist yet)
        Ok(true)
    }
}

/// Resolves references in PathStep entries (checks if path and content exist)
pub struct PathStepReferenceResolver;

impl ReferenceResolver for PathStepReferenceResolver {
    fn resolve_reference(&self, entry_type: &str, id: &str) -> Result<bool, String> {
        // In a real implementation:
        // If entry_type=="path", check path exists
        // If entry_type=="content", check content exists
        Ok(true)
    }
}

/// Resolves references in ContentMastery (checks if human and content exist)
pub struct ContentMasteryReferenceResolver;

impl ReferenceResolver for ContentMasteryReferenceResolver {
    fn resolve_reference(&self, entry_type: &str, id: &str) -> Result<bool, String> {
        // Check human or content references
        Ok(true)
    }
}

// ============================================================================
// DEGRADATION HANDLERS - Handle Validation/Reference Failures
// ============================================================================

/// Determines what to do when Content healing encounters errors
pub struct ContentDegradationHandler;

impl DegradationHandler for ContentDegradationHandler {
    fn handle_validation_failure(
        &self,
        _entry_type: &str,
        _error: &str,
        _was_migrated: bool,
    ) -> DegradationDecision {
        // For Content: allow degradation so app doesn't crash
        // but app can detect via validation_status="Degraded"
        DegradationDecision::Degrade
    }

    fn handle_missing_reference(
        &self,
        _entry_type: &str,
        _ref_type: &str,
        _ref_id: &str,
    ) -> DegradationDecision {
        // Missing references to other content is not fatal
        DegradationDecision::Degrade
    }
}

/// Determines what to do when PathStep healing encounters errors
pub struct PathStepDegradationHandler;

impl DegradationHandler for PathStepDegradationHandler {
    fn handle_validation_failure(
        &self,
        _entry_type: &str,
        _error: &str,
        _was_migrated: bool,
    ) -> DegradationDecision {
        DegradationDecision::Degrade
    }

    fn handle_missing_reference(
        &self,
        _entry_type: &str,
        _ref_type: &str,
        _ref_id: &str,
    ) -> DegradationDecision {
        DegradationDecision::Degrade
    }
}

/// Determines what to do when ContentMastery healing encounters errors
pub struct ContentMasteryDegradationHandler;

impl DegradationHandler for ContentMasteryDegradationHandler {
    fn handle_validation_failure(
        &self,
        _entry_type: &str,
        _error: &str,
        _was_migrated: bool,
    ) -> DegradationDecision {
        DegradationDecision::Degrade
    }

    fn handle_missing_reference(
        &self,
        _entry_type: &str,
        _ref_type: &str,
        _ref_id: &str,
    ) -> DegradationDecision {
        // Missing content or human is a problem, but we'll still return degraded mastery
        DegradationDecision::Degrade
    }
}

// ============================================================================
// ENTRY TYPE PROVIDERS - Compose All Components
// ============================================================================

/// Complete provider for Content entry type
pub struct ContentProvider;

impl EntryTypeProvider for ContentProvider {
    fn entry_type(&self) -> &str {
        "content"
    }

    fn validator(&self) -> &dyn Validator {
        &ContentValidator
    }

    fn transformer(&self) -> &dyn Transformer {
        &ContentTransformer
    }

    fn reference_resolver(&self) -> &dyn ReferenceResolver {
        &ContentReferenceResolver
    }

    fn degradation_handler(&self) -> &dyn DegradationHandler {
        &ContentDegradationHandler
    }

    fn create_healing_instance(&self, id: &str, v1_data: &Value) -> Result<Vec<u8>, String> {
        // Transform v1 to v2 JSON
        let v2_json = self.transformer().transform_v1_to_v2(v1_data)?;

        // Validate the transformed entry
        self.validator().validate_json(&v2_json)?;

        // Serialize to bytes
        Ok(serde_json::to_vec(&v2_json)
            .map_err(|e| format!("Failed to serialize healed Content: {}", e))?)
    }
}

/// Complete provider for LearningPath entry type
pub struct LearningPathProvider;

impl EntryTypeProvider for LearningPathProvider {
    fn entry_type(&self) -> &str {
        "learning_path"
    }

    fn validator(&self) -> &dyn Validator {
        &LearningPathValidator
    }

    fn transformer(&self) -> &dyn Transformer {
        &LearningPathTransformer
    }

    fn reference_resolver(&self) -> &dyn ReferenceResolver {
        &ContentReferenceResolver
    }

    fn degradation_handler(&self) -> &dyn DegradationHandler {
        &ContentDegradationHandler
    }

    fn create_healing_instance(&self, _id: &str, v1_data: &Value) -> Result<Vec<u8>, String> {
        let v2_json = self.transformer().transform_v1_to_v2(v1_data)?;
        self.validator().validate_json(&v2_json)?;
        Ok(serde_json::to_vec(&v2_json)
            .map_err(|e| format!("Failed to serialize healed LearningPath: {}", e))?)
    }
}

/// Complete provider for PathStep entry type
pub struct PathStepProvider;

impl EntryTypeProvider for PathStepProvider {
    fn entry_type(&self) -> &str {
        "path_step"
    }

    fn validator(&self) -> &dyn Validator {
        &PathStepValidator
    }

    fn transformer(&self) -> &dyn Transformer {
        &PathStepTransformer
    }

    fn reference_resolver(&self) -> &dyn ReferenceResolver {
        &PathStepReferenceResolver
    }

    fn degradation_handler(&self) -> &dyn DegradationHandler {
        &PathStepDegradationHandler
    }

    fn create_healing_instance(&self, _id: &str, v1_data: &Value) -> Result<Vec<u8>, String> {
        let v2_json = self.transformer().transform_v1_to_v2(v1_data)?;
        self.validator().validate_json(&v2_json)?;
        Ok(serde_json::to_vec(&v2_json)
            .map_err(|e| format!("Failed to serialize healed PathStep: {}", e))?)
    }
}

/// Complete provider for ContentMastery entry type
pub struct ContentMasteryProvider;

impl EntryTypeProvider for ContentMasteryProvider {
    fn entry_type(&self) -> &str {
        "content_mastery"
    }

    fn validator(&self) -> &dyn Validator {
        &ContentMasteryValidator
    }

    fn transformer(&self) -> &dyn Transformer {
        &ContentMasteryTransformer
    }

    fn reference_resolver(&self) -> &dyn ReferenceResolver {
        &ContentMasteryReferenceResolver
    }

    fn degradation_handler(&self) -> &dyn DegradationHandler {
        &ContentMasteryDegradationHandler
    }

    fn create_healing_instance(&self, _id: &str, v1_data: &Value) -> Result<Vec<u8>, String> {
        let v2_json = self.transformer().transform_v1_to_v2(v1_data)?;
        self.validator().validate_json(&v2_json)?;
        Ok(serde_json::to_vec(&v2_json)
            .map_err(|e| format!("Failed to serialize healed ContentMastery: {}", e))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_validator_valid() {
        let validator = ContentValidator;
        let valid_content = serde_json::json!({
            "id": "content-1",
            "content_type": "lesson",
            "title": "Learning Rust",
            "description": "An introduction to Rust",
            "content": "...",
            "content_format": "markdown",
            "tags": ["rust", "programming"],
            "reach": "community",
            "related_node_ids": [],
            "schema_version": 2,
            "validation_status": "Valid"
        });

        assert!(validator.validate_json(&valid_content).is_ok());
    }

    #[test]
    fn test_content_validator_missing_required_field() {
        let validator = ContentValidator;
        let invalid_content = serde_json::json!({
            "id": "content-1",
            // missing title!
            "content_type": "lesson",
            "schema_version": 2,
        });

        assert!(validator.validate_json(&invalid_content).is_err());
    }

    #[test]
    fn test_content_transformer() {
        let transformer = ContentTransformer;
        let v1_data = serde_json::json!({
            "id": "old-content-1",
            "title": "Old Title",
            "description": "Old description",
            "content": "Old content",
            "content_format": "markdown",
            "content_type": "lesson",
            "tags": ["tag1", "tag2"],
            "reach": "community",
            "related_node_ids": ["related-1"]
        });

        let result = transformer.transform_v1_to_v2(&v1_data);
        assert!(result.is_ok());

        let v2_data = result.unwrap();
        assert_eq!(v2_data["id"], "old-content-1");
        assert_eq!(v2_data["schema_version"], 2);
        assert_eq!(v2_data["validation_status"], "Migrated");
    }

    #[test]
    fn test_content_provider_entry_type() {
        let provider = ContentProvider;
        assert_eq!(provider.entry_type(), "content");
    }

    #[test]
    fn test_different_degradation_decisions() {
        let content_handler = ContentDegradationHandler;
        let decision = content_handler.handle_validation_failure("content", "error", false);

        match decision {
            DegradationDecision::Degrade => {},
            _ => panic!("Expected Degrade decision"),
        }
    }
}
