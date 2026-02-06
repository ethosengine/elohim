# Provider Testing Guide

After generating and customizing providers with `hc-rna-generate`, comprehensive testing ensures your healing implementation works correctly.

## Testing Strategy

```
Unit Tests (Individual Components)
  ├─ Validator tests
  ├─ Transformer tests
  ├─ Resolver tests
  └─ Handler tests
       ↓
Integration Tests (Multi-Component)
  ├─ Provider composition tests
  ├─ Strategy with provider tests
  └─ Full healing workflow tests
       ↓
End-to-End Tests (Real DNA)
  ├─ v1→v2 bridge tests
  ├─ Healing signal tests
  └─ Real DHT tests
```

## Unit Tests: Validators

### Test Required Fields

```rust
#[cfg(test)]
mod validator_tests {
    use super::*;

    #[test]
    fn test_content_validator_requires_id() {
        let validator = ContentValidator;
        let missing_id = serde_json::json!({
            "title": "Test",
            "schema_version": 2,
        });

        let result = validator.validate_json(&missing_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("id"));
    }

    #[test]
    fn test_content_validator_rejects_empty_id() {
        let validator = ContentValidator;
        let empty_id = serde_json::json!({
            "id": "",
            "title": "Test",
            "schema_version": 2,
        });

        let result = validator.validate_json(&empty_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_content_validator_accepts_valid_data() {
        let validator = ContentValidator;
        let valid = serde_json::json!({
            "id": "content_1",
            "content_type": "lesson",
            "title": "Test",
            "schema_version": 2,
        });

        assert!(validator.validate_json(&valid).is_ok());
    }
}
```

### Test Enum Validation

```rust
#[test]
fn test_content_validator_enum_validation() {
    let validator = ContentValidator;

    // Valid enum values
    let valid = serde_json::json!({
        "id": "content_1",
        "content_type": "lesson",
        "reach": "community",
        "schema_version": 2,
    });
    assert!(validator.validate_json(&valid).is_ok());

    // Invalid enum value
    let invalid_type = serde_json::json!({
        "id": "content_1",
        "content_type": "invalid_type",
        "reach": "community",
        "schema_version": 2,
    });
    assert!(validator.validate_json(&invalid_type).is_err());

    // Invalid reach
    let invalid_reach = serde_json::json!({
        "id": "content_1",
        "content_type": "lesson",
        "reach": "invalid_reach",
        "schema_version": 2,
    });
    assert!(validator.validate_json(&invalid_reach).is_err());
}
```

### Test Constraints

```rust
#[test]
fn test_content_validator_length_constraints() {
    let validator = ContentValidator;

    // Title too short
    let short_title = serde_json::json!({
        "id": "content_1",
        "title": "a",  // Less than 3 characters
        "content_type": "lesson",
        "schema_version": 2,
    });
    assert!(validator.validate_json(&short_title).is_err());

    // Title too long
    let long_title = serde_json::json!({
        "id": "content_1",
        "title": "a".repeat(201),  // More than 200 characters
        "content_type": "lesson",
        "schema_version": 2,
    });
    assert!(validator.validate_json(&long_title).is_err());

    // Title just right
    let good_title = serde_json::json!({
        "id": "content_1",
        "title": "Valid Title",
        "content_type": "lesson",
        "schema_version": 2,
    });
    assert!(validator.validate_json(&good_title).is_ok());
}
```

### Test Schema Version

```rust
#[test]
fn test_content_validator_schema_version() {
    let validator = ContentValidator;

    // Wrong version
    let v1 = serde_json::json!({
        "id": "content_1",
        "title": "Test",
        "content_type": "lesson",
        "schema_version": 1,  // Wrong version
    });
    assert!(validator.validate_json(&v1).is_err());

    // Correct version
    let v2 = serde_json::json!({
        "id": "content_1",
        "title": "Test",
        "content_type": "lesson",
        "schema_version": 2,
    });
    assert!(validator.validate_json(&v2).is_ok());

    // Missing version defaults to 0
    let no_version = serde_json::json!({
        "id": "content_1",
        "title": "Test",
        "content_type": "lesson",
    });
    assert!(validator.validate_json(&no_version).is_err());
}
```

## Unit Tests: Transformers

### Test Field Extraction

```rust
#[cfg(test)]
mod transformer_tests {
    use super::*;

    #[test]
    fn test_content_transformer_extracts_all_fields() {
        let transformer = ContentTransformer::new();
        let v1_data = serde_json::json!({
            "id": "old_content_1",
            "type": "lesson",
            "title": "Old Title",
            "description": "Old description",
            "content": "Old content",
            "tags": ["tag1", "tag2"],
        });

        let v2_result = transformer.transform_v1_to_v2(&v1_data);
        assert!(v2_result.is_ok());

        let v2_data = v2_result.unwrap();
        assert_eq!(v2_data["id"], "old_content_1");
        assert_eq!(v2_data["title"], "Old Title");
        assert_eq!(v2_data["tags"][0], "tag1");
    }

    #[test]
    fn test_content_transformer_handles_missing_optional_fields() {
        let transformer = ContentTransformer::new();
        let v1_data = serde_json::json!({
            "id": "content_1",
            "type": "lesson",
            "title": "Minimal",
            // Missing optional fields
        });

        let v2_result = transformer.transform_v1_to_v2(&v1_data);
        assert!(v2_result.is_ok());

        let v2_data = v2_result.unwrap();
        assert_eq!(v2_data["id"], "content_1");
        // Optional fields use defaults
    }

    #[test]
    fn test_content_transformer_sets_schema_version() {
        let transformer = ContentTransformer::new();
        let v1_data = serde_json::json!({
            "id": "content_1",
            "type": "lesson",
            "title": "Test",
        });

        let v2_data = transformer.transform_v1_to_v2(&v1_data).unwrap();

        // Always set to v2
        assert_eq!(v2_data["schema_version"], 2);
        assert_eq!(v2_data["validation_status"], "Migrated");
    }

    #[test]
    fn test_content_transformer_field_mapping() {
        let transformer = ContentTransformer::new();
        let v1_data = serde_json::json!({
            "id": "content_1",
            "type": "lesson",              // v1 field name
            "title": "Test",
            "time_estimate": "45",         // v1 different naming
        });

        let v2_data = transformer.transform_v1_to_v2(&v1_data).unwrap();

        // Verify field name mapping
        assert_eq!(v2_data["content_type"], "lesson");  // "type" → "content_type"
        assert_eq!(v2_data["estimated_minutes"], 45);   // "time_estimate" → "estimated_minutes"
    }
}
```

### Test Type Conversions

```rust
#[test]
fn test_content_transformer_type_conversions() {
    let transformer = ContentTransformer::new();

    // String to number conversion
    let v1_data = serde_json::json!({
        "id": "content_1",
        "title": "Test",
        "time_estimate": "60",  // String
    });

    let v2_data = transformer.transform_v1_to_v2(&v1_data).unwrap();
    assert_eq!(v2_data["estimated_minutes"], 60);  // Now number
    assert!(v2_data["estimated_minutes"].is_number());
}
```

## Unit Tests: Reference Resolvers

### Test Reference Detection

```rust
#[cfg(test)]
mod resolver_tests {
    use super::*;

    #[test]
    fn test_content_reference_resolver_known_type() {
        let resolver = ContentReferenceResolver;

        // Mock DHT: content exists
        let result = resolver.resolve_reference("content", "content_1");
        assert!(result.is_ok());
        // In real implementation, would check DHT
    }

    #[test]
    fn test_content_reference_resolver_unknown_type() {
        let resolver = ContentReferenceResolver;

        // Unknown types are assumed to exist
        let result = resolver.resolve_reference("unknown_type", "id_123");
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_content_reference_resolver_missing_reference() {
        let resolver = ContentReferenceResolver;

        // When implementation adds real DHT checks:
        let result = resolver.resolve_reference("content", "nonexistent_id");
        // assert_eq!(result.unwrap(), false);
    }
}
```

## Unit Tests: Degradation Handlers

### Test Degradation Decisions

```rust
#[cfg(test)]
mod handler_tests {
    use super::*;

    #[test]
    fn test_content_degradation_handler_validation_failure() {
        let handler = ContentDegradationHandler;

        // Minor validation error → degrade
        let decision = handler.handle_validation_failure(
            "content",
            "tag validation failed",
            false,
        );
        assert_eq!(decision, DegradationDecision::Degrade);
    }

    #[test]
    fn test_content_degradation_handler_critical_error() {
        let handler = ContentDegradationHandler;

        // Critical error → fail
        let decision = handler.handle_validation_failure(
            "content",
            "id is invalid",
            false,
        );
        assert_eq!(decision, DegradationDecision::Fail);
    }

    #[test]
    fn test_content_degradation_handler_missing_reference() {
        let handler = ContentDegradationHandler;

        // Missing optional reference → degrade
        let decision = handler.handle_missing_reference(
            "content",
            "related_content",
            "missing_id",
        );
        assert_eq!(decision, DegradationDecision::Degrade);
    }

    #[test]
    fn test_critical_data_handler_strict_mode() {
        let handler = CriticalDataDegradationHandler;

        // Any validation failure → fail
        let decision = handler.handle_validation_failure(
            "critical_data",
            "minor error",
            false,
        );
        assert_eq!(decision, DegradationDecision::Fail);
    }
}
```

## Integration Tests: Provider Composition

### Test Provider Methods

```rust
#[cfg(test)]
mod provider_tests {
    use super::*;

    #[test]
    fn test_content_provider_returns_correct_entry_type() {
        let provider = ContentProvider;
        assert_eq!(provider.entry_type(), "content");
    }

    #[test]
    fn test_content_provider_has_all_components() {
        let provider = ContentProvider;

        // Provider should have all four components
        assert!(!provider.validator() as *const _ as usize == 0);
        assert!(!provider.transformer() as *const _ as usize == 0);
        assert!(!provider.reference_resolver() as *const _ as usize == 0);
        assert!(!provider.degradation_handler() as *const _ as usize == 0);
    }

    #[test]
    fn test_content_provider_create_healing_instance() {
        let provider = ContentProvider;
        let v1_data = serde_json::json!({
            "id": "content_1",
            "type": "lesson",
            "title": "Test",
        });

        let result = provider.create_healing_instance("id_1", &v1_data);
        assert!(result.is_ok());

        // Result should be valid bytes
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
}
```

## Integration Tests: Healing Strategies

### Test Strategy with Provider

```rust
#[cfg(test)]
mod strategy_tests {
    use super::*;
    use hc_rna::{HealingStrategy, HealingContext, BridgeFirstStrategy};

    #[test]
    fn test_bridge_first_strategy_with_content_provider() {
        let strategy = BridgeFirstStrategy;
        let provider = ContentProvider;

        // Create mock healing context
        let validator = provider.validator();
        let transformer = provider.transformer();
        let resolver = provider.reference_resolver();

        // Mock v1 data
        let v1_bytes = Some(br#"{"id":"c1","type":"lesson","title":"Test"}"#.to_vec());

        // In real implementation, would test healing flow
        // This would require mocking bridge calls
    }
}
```

## End-to-End Tests: Real Healing

### Test Full Workflow

```rust
#[cfg(test)]
mod e2e_tests {
    use super::*;
    use hc_rna::{FlexibleOrchestrator, FlexibleOrchestratorConfig, EntryTypeRegistry, BridgeFirstStrategy};
    use std::sync::Arc;

    #[test]
    fn test_full_healing_workflow() {
        // Setup
        let mut registry = EntryTypeRegistry::new();
        registry.register(Arc::new(ContentProvider)).unwrap();

        let config = FlexibleOrchestratorConfig {
            v1_role_name: Some("v1".to_string()),
            v2_role_name: Some("v2".to_string()),
            healing_strategy: Arc::new(BridgeFirstStrategy),
            allow_degradation: true,
            max_attempts: 3,
            emit_signals: true,
        };

        let orchestrator = FlexibleOrchestrator::new(config, registry);

        // Verify orchestrator is ready
        assert!(orchestrator.supports_entry_type("content"));

        // In integration test, would:
        // 1. Set up v1 DNA with data
        // 2. Call heal_by_id
        // 3. Verify healing result
        // 4. Check data was transformed correctly
        // 5. Verify validation passed
    }

    #[test]
    fn test_healing_with_degradation() {
        // Test that degraded entries are still returned
        let mut registry = EntryTypeRegistry::new();
        registry.register(Arc::new(ContentProvider)).unwrap();

        let config = FlexibleOrchestratorConfig {
            allow_degradation: true,
            ..Default::default()
        };

        let orchestrator = FlexibleOrchestrator::new(config, registry);

        // In integration test:
        // 1. Heal entry that fails validation
        // 2. Verify degraded entry is returned
        // 3. Check validation_status is "Degraded"
    }
}
```

## Test Checklist

### Before Deploying Providers

- [ ] **Unit Tests**
  - [ ] Validators: all required fields
  - [ ] Validators: all enum values
  - [ ] Validators: all constraints
  - [ ] Validators: schema version
  - [ ] Transformers: all field extractions
  - [ ] Transformers: optional field handling
  - [ ] Transformers: type conversions
  - [ ] Transformers: field mapping
  - [ ] Resolvers: known types
  - [ ] Resolvers: unknown types
  - [ ] Handlers: degradation decisions

- [ ] **Integration Tests**
  - [ ] Provider composition
  - [ ] Provider methods
  - [ ] Provider instances
  - [ ] Strategy with provider
  - [ ] Multiple entry types

- [ ] **End-to-End Tests**
  - [ ] V1→V2 transformation
  - [ ] Healing with success
  - [ ] Healing with degradation
  - [ ] Healing failure handling
  - [ ] Registration in orchestrator

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_content_validator_enum_validation

# Run with output
cargo test -- --nocapture

# Run specific module
cargo test validator_tests

# Check coverage
cargo tarpaulin --out Html
```

## Test Data Generators

Helper function to generate consistent test data:

```rust
mod test_helpers {
    use serde_json::json;

    pub fn valid_content_v1() -> serde_json::Value {
        json!({
            "id": "content_test_1",
            "type": "lesson",
            "title": "Test Content",
            "description": "Test description",
            "content": "Test content",
            "tags": ["test"],
        })
    }

    pub fn valid_content_v2() -> serde_json::Value {
        json!({
            "id": "content_test_1",
            "content_type": "lesson",
            "title": "Test Content",
            "description": "Test description",
            "content": "Test content",
            "tags": ["test"],
            "schema_version": 2,
            "validation_status": "Migrated",
        })
    }

    pub fn invalid_content_missing_id() -> serde_json::Value {
        json!({
            "type": "lesson",
            "title": "Test",
        })
    }

    pub fn invalid_content_bad_type() -> serde_json::Value {
        json!({
            "id": "content_1",
            "content_type": "invalid_type",
            "title": "Test",
        })
    }
}
```

## Coverage Goals

| Component | Target Coverage | Why |
|-----------|-----------------|-----|
| Validator | 100% | Validate all branches, enums, constraints |
| Transformer | 100% | All field mappings and conversions |
| Resolver | 90%+ | Hard to test without mocking DHT |
| Handler | 100% | All degradation decision paths |
| Provider | 95%+ | All public methods and integrations |

## Performance Testing

```rust
#[test]
fn test_transformer_performance() {
    let transformer = ContentTransformer::new();
    let v1_data = serde_json::json!({
        "id": "content_1",
        "type": "lesson",
        "title": "Test",
        // ... many more fields
    });

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = transformer.transform_v1_to_v2(&v1_data);
    }
    let elapsed = start.elapsed();

    // Should complete 1000 transformations in < 100ms
    assert!(elapsed.as_millis() < 100, "Transformation too slow: {:?}", elapsed);
}
```

## Next Steps

1. Write unit tests for each validator, transformer, resolver, handler
2. Write integration tests for providers and strategies
3. Write end-to-end tests for full healing workflow
4. Achieve 95%+ code coverage
5. Run performance tests under load
6. Deploy with confidence!
