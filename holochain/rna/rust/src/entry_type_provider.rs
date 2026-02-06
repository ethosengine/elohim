//! Entry Type Provider System for Flexible Migration
//!
//! This module defines the trait-based system that allows different entry types
//! to be registered with their own healing, validation, and transformation logic
//! without modifying the core RNA framework.
//!
//! Key principle: Entry types are registered at startup, not implemented in the
//! framework. This allows different apps to support different entry types and
//! healing strategies without touching the RNA module.

use serde_json::Value;
use std::any::Any;

/// Represents an entry that can participate in self-healing migration
pub trait HealableEntry: Send + Sync {
    /// Get this entry's unique identifier (stable across schema changes)
    fn entry_id(&self) -> String;

    /// Get the schema version this entry conforms to
    fn schema_version(&self) -> u32;

    /// Get this entry's validation status
    fn validation_status(&self) -> crate::ValidationStatus;

    /// Set validation status (mutable operation)
    fn set_validation_status(&mut self, status: crate::ValidationStatus);

    /// Record when this entry was healed
    fn set_healed_at(&mut self, timestamp: u64);

    /// Validate this entry against current schema expectations
    fn validate(&self) -> Result<(), String>;

    /// Attempt self-repair if possible (e.g., normalize fields)
    fn try_self_repair(&mut self) -> Result<bool, String> {
        self.validate()?;
        Ok(false) // Default: no repair
    }

    /// Convert to JSON for serialization (needed for bridge calls)
    fn to_json(&self) -> Result<Value, String>;

    /// Downcast to concrete type for type-specific operations
    fn as_any(&self) -> &dyn Any;
}

/// Validates entries according to schema and business rules
pub trait Validator: Send + Sync {
    /// Validate an entry in JSON form (from v1 bridge)
    fn validate_json(&self, data: &Value) -> Result<(), String>;

    /// Validate a concrete entry
    fn validate_entry(&self, entry: &dyn HealableEntry) -> Result<(), String> {
        entry.validate()
    }
}

/// Transcribes entries between DNA versions
///
/// Like biological RNA transcribing genetic information, this trait
/// handles the translation of data between DNA versions in either direction.
pub trait Transcriber: Send + Sync {
    /// Transcribe data from previous DNA version into self (this DNA)
    fn transcribe_from_prev(&self, prev_data: &Value) -> Result<Value, String>;

    /// Transcribe data from self (this DNA) back to previous DNA version
    ///
    /// Used for export, rollback, or compatibility with nodes
    /// that haven't yet upgraded.
    fn transcribe_to_prev(&self, _self_data: &Value) -> Result<Value, String> {
        // Default: not supported (one-way migration)
        Err("Transcription to previous DNA not implemented for this entry type".to_string())
    }

    /// Description of what this transcriber does (for logging/debugging)
    fn description(&self) -> &str;
}

/// Resolves references during healing (checks if linked entries exist)
pub trait ReferenceResolver: Send + Sync {
    /// Check if a reference to another entry exists
    fn resolve_reference(&self, entry_type: &str, id: &str) -> Result<bool, String>;

    /// Get all references from an entry that need resolution
    fn extract_references(&self, _entry: &dyn HealableEntry) -> Result<Vec<(String, String)>, String> {
        Ok(Vec::new()) // Default: no references
    }
}

/// Decides what to do when healing encounters problems
pub trait DegradationHandler: Send + Sync {
    /// Determine if an entry should be marked as Degraded or if healing should fail
    fn handle_validation_failure(
        &self,
        entry_type: &str,
        error: &str,
        was_migrated: bool,
    ) -> DegradationDecision;

    /// Determine if a missing reference should block healing
    fn handle_missing_reference(
        &self,
        entry_type: &str,
        ref_type: &str,
        ref_id: &str,
    ) -> DegradationDecision;
}

/// Decision for what to do when healing encounters problems
#[derive(Debug, Clone, Copy)]
pub enum DegradationDecision {
    /// Mark the entry as Degraded and continue
    Degrade,
    /// Fail the healing attempt completely
    Fail,
    /// Accept the entry despite the issue
    Accept,
}

/// Complete provider for an entry type's healing behavior
pub trait EntryTypeProvider: Send + Sync {
    /// The entry type this provider handles (e.g., "content", "learning_path")
    fn entry_type(&self) -> &str;

    /// Get the validator for this entry type
    fn validator(&self) -> &dyn Validator;

    /// Get the transcriber for this entry type
    fn transcriber(&self) -> &dyn Transcriber;

    /// Get the reference resolver for this entry type
    fn reference_resolver(&self) -> &dyn ReferenceResolver;

    /// Get the degradation handler for this entry type
    fn degradation_handler(&self) -> &dyn DegradationHandler;

    /// Create a healing instance for this entry type
    fn create_healing_instance(&self, id: &str, v1_data: &Value) -> Result<Vec<u8>, String>;
}

/// Registry of all entry type providers
///
/// At startup, the coordinator registers all entry types it supports.
/// During healing, we look up the provider for an entry type and use it.
///
/// This means:
/// - Adding a new entry type requires implementing EntryTypeProvider, not modifying RNA
/// - Changing validation/transformation logic is isolated to one trait implementation
/// - Different apps can support different entry types
/// - Healing strategies are completely pluggable
pub struct EntryTypeRegistry {
    providers: std::collections::HashMap<String, std::sync::Arc<dyn EntryTypeProvider>>,
}

impl EntryTypeRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    /// Register a provider for an entry type
    pub fn register(
        &mut self,
        provider: std::sync::Arc<dyn EntryTypeProvider>,
    ) -> Result<(), String> {
        let entry_type = provider.entry_type().to_string();
        if self.providers.contains_key(&entry_type) {
            return Err(format!("Entry type '{}' already registered", entry_type));
        }
        self.providers.insert(entry_type, provider);
        Ok(())
    }

    /// Get the provider for an entry type
    pub fn get(&self, entry_type: &str) -> Option<std::sync::Arc<dyn EntryTypeProvider>> {
        self.providers.get(entry_type).cloned()
    }

    /// List all registered entry types
    pub fn list_entry_types(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Check if an entry type is registered
    pub fn has(&self, entry_type: &str) -> bool {
        self.providers.contains_key(entry_type)
    }
}

impl Default for EntryTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockValidator;
    impl Validator for MockValidator {
        fn validate_json(&self, _data: &Value) -> Result<(), String> {
            Ok(())
        }
    }

    struct MockTranscriber;
    impl Transcriber for MockTranscriber {
        fn transcribe_from_prev(&self, data: &Value) -> Result<Value, String> {
            Ok(data.clone())
        }

        fn description(&self) -> &str {
            "Mock transcriber"
        }
    }

    struct MockResolver;
    impl ReferenceResolver for MockResolver {
        fn resolve_reference(&self, _entry_type: &str, _id: &str) -> Result<bool, String> {
            Ok(true)
        }
    }

    struct MockDegradationHandler;
    impl DegradationHandler for MockDegradationHandler {
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

    struct MockProvider;
    impl EntryTypeProvider for MockProvider {
        fn entry_type(&self) -> &str {
            "test"
        }

        fn validator(&self) -> &dyn Validator {
            &MockValidator
        }

        fn transcriber(&self) -> &dyn Transcriber {
            &MockTranscriber
        }

        fn reference_resolver(&self) -> &dyn ReferenceResolver {
            &MockResolver
        }

        fn degradation_handler(&self) -> &dyn DegradationHandler {
            &MockDegradationHandler
        }

        fn create_healing_instance(&self, id: &str, _v1_data: &Value) -> Result<Vec<u8>, String> {
            Ok(format!("healed-{}", id).into_bytes())
        }
    }

    #[test]
    fn test_registry_registration() {
        let mut registry = EntryTypeRegistry::new();
        let provider = std::sync::Arc::new(MockProvider);

        assert!(registry.register(provider).is_ok());
        assert!(registry.has("test"));
    }

    #[test]
    fn test_registry_duplicate_registration() {
        let mut registry = EntryTypeRegistry::new();
        let provider = std::sync::Arc::new(MockProvider);

        assert!(registry.register(provider.clone()).is_ok());
        assert!(registry.register(provider).is_err());
    }

    #[test]
    fn test_registry_retrieval() {
        let mut registry = EntryTypeRegistry::new();
        let provider = std::sync::Arc::new(MockProvider);

        registry.register(provider.clone()).unwrap();
        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }
}
