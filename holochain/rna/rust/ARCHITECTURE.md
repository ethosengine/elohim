# Flexible Migration Architecture

## Overview

The RNA module now supports a **registry-based, strategy-driven** architecture for migration and healing. This provides maximum flexibility for different applications without requiring modifications to the core framework.

## Core Concept

Instead of hard-coding healing logic:
```rust
// ❌ Old way - hard-coded in framework
pub fn heal() {
    if entry_type == "content" {
        // content-specific logic
    } else if entry_type == "learning_path" {
        // path-specific logic
    }
}
```

We use **pluggable providers and strategies**:
```rust
// ✅ New way - data-driven
let registry = EntryTypeRegistry::new();
registry.register(Arc::new(ContentProvider))?;      // No framework changes needed
registry.register(Arc::new(LearningPathProvider))?; // Just provide implementations

let orchestrator = FlexibleOrchestrator::new(config, registry);
// Orchestrator automatically uses the right provider for each type
```

## Architecture Layers

### 1. **Entry Type Provider** (`entry_type_provider.rs`)

Defines what each entry type needs to support healing:

```rust
pub trait EntryTypeProvider {
    fn entry_type(&self) -> &str;
    fn validator(&self) -> &dyn Validator;
    fn transformer(&self) -> &dyn Transformer;
    fn reference_resolver(&self) -> &dyn ReferenceResolver;
    fn degradation_handler(&self) -> &dyn DegradationHandler;
}
```

**This is where you implement:**
- Schema validation (required fields, enums, constraints)
- V1→V2 transformation
- Reference resolution (are linked entries available?)
- Degradation policy (fail or mark as Degraded?)

### 2. **Healing Strategy** (`healing_strategy.rs`)

Defines HOW to attempt healing:

```rust
pub trait HealingStrategy {
    fn heal(
        &self,
        entry_type: &str,
        entry_id: &str,
        v2_entry: Option<Vec<u8>>,
        context: &HealingContext,
    ) -> Result<Option<HealingResult<Vec<u8>>>, String>;
}
```

**Built-in strategies:**
- `BridgeFirstStrategy` - Try v1 bridge, fall back to self-repair
- `SelfRepairFirstStrategy` - Try local repair first, use v1 as fallback
- `LocalRepairOnlyStrategy` - Never use v1 bridge
- `NoHealingStrategy` - Accept entries as-is

**Extend it:**
- Try both in parallel, use fastest response
- Try cloud-based healing service
- ML-based entry repair
- Custom app-specific logic

### 3. **Orchestrator** (`flexible_orchestrator.rs`)

Coordinates healing using providers and strategies:

```rust
pub struct FlexibleOrchestrator {
    config: OrchestratorConfig,
    registry: EntryTypeRegistry,
}

impl FlexibleOrchestrator {
    pub fn heal_by_id(&self, entry_type: &str, id: &str, v2_entry: Option<Vec<u8>>)
        -> Result<Option<HealingOutcome>, String>
    {
        // 1. Look up provider in registry
        let provider = self.registry.get(entry_type)?;

        // 2. Create context with provider's validators/transformers
        let context = create_context(provider);

        // 3. Use strategy to heal
        self.config.healing_strategy.heal(entry_type, id, v2_entry, &context)?

        // 4. Return outcome with metadata
    }
}
```

## Usage Pattern

### For Lamad (or any application):

**1. Create providers for each entry type:**

```rust
// src/healing/content_provider.rs
use hc_rna::EntryTypeProvider;

pub struct ContentProvider;

impl EntryTypeProvider for ContentProvider {
    fn entry_type(&self) -> &str { "content" }

    fn validator(&self) -> &dyn Validator {
        &ContentValidator
    }

    fn transformer(&self) -> &dyn Transformer {
        &ContentTransformer
    }

    // ... implement other traits
}

// Repeat for LearningPathProvider, PathStepProvider, etc.
```

**2. Register them at startup:**

```rust
// In your init() function
pub fn init() -> ExternResult<InitCallbackResult> {
    let mut registry = EntryTypeRegistry::new();

    registry.register(Arc::new(ContentProvider))?;
    registry.register(Arc::new(LearningPathProvider))?;
    registry.register(Arc::new(PathStepProvider))?;
    registry.register(Arc::new(ContentMasteryProvider))?;

    let config = FlexibleOrchestratorConfig {
        healing_strategy: Arc::new(BridgeFirstStrategy),
        allow_degradation: true,
        ..Default::default()
    };

    let orchestrator = FlexibleOrchestrator::new(config, registry);
    ORCHESTRATOR.set(orchestrator).ok();

    Ok(InitCallbackResult::Pass)
}
```

**3. Use in read paths:**

```rust
pub fn get_content_by_id(id: &str) -> ExternResult<Option<Content>> {
    // Try v2 first
    if let Some(entry) = query_v2(id)? {
        return Ok(Some(entry));
    }

    // Heal from v1 if needed
    let orchestrator = ORCHESTRATOR.get();
    if let Some(healed) = orchestrator.heal_by_id("content", id, None)? {
        let content: Content = serde_json::from_slice(&healed.healed_entry)?;
        return Ok(Some(content));
    }

    Ok(None)
}
```

## Key Advantages

### 1. **Zero Core Changes for New Entry Types**

To add `Assessment` entry type:
- Just implement `AssessmentProvider`
- Register it in init()
- No changes to orchestrator, strategies, or RNA module

### 2. **Pluggable Strategies**

Switch strategies based on deployment:
```rust
let strategy = if env::var("USE_BRIDGE").is_ok() {
    Arc::new(BridgeFirstStrategy)
} else {
    Arc::new(LocalRepairOnlyStrategy)
};

let config = FlexibleOrchestratorConfig {
    healing_strategy: strategy,
    ..Default::default()
};
```

### 3. **Isolated Concerns**

- **Validators** - Only validate, don't heal
- **Transformers** - Only transform, don't validate
- **Strategies** - Only orchestrate, don't validate/transform
- **Providers** - Plug validators/transformers together

Change one, others unaffected.

### 4. **Testability**

```rust
#[test]
fn test_content_validation() {
    let validator = ContentValidator;
    let data = serde_json::json!({ "id": "", "title": "test" });

    assert!(validator.validate_json(&data).is_err()); // Missing id
}

#[test]
fn test_bridge_first_strategy() {
    let strategy = BridgeFirstStrategy;
    let context = create_test_context();

    let result = strategy.heal("content", "id", None, &context)?;
    // Strategy doesn't need to know about Lamad specifics
}
```

### 5. **Different Apps, Different Choices**

- **App A**: Use BridgeFirstStrategy, allow degradation, all entry types
- **App B**: Use LocalRepairOnlyStrategy, fail on error, only content
- **App C**: Custom strategy, custom degradation policy, specific types

All using the same RNA framework, zero code conflicts.

## Migration Guide (from old architecture)

### Old way (Lamad before refactor):
```
healing_impl.rs: SelfHealingEntry impls for Content/LearningPath/etc
healing_integration.rs: Hard-coded get_*_with_healing() functions
```

### New way (Lamad after refactor):
```
healing/content_provider.rs: ContentProvider impl
healing/learning_path_provider.rs: LearningPathProvider impl
healing/orchestrator_setup.rs: Register all in init()
coordinator/zome.rs: Use orchestrator in read paths
```

**Old code removed entirely** - no duplications, no conflicts.

## Trait Implementations

### Validator

```rust
pub trait Validator: Send + Sync {
    fn validate_json(&self, data: &Value) -> Result<(), String>;
}
```

Implement once per entry type. Examples:
- Check required fields
- Validate enum values
- Validate field constraints
- Never: resolve references (that's ReferenceResolver's job)

### Transformer

```rust
pub trait Transformer: Send + Sync {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String>;
}
```

Implement once per entry type. Examples:
- Map old field names to new
- Normalize data values
- Add new required fields with defaults
- Never: validate the result (that's Validator's job)

### ReferenceResolver

```rust
pub trait ReferenceResolver: Send + Sync {
    fn resolve_reference(&self, entry_type: &str, id: &str) -> Result<bool, String>;
}
```

Implement once per entry type. Examples:
- Check if referenced content exists
- Check if referenced path exists
- Never: block healing on failure (that's DegradationHandler's job)

### DegradationHandler

```rust
pub trait DegradationHandler: Send + Sync {
    fn handle_validation_failure(...) -> DegradationDecision;
    fn handle_missing_reference(...) -> DegradationDecision;
}
```

Implement once per entry type. Decides:
- Should validation failure mark as Degraded or fail completely?
- Should missing reference block healing or allow degradation?

## Testing Strategy

```rust
// Test each provider independently
#[test]
fn test_content_provider() {
    let provider = ContentProvider;

    // Test validator
    assert!(provider.validator().validate_json(&invalid_data).is_err());

    // Test transformer
    let v1 = serde_json::json!({ "old_field": "value" });
    let v2 = provider.transformer().transform_v1_to_v2(&v1)?;
    assert_eq!(v2["new_field"], "value");

    // Test resolver
    assert!(provider.reference_resolver().resolve_reference("content", "id")?);
}

// Test each strategy independently
#[test]
fn test_bridge_first_strategy() {
    let strategy = BridgeFirstStrategy;
    let context = create_mock_context();

    let result = strategy.heal("content", "id", None, &context)?;
    // No Lamad-specific knowledge needed
}

// Test orchestrator integration
#[test]
fn test_orchestrator_with_registry() {
    let mut registry = EntryTypeRegistry::new();
    registry.register(Arc::new(MockContentProvider))?;

    let orchestrator = FlexibleOrchestrator::new(
        FlexibleOrchestratorConfig::default(),
        registry
    );

    assert!(orchestrator.supports_entry_type("content"));

    let outcome = orchestrator.heal_by_id("content", "id", None)?;
    // Integration works without real DHT
}
```

## Summary

The flexible architecture provides:

1. **Registry pattern** for zero-modification extensibility
2. **Trait-based composition** for clean separation of concerns
3. **Pluggable strategies** for different healing approaches
4. **Configuration injection** for deployment flexibility
5. **Complete isolation** - changes to one entry type don't affect others

This allows Lamad (or any app) to:
- Support any number of entry types
- Switch healing strategies without code changes
- Test each component independently
- Deploy different configurations to different environments
- Add new entry types with zero framework modifications

**The key insight:** Move logic out of the framework and into pluggable providers registered at startup.
