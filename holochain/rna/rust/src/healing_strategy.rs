//! Pluggable Healing Strategies
//!
//! Different applications may need different healing approaches:
//! - Some may want to always try the v1 bridge first
//! - Some may prefer local self-repair first
//! - Some may try multiple strategies in sequence
//! - Some may use custom strategies entirely
//!
//! This module defines the strategy pattern so healing behavior can be
//! configured at startup without modifying the RNA framework.

use serde_json::Value;

/// Result of a healing attempt
#[derive(Debug, Clone)]
pub struct HealingResult<T> {
    /// The healed entry if successful
    pub entry: Option<T>,
    /// Whether this came from v1 (true) or local repair (false)
    pub was_migrated: bool,
    /// How many healing attempts were made
    pub attempts: u32,
    /// Any warnings or notes about the healing
    pub notes: Vec<String>,
}

/// Decides how to attempt healing for a specific entry type
pub trait HealingStrategy: Send + Sync {
    /// Attempt to heal an entry of a given type
    ///
    /// # Arguments
    /// * `entry_type` - Type of entry (e.g., "content", "learning_path")
    /// * `entry_id` - ID of the entry to heal
    /// * `v2_entry` - Current v2 entry if it exists (may be degraded)
    /// * `context` - Context containing bridge and other healing tools
    ///
    /// # Returns
    /// Option<Vec<u8>> - Healed entry as bytes, or None if healing failed
    fn heal(
        &self,
        entry_type: &str,
        entry_id: &str,
        v2_entry: Option<Vec<u8>>,
        context: &HealingContext<'_>,
    ) -> Result<Option<HealingResult<Vec<u8>>>, String>;

    /// Description of this strategy for logging
    fn description(&self) -> &str;
}

/// Interface for entry validation
pub trait ValidationProvider: Send + Sync {
    fn validate_json(&self, entry_type: &str, data: &Value) -> Result<(), String>;
}

/// Interface for entry transformation
pub trait TransformationProvider: Send + Sync {
    fn transform_v1_to_v2(&self, entry_type: &str, data: &Value) -> Result<Value, String>;
}

/// Interface for reference resolution
pub trait ReferenceResolutionProvider: Send + Sync {
    fn resolve_reference(&self, entry_type: &str, id: &str) -> Result<bool, String>;
}

/// Context provided to healing strategies
///
/// This gives strategies everything they need to perform healing without
/// being tightly coupled to specific implementations.
pub struct HealingContext<'a> {
    /// Provider for entry validation
    pub validator: &'a dyn ValidationProvider,

    /// Provider for v1 to v2 transformation
    pub transformer: &'a dyn TransformationProvider,

    /// Provider for reference resolution
    pub reference_resolver: &'a dyn ReferenceResolutionProvider,

    /// Function to call the v1 bridge (if available)
    pub v1_bridge_caller: Option<&'a dyn Fn(&str, &str, Value) -> Result<Value, String>>,

    /// Maximum number of healing attempts
    pub max_attempts: u32,

    /// Whether to mark entries that fail validation as Degraded or fail completely
    pub allow_degradation: bool,
}

/// Try v1 bridge first, fall back to self-repair
pub struct BridgeFirstStrategy;

impl HealingStrategy for BridgeFirstStrategy {
    fn heal(
        &self,
        entry_type: &str,
        entry_id: &str,
        v2_entry: Option<Vec<u8>>,
        context: &HealingContext<'_>,
    ) -> Result<Option<HealingResult<Vec<u8>>>, String> {
        let mut result = HealingResult {
            entry: None,
            was_migrated: false,
            attempts: 0,
            notes: Vec::new(),
        };

        // 1. Try v1 bridge
        if let Some(bridge_caller) = context.v1_bridge_caller {
            result.attempts += 1;

            match bridge_caller(
                entry_type,
                entry_id,
                serde_json::json!({"id": entry_id}),
            ) {
                Ok(v1_data) => {
                    result.notes.push("Retrieved from v1 bridge".to_string());

                    // Validate v1 data
                    if let Err(e) = context.validator.validate_json(entry_type, &v1_data) {
                        result.notes.push(format!("V1 data validation failed: {}", e));
                        if !context.allow_degradation {
                            return Err(e);
                        }
                    }

                    // Transform to v2
                    match context.transformer.transform_v1_to_v2(entry_type, &v1_data) {
                        Ok(v2_data) => {
                            result.entry = Some(v2_data.to_string().into_bytes());
                            result.was_migrated = true;
                            result.notes.push("Successfully transformed to v2".to_string());
                            return Ok(Some(result));
                        }
                        Err(e) => {
                            result.notes.push(format!("Transformation failed: {}", e));
                            if !context.allow_degradation {
                                return Err(e);
                            }
                        }
                    }
                }
                Err(_) => {
                    result.notes.push("V1 bridge not available or entry not found".to_string());
                }
            }
        }

        // 2. Try self-repair on v2 entry if it exists
        if let Some(v2_data) = v2_entry {
            result.attempts += 1;
            result.notes.push("Attempting self-repair on v2 entry".to_string());

            // Try to parse and validate
            if let Ok(v2_json) = serde_json::from_slice::<Value>(&v2_data) {
                if context.validator.validate_json(entry_type, &v2_json).is_ok() {
                    result.entry = Some(v2_data);
                    result.notes.push("V2 entry self-repaired successfully".to_string());
                    return Ok(Some(result));
                }
            }
        }

        // 3. If we're here, healing failed
        Ok(None)
    }

    fn description(&self) -> &str {
        "Try v1 bridge first, fall back to v2 self-repair"
    }
}

/// Try self-repair first, only use v1 bridge as fallback
pub struct SelfRepairFirstStrategy;

impl HealingStrategy for SelfRepairFirstStrategy {
    fn heal(
        &self,
        entry_type: &str,
        entry_id: &str,
        v2_entry: Option<Vec<u8>>,
        context: &HealingContext<'_>,
    ) -> Result<Option<HealingResult<Vec<u8>>>, String> {
        let mut result = HealingResult {
            entry: None,
            was_migrated: false,
            attempts: 0,
            notes: Vec::new(),
        };

        // 1. Try self-repair first
        if let Some(v2_data) = v2_entry {
            result.attempts += 1;
            result.notes.push("Attempting self-repair on v2 entry".to_string());

            if let Ok(v2_json) = serde_json::from_slice::<Value>(&v2_data) {
                if context.validator.validate_json(entry_type, &v2_json).is_ok() {
                    result.entry = Some(v2_data);
                    result.notes.push("V2 entry self-repaired successfully".to_string());
                    return Ok(Some(result));
                }
            }
        }

        // 2. Fall back to v1 bridge
        if let Some(ref bridge_caller) = context.v1_bridge_caller {
            result.attempts += 1;
            result.notes.push("Falling back to v1 bridge".to_string());

            match bridge_caller(
                entry_type,
                entry_id,
                serde_json::json!({"id": entry_id}),
            ) {
                Ok(v1_data) => {
                    if context.validator.validate_json(entry_type, &v1_data).is_ok() {
                        match context.transformer.transform_v1_to_v2(entry_type, &v1_data) {
                            Ok(v2_data) => {
                                result.entry = Some(v2_data.to_string().into_bytes());
                                result.was_migrated = true;
                                result.notes.push("Successfully healed from v1".to_string());
                                return Ok(Some(result));
                            }
                            Err(e) => {
                                result.notes.push(format!("Transformation failed: {}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    result.notes.push(format!("V1 bridge failed: {}", e));
                }
            }
        }

        Ok(None)
    }

    fn description(&self) -> &str {
        "Try self-repair first, fall back to v1 bridge"
    }
}

/// Never use v1 bridge, only local self-repair
pub struct LocalRepairOnlyStrategy;

impl HealingStrategy for LocalRepairOnlyStrategy {
    fn heal(
        &self,
        entry_type: &str,
        _entry_id: &str,
        v2_entry: Option<Vec<u8>>,
        context: &HealingContext<'_>,
    ) -> Result<Option<HealingResult<Vec<u8>>>, String> {
        let mut result = HealingResult {
            entry: None,
            was_migrated: false,
            attempts: 0,
            notes: Vec::new(),
        };

        if let Some(v2_data) = v2_entry {
            result.attempts += 1;

            if let Ok(v2_json) = serde_json::from_slice::<Value>(&v2_data) {
                if context.validator.validate_json(entry_type, &v2_json).is_ok() {
                    result.entry = Some(v2_data);
                    result.notes.push("Repaired locally".to_string());
                    return Ok(Some(result));
                } else if context.allow_degradation {
                    result.entry = Some(v2_data);
                    result.notes.push("Marked as degraded, local repair incomplete".to_string());
                    return Ok(Some(result));
                }
            }
        }

        Ok(None)
    }

    fn description(&self) -> &str {
        "Local self-repair only, no v1 bridge"
    }
}

/// Accept any v2 entry as-is, never attempt healing
pub struct NoHealingStrategy;

impl HealingStrategy for NoHealingStrategy {
    fn heal(
        &self,
        _entry_type: &str,
        _entry_id: &str,
        v2_entry: Option<Vec<u8>>,
        _context: &HealingContext<'_>,
    ) -> Result<Option<HealingResult<Vec<u8>>>, String> {
        let result = v2_entry.map(|entry| HealingResult {
            entry: Some(entry),
            was_migrated: false,
            attempts: 0,
            notes: vec!["No healing attempted, accepted v2 entry as-is".to_string()],
        });

        Ok(result)
    }

    fn description(&self) -> &str {
        "No healing, accept v2 entries as-is"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_context() -> HealingContext {
        HealingContext {
            v1_bridge_caller: None,
            validator: Box::new(|_et, _data| Ok(())),
            transformer: Box::new(|_et, data| Ok(data.clone())),
            reference_resolver: Box::new(|_et, _id| Ok(true)),
            max_attempts: 3,
            allow_degradation: true,
        }
    }

    #[test]
    fn test_bridge_first_with_no_v1() {
        let strategy = BridgeFirstStrategy;
        let context = mock_context();

        let result = strategy.heal("content", "id-1", None, &context).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_self_repair_first_with_v2_entry() {
        let strategy = SelfRepairFirstStrategy;
        let context = mock_context();
        let v2_entry = Some(b"{}".to_vec());

        let result = strategy
            .heal("content", "id-1", v2_entry, &context)
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_no_healing_strategy() {
        let strategy = NoHealingStrategy;
        let context = mock_context();
        let v2_entry = Some(b"{}".to_vec());

        let result = strategy
            .heal("content", "id-1", v2_entry, &context)
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().attempts, 0);
    }
}
