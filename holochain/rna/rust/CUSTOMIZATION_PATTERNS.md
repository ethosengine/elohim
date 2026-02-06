# Provider Customization Patterns

After generating providers with `hc-rna-generate`, you'll need to customize the generated code for your specific domain. This guide shows common patterns and best practices.

## Pattern 1: Enum Validation

### Problem
Generated validators check required fields but don't validate enum values.

### Solution

**Before (generated):**
```rust
pub struct ContentValidator;

impl Validator for ContentValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        let content_type = data["content_type"].as_str().ok_or("content_type required")?;
        // TODO: Add enum validation if needed
        Ok(())
    }
}
```

**After (customized):**
```rust
pub struct ContentValidator;

impl Validator for ContentValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        let content_type = data["content_type"].as_str().ok_or("content_type required")?;

        // ✓ Enum validation
        const VALID_CONTENT_TYPES: &[&str] = &[
            "concept", "lesson", "assessment", "scenario", "resource",
            "practice", "reflection", "reference"
        ];
        if !VALID_CONTENT_TYPES.contains(&content_type) {
            return Err(format!(
                "Invalid content_type '{}'. Must be one of: {:?}",
                content_type, VALID_CONTENT_TYPES
            ));
        }

        let reach = data["reach"].as_str().unwrap_or("community");
        const VALID_REACH: &[&str] = &[
            "private", "intimate", "trusted", "familiar", "community", "public", "commons"
        ];
        if !VALID_REACH.contains(&reach) {
            return Err(format!("Invalid reach '{}'", reach));
        }

        Ok(())
    }
}
```

### When to use
- Always validate enum fields to prevent invalid data
- Define constants at the top of the validator
- Provide helpful error messages

### Reusable pattern
```rust
macro_rules! validate_enum {
    ($data:expr, $field:expr, $valid_values:expr, $error_msg:expr) => {{
        let value = $data[$field].as_str().ok_or(concat!($field, " required"))?;
        if !$valid_values.contains(&value) {
            return Err(format!($error_msg, value));
        }
        value
    }};
}
```

## Pattern 2: Constraint Validation

### Problem
Generated validators don't check business constraints.

### Solution

**Before (generated):**
```rust
pub struct ContentValidator;

impl Validator for ContentValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        let id = data["id"].as_str().ok_or("id required")?;
        if id.is_empty() { return Err("id cannot be empty".to_string()); }

        // TODO: Add constraint validation if needed
        Ok(())
    }
}
```

**After (customized):**
```rust
pub struct ContentValidator;

impl Validator for ContentValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        let id = data["id"].as_str().ok_or("id required")?;
        if id.is_empty() { return Err("id cannot be empty".to_string()); }

        // ✓ ID format validation
        if !id.starts_with("content_") {
            return Err("content id must start with 'content_'".to_string());
        }

        let title = data["title"].as_str().ok_or("title required")?;

        // ✓ Length constraints
        if title.len() < 3 {
            return Err("title must be at least 3 characters".to_string());
        }
        if title.len() > 200 {
            return Err("title must be at most 200 characters".to_string());
        }

        // ✓ Tag validation
        if let Some(tags) = data["tags"].as_array() {
            if tags.len() > 10 {
                return Err("maximum 10 tags allowed".to_string());
            }
            for tag in tags {
                let tag_str = tag.as_str().ok_or("tag must be string")?;
                if tag_str.len() < 2 || tag_str.len() > 50 {
                    return Err("each tag must be 2-50 characters".to_string());
                }
            }
        }

        // ✓ Related entries count
        if let Some(related) = data["related_node_ids"].as_array() {
            if related.len() > 50 {
                return Err("maximum 50 related entries".to_string());
            }
        }

        Ok(())
    }
}
```

### When to use
- Format validation (IDs, URIs, emails)
- Length constraints (min/max)
- Numeric ranges
- Collection size limits
- Cross-field validation

## Pattern 3: Reference Resolution with DHT

### Problem
Generated resolvers return `Ok(true)` - they don't actually check if references exist.

### Solution

**Before (generated):**
```rust
pub struct ContentReferenceResolver;

impl ReferenceResolver for ContentReferenceResolver {
    fn resolve_reference(&self, entry_type: &str, _id: &str) -> Result<bool, String> {
        // TODO: Implement DHT lookups for referenced entries
        Ok(true)
    }
}
```

**After (customized):**
```rust
use hdk::prelude::*;

pub struct ContentReferenceResolver;

impl ReferenceResolver for ContentReferenceResolver {
    fn resolve_reference(&self, entry_type: &str, id: &str) -> Result<bool, String> {
        match entry_type {
            "content" => {
                // Check if referenced content exists
                match get_entry(id) {
                    Ok(Some(_)) => Ok(true),
                    Ok(None) => Ok(false),
                    Err(e) => Err(format!("DHT error: {}", e)),
                }
            }
            "agent" => {
                // Check if agent exists
                match get_entry(id) {
                    Ok(Some(_)) => Ok(true),
                    Ok(None) => Ok(false),
                    Err(e) => Err(format!("DHT error: {}", e)),
                }
            }
            _ => Ok(true), // Unknown type - assume it exists
        }
    }
}
```

### Advanced: Cached Resolution
```rust
pub struct ContentReferenceResolver {
    cache: std::cell::RefCell<std::collections::HashMap<String, bool>>,
}

impl ReferenceResolver for ContentReferenceResolver {
    fn resolve_reference(&self, entry_type: &str, id: &str) -> Result<bool, String> {
        let cache_key = format!("{}:{}", entry_type, id);

        // Check cache first
        if let Some(&cached) = self.cache.borrow().get(&cache_key) {
            return Ok(cached);
        }

        // Resolve and cache
        let exists = match get_entry(id) {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => return Err(format!("DHT error: {}", e)),
        };

        self.cache.borrow_mut().insert(cache_key, exists);
        Ok(exists)
    }
}
```

### When to use
- Check if referenced entries exist in DHT
- Verify relationships are valid
- Prevent orphaned references
- Cache results for performance

## Pattern 4: Custom Degradation Policies

### Problem
Generated degradation handlers use same policy for all failures.

### Solution

**Before (generated):**
```rust
pub struct ContentDegradationHandler;

impl DegradationHandler for ContentDegradationHandler {
    fn handle_validation_failure(...) -> DegradationDecision {
        DegradationDecision::Degrade  // Always degrade
    }

    fn handle_missing_reference(...) -> DegradationDecision {
        DegradationDecision::Degrade  // Always degrade
    }
}
```

**After (customized by entry type):**
```rust
// ✓ Content: graceful degradation
pub struct ContentDegradationHandler;

impl DegradationHandler for ContentDegradationHandler {
    fn handle_validation_failure(&self, _et: &str, error: &str, _was_migrated: bool) -> DegradationDecision {
        // Content can be shown with warnings
        if error.contains("tag") {
            DegradationDecision::Degrade  // Tag errors are non-critical
        } else {
            DegradationDecision::Fail  // ID/type errors are critical
        }
    }

    fn handle_missing_reference(&self, _et: &str, ref_type: &str, _ref_id: &str) -> DegradationDecision {
        // Missing related content is acceptable
        DegradationDecision::Degrade
    }
}

// ✓ CriticalData: strict mode
pub struct CriticalDataDegradationHandler;

impl DegradationHandler for CriticalDataDegradationHandler {
    fn handle_validation_failure(&self, _et: &str, _error: &str, _was_migrated: bool) -> DegradationDecision {
        // Critical data must be valid
        DegradationDecision::Fail
    }

    fn handle_missing_reference(&self, _et: &str, _ref_type: &str, _ref_id: &str) -> DegradationDecision {
        // All references must exist
        DegradationDecision::Fail
    }
}

// ✓ OptionalData: always accept
pub struct OptionalDataDegradationHandler;

impl DegradationHandler for OptionalDataDegradationHandler {
    fn handle_validation_failure(&self, _et: &str, _error: &str, _was_migrated: bool) -> DegradationDecision {
        // Optional data is used as-is
        DegradationDecision::Accept
    }

    fn handle_missing_reference(&self, _et: &str, _ref_type: &str, _ref_id: &str) -> DegradationDecision {
        // Missing optional references are fine
        DegradationDecision::Accept
    }
}
```

### When to use
- Content: Degrade (show with warnings)
- Critical metadata: Fail (must be valid)
- Analytics: Accept (always usable)
- Optional data: Accept (degrade gracefully)

## Pattern 5: Smart Transformation with Mapping

### Problem
Generated transformers assume v1 field names match v2.

### Solution

**Before (generated):**
```rust
pub struct ContentTransformer;

impl Transformer for ContentTransformer {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {
        let id = v1_data["id"].as_str().ok_or("v1 id missing")?;
        let title = v1_data["title"].as_str().ok_or("v1 title missing")?;
        // TODO: Map old_field_name → new_field_name if needed

        Ok(serde_json::json!({
            "id": id,
            "title": title,
            // ... etc
        }))
    }
}
```

**After (with field mapping):**
```rust
pub struct ContentTransformer {
    // v1 → v2 field name mapping
    field_map: std::collections::HashMap<&'static str, &'static str>,
}

impl ContentTransformer {
    pub fn new() -> Self {
        let mut field_map = std::collections::HashMap::new();

        // Map v1 field names to v2 field names
        field_map.insert("type", "content_type");          // Renamed field
        field_map.insert("time_estimate", "estimated_minutes"); // Different naming
        field_map.insert("source", "source_path");         // Abbreviated

        Self { field_map }
    }

    fn map_field(&self, v1_name: &str) -> &str {
        self.field_map.get(v1_name).map(|&s| s).unwrap_or(v1_name)
    }
}

impl Transformer for ContentTransformer {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {
        let mut v2 = serde_json::Map::new();

        // Transform each field with mapping
        for (key, value) in v1_data.as_object().ok_or("v1 must be object")? {
            let v2_key = self.map_field(key);

            // Transform value if needed
            let v2_value = match key.as_str() {
                "time_estimate" => {
                    // Convert string to number
                    value.as_str()
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(|n| serde_json::json!(n))
                        .ok_or("Invalid time_estimate")?
                }
                "tags" => {
                    // Ensure tags is array
                    if value.is_array() {
                        value.clone()
                    } else {
                        serde_json::json!([])
                    }
                }
                _ => value.clone(),
            };

            v2.insert(v2_key.to_string(), v2_value);
        }

        // Add schema version
        v2.insert("schema_version".to_string(), serde_json::json!(2));
        v2.insert("validation_status".to_string(), serde_json::json!("Migrated"));

        Ok(serde_json::Value::Object(v2))
    }

    fn description(&self) -> &str {
        "Transform Content from v1 to v2 schema with field mapping"
    }
}
```

### When to use
- Field names changed between versions
- Field types changed (string → number, etc.)
- New required fields need defaults
- Old fields need normalization
- Type conversion is needed

## Pattern 6: Logging and Debugging

### Problem
No visibility into what's happening during healing.

### Solution

```rust
use hdk::prelude::*;

pub struct ContentTransformer;

impl Transformer for ContentTransformer {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {
        debug!("ContentTransformer: Starting transformation");
        debug!("  v1 data: {:?}", v1_data);

        let id = v1_data["id"].as_str().ok_or("v1 id missing")?;
        debug!("  Extracted id: {}", id);

        // ... transformation logic ...

        let result = serde_json::json!({
            "id": id,
            // ... fields ...
            "schema_version": 2,
            "validation_status": "Migrated"
        });

        debug!("  Transformation complete, v2 size: {}", result.to_string().len());
        Ok(result)
    }

    fn description(&self) -> &str {
        "Transform Content from v1 to v2 schema"
    }
}

pub struct ContentValidator;

impl Validator for ContentValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        let id = data["id"].as_str().ok_or_else(|| {
            warn!("ContentValidator: Missing id field");
            "id required".to_string()
        })?;

        const VALID_TYPES: &[&str] = &["concept", "lesson"];
        let content_type = data["content_type"].as_str().unwrap_or("unknown");

        if !VALID_TYPES.contains(&content_type) {
            warn!("ContentValidator: Invalid content_type '{}' for id '{}'", content_type, id);
            return Err(format!("Invalid content_type: {}", content_type));
        }

        debug!("ContentValidator: {} is valid", id);
        Ok(())
    }
}
```

### When to use
- Debug healing failures
- Audit transformation process
- Monitor performance
- Investigate data quality issues

## Testing Customized Providers

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_validator_enum_validation() {
        let validator = ContentValidator;
        let invalid = serde_json::json!({
            "id": "content_1",
            "content_type": "invalid_type",  // Not in VALID_TYPES
            "schema_version": 2,
        });
        assert!(validator.validate_json(&invalid).is_err());
    }

    #[test]
    fn test_content_transformer_field_mapping() {
        let transformer = ContentTransformer::new();
        let v1_data = serde_json::json!({
            "id": "old-id",
            "type": "lesson",  // v1 field name
            "title": "Test",
            "time_estimate": "45"  // v1: string
        });

        let v2_data = transformer.transform_v1_to_v2(&v1_data).unwrap();

        // Verify field mapping
        assert_eq!(v2_data["content_type"], "lesson");  // "type" → "content_type"
        assert_eq!(v2_data["estimated_minutes"], 45);   // "time_estimate" → "estimated_minutes"
        assert_eq!(v2_data["schema_version"], 2);
    }

    #[test]
    fn test_content_degradation_handler_policies() {
        let handler = ContentDegradationHandler;

        // Content errors degrade
        assert_eq!(
            handler.handle_validation_failure("content", "tag validation failed", false),
            DegradationDecision::Degrade
        );

        // Critical errors fail
        assert_eq!(
            handler.handle_validation_failure("content", "id is invalid", false),
            DegradationDecision::Fail
        );
    }
}
```

## Summary

| Pattern | When to Use | Complexity |
|---------|------------|-----------|
| Enum Validation | Always (for enum fields) | Low |
| Constraint Validation | Domain-specific rules needed | Medium |
| Reference Resolution | Data relationships matter | Medium |
| Custom Degradation | Different entry types need different policies | Medium |
| Field Mapping | Schema changed between versions | High |
| Logging/Debug | Troubleshooting healing issues | Low |

**Start with enum validation, then add other patterns as needed.**
