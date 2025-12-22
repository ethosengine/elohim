//! Flexible Healing Orchestrator using Registry and Strategies
//!
//! This is the new orchestrator that leverages:
//! - EntryTypeRegistry: Knows how to handle each entry type
//! - HealingStrategy: Pluggable healing approach
//! - Configuration injection: No hard-coded values
//!
//! This allows complete flexibility in how healing behaves without touching
//! the RNA framework code.

use std::sync::Arc;
use crate::entry_type_provider::EntryTypeRegistry;
use crate::healing_strategy::{HealingStrategy, HealingContext, ValidationProvider, TransformationProvider, ReferenceResolutionProvider};

/// Configuration for the healing orchestrator
pub struct OrchestratorConfig {
    /// V1 DNA role name (if different from default "lamad-v1")
    pub v1_role_name: Option<String>,

    /// V2 DNA role name (if different from default "lamad-v2")
    pub v2_role_name: Option<String>,

    /// Healing strategy to use
    pub healing_strategy: Arc<dyn HealingStrategy>,

    /// Whether to allow entries to be marked as Degraded
    pub allow_degradation: bool,

    /// Maximum healing attempts
    pub max_attempts: u32,

    /// Whether to emit signals on healing events
    pub emit_signals: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            v1_role_name: Some("lamad-v1".to_string()),
            v2_role_name: Some("lamad-v2".to_string()),
            healing_strategy: Arc::new(crate::healing_strategy::BridgeFirstStrategy),
            allow_degradation: true,
            max_attempts: 3,
            emit_signals: true,
        }
    }
}

/// Adapter that implements ValidationProvider using provider traits
struct ProviderValidationAdapter<'a> {
    validator: &'a dyn crate::entry_type_provider::Validator,
}

impl<'a> ValidationProvider for ProviderValidationAdapter<'a> {
    fn validate_json(&self, _entry_type: &str, data: &serde_json::Value) -> Result<(), String> {
        self.validator.validate_json(data)
    }
}

/// Adapter that implements TransformationProvider using provider traits
struct ProviderTransformationAdapter<'a> {
    transformer: &'a dyn crate::entry_type_provider::Transformer,
}

impl<'a> TransformationProvider for ProviderTransformationAdapter<'a> {
    fn transform_v1_to_v2(&self, _entry_type: &str, data: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.transformer.transform_v1_to_v2(data)
    }
}

/// Adapter that implements ReferenceResolutionProvider using provider traits
struct ProviderReferenceAdapter<'a> {
    resolver: &'a dyn crate::entry_type_provider::ReferenceResolver,
}

impl<'a> ReferenceResolutionProvider for ProviderReferenceAdapter<'a> {
    fn resolve_reference(&self, _entry_type: &str, id: &str) -> Result<bool, String> {
        self.resolver.resolve_reference("", id) // entry_type would need to be passed differently
    }
}

/// Main orchestrator that coordinates healing across all entry types
///
/// Instead of hard-coding logic, it:
/// 1. Looks up the entry type in the registry
/// 2. Uses the configured healing strategy
/// 3. Delegates to the provider for type-specific logic
pub struct FlexibleOrchestrator {
    config: OrchestratorConfig,
    registry: EntryTypeRegistry,
}

impl FlexibleOrchestrator {
    /// Create a new orchestrator with configuration and registry
    pub fn new(config: OrchestratorConfig, registry: EntryTypeRegistry) -> Self {
        Self { config, registry }
    }

    /// Check if v1 DNA is available on startup
    ///
    /// Returns Ok(Some(true)) if v1 is available with data
    /// Returns Ok(Some(false)) if v1 is available but empty
    /// Returns Ok(None) if v1 is not available
    pub fn check_v1_availability(&self) -> Result<Option<bool>, String> {
        // In a real implementation, this would try to call v1's is_data_present() function
        // For now, this is a placeholder
        Ok(None)
    }

    /// Heal a specific entry by ID
    ///
    /// Returns the healed entry if successful, with metadata about the healing
    pub fn heal_by_id(
        &self,
        entry_type: &str,
        entry_id: &str,
        v2_entry: Option<Vec<u8>>,
    ) -> Result<Option<HealingOutcome>, String> {
        // 1. Check if we have a provider for this entry type
        let provider = self
            .registry
            .get(entry_type)
            .ok_or_else(|| format!("No provider registered for entry type '{}'", entry_type))?;

        // 2. Create healing context with provider's tools
        let validator = provider.validator();
        let transformer = provider.transformer();
        let resolver = provider.reference_resolver();

        let validation_adapter = ProviderValidationAdapter { validator };
        let transformation_adapter = ProviderTransformationAdapter { transformer };
        let reference_adapter = ProviderReferenceAdapter { resolver };

        let context = HealingContext {
            validator: &validation_adapter,
            transformer: &transformation_adapter,
            reference_resolver: &reference_adapter,
            v1_bridge_caller: None, // Would be populated from v1 bridge if available
            max_attempts: self.config.max_attempts,
            allow_degradation: self.config.allow_degradation,
        };

        // 3. Use the healing strategy to attempt healing
        let healing_result = self
            .config
            .healing_strategy
            .heal(entry_type, entry_id, v2_entry, &context)?;

        // 4. Convert strategy result to outcome
        let outcome = healing_result.map(|result| HealingOutcome {
            entry_id: entry_id.to_string(),
            entry_type: entry_type.to_string(),
            healed_entry: result.entry,
            was_migrated: result.was_migrated,
            attempts: result.attempts,
            notes: result.notes,
            strategy_used: self.config.healing_strategy.description().to_string(),
        });

        Ok(outcome)
    }

    /// List all entry types this orchestrator can heal
    pub fn list_supported_entry_types(&self) -> Vec<String> {
        self.registry.list_entry_types()
    }

    /// Check if a specific entry type is supported
    pub fn supports_entry_type(&self, entry_type: &str) -> bool {
        self.registry.has(entry_type)
    }

    /// Get a description of the healing strategy being used
    pub fn healing_strategy_description(&self) -> &str {
        self.config.healing_strategy.description()
    }
}

/// Outcome of a healing attempt
#[derive(Debug, Clone)]
pub struct HealingOutcome {
    pub entry_id: String,
    pub entry_type: String,
    pub healed_entry: Option<Vec<u8>>,
    pub was_migrated: bool,
    pub attempts: u32,
    pub notes: Vec<String>,
    pub strategy_used: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let config = OrchestratorConfig::default();
        let registry = EntryTypeRegistry::new();
        let _orchestrator = FlexibleOrchestrator::new(config, registry);
    }

    #[test]
    fn test_orchestrator_empty_registry() {
        let config = OrchestratorConfig::default();
        let registry = EntryTypeRegistry::new();
        let orchestrator = FlexibleOrchestrator::new(config, registry);

        assert_eq!(orchestrator.list_supported_entry_types().len(), 0);
        assert!(!orchestrator.supports_entry_type("content"));
    }

    #[test]
    fn test_orchestrator_healing_unknown_type() {
        let config = OrchestratorConfig::default();
        let registry = EntryTypeRegistry::new();
        let orchestrator = FlexibleOrchestrator::new(config, registry);

        let result = orchestrator.heal_by_id("unknown", "id-1", None);
        assert!(result.is_err());
    }
}
