//! Self-healing entry trait that applications can implement
//!
//! # Pattern
//!
//! Any entry type that needs to survive schema evolution should implement
//! this trait. It enables:
//!
//! - Version tracking (which schema version is this entry?)
//! - Status tracking (is it valid, degraded, healing?)
//! - Self-validation (can it validate itself?)
//! - Healing awareness (can it heal itself from v1?)
//!
//! # Example
//!
//! ```rust,ignore
//! use hc_rna::{SelfHealingEntry, ValidationStatus};
//!
//! #[hdk_entry(type = "content_v2")]
//! #[derive(Clone)]
//! pub struct Content {
//!     pub id: String,
//!     pub title: String,
//!     pub schema_version: u32,
//!     pub validation_status: ValidationStatus,
//!     pub parent_id: Option<String>,
//! }
//!
//! impl SelfHealingEntry for Content {
//!     fn schema_version(&self) -> u32 {
//!         self.schema_version
//!     }
//!
//!     fn validation_status(&self) -> ValidationStatus {
//!         self.validation_status
//!     }
//!
//!     fn set_validation_status(&mut self, status: ValidationStatus) {
//!         self.validation_status = status;
//!     }
//!
//!     fn entry_id(&self) -> String {
//!         self.id.clone()
//!     }
//!
//!     fn validate(&self) -> Result<(), String> {
//!         // Check required fields
//!         if self.title.is_empty() {
//!             return Err("Title is required".to_string());
//!         }
//!
//!         // Check reference integrity
//!         if let Some(parent_id) = &self.parent_id {
//!             match get_content_by_id(parent_id) {
//!                 Ok(Some(_)) => {},
//!                 Ok(None) => return Err(format!("Parent {} not found", parent_id)),
//!                 Err(e) => return Err(format!("Error checking parent: {:?}", e)),
//!             }
//!         }
//!
//!         Ok(())
//!     }
//!
//!     fn set_healed_at(&mut self, timestamp: u64) {
//!         // Store timestamp if you have a field for it
//!         // self.healed_at = Some(timestamp);
//!     }
//! }
//! ```

use crate::healing::ValidationStatus;
use hdk::prelude::*;

/// Trait that any entry can implement to support self-healing schema evolution
///
/// This enables the entry to:
/// - Declare its schema version
/// - Track its validation status
/// - Validate itself against current expectations
/// - Participate in healing workflows
///
/// Implement this trait for any entry type that needs to survive
/// schema changes and support migration from previous versions.
pub trait SelfHealingEntry: Serialize + serde::de::DeserializeOwned {
    /// Get this entry's schema version
    ///
    /// Use a numeric version (1, 2, 3...) to track schema evolution.
    /// When you add/remove/change fields, increment this.
    fn schema_version(&self) -> u32;

    /// Get this entry's current validation status
    fn validation_status(&self) -> ValidationStatus;

    /// Set this entry's validation status
    fn set_validation_status(&mut self, status: ValidationStatus);

    /// Get a unique identifier for this entry
    ///
    /// This should be a stable ID that survives schema changes
    /// (e.g., a content ID, not an action hash)
    fn entry_id(&self) -> String;

    /// Validate this entry against current schema expectations
    ///
    /// This should check:
    /// - Required fields are present
    /// - Type constraints are met
    /// - References to other entries exist
    /// - Derived fields are consistent
    ///
    /// # Returns
    ///
    /// - `Ok(())` if entry is valid
    /// - `Err(message)` if validation fails (don't panic)
    fn validate(&self) -> Result<(), String>;

    /// Called when this entry has been successfully healed
    ///
    /// Store healing timestamp or other metadata if desired.
    /// Default implementation does nothing.
    fn set_healed_at(&mut self, _timestamp: u64) {}

    /// Called when this entry's schema version doesn't match current
    ///
    /// Default implementation marks it as Degraded.
    /// Override to handle version-specific healing.
    fn handle_version_mismatch(&mut self, current_version: u32) -> Result<(), String> {
        if self.schema_version() != current_version {
            self.set_validation_status(ValidationStatus::Degraded);
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Validate and fix this entry in-place if possible
    ///
    /// The default implementation just calls validate().
    /// Override if your entry can auto-correct issues.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if entry was modified during healing
    /// - `Ok(false)` if entry was already valid
    /// - `Err(message)` if healing failed and entry is still broken
    fn try_self_heal(&mut self) -> Result<bool, String> {
        self.validate()?;
        Ok(false)
    }
}

/// A wrapper around a self-healing entry with healing metadata
#[derive(Serialize, Debug, Clone)]
pub struct HealedEntry<T: SelfHealingEntry + Serialize> {
    /// The actual entry data
    pub entry: T,
    /// When was this healed (unix timestamp)
    pub healed_at: Option<u64>,
    /// How many healing attempts were made
    pub healing_attempts: u32,
    /// The validation status at the time of healing
    pub final_status: ValidationStatus,
}

impl<T: SelfHealingEntry> HealedEntry<T> {
    /// Create a new healed entry wrapper
    pub fn new(entry: T) -> Self {
        let final_status = entry.validation_status();
        Self {
            entry,
            healed_at: None,
            healing_attempts: 0,
            final_status,
        }
    }

    /// Mark this entry as healed
    pub fn mark_healed(&mut self) {
        // Use sys_time() in Holochain context, fall back to 0 in tests
        let now = sys_time().ok().map(|t| t.as_millis() as u64).unwrap_or(0);
        self.healed_at = Some(now);
        self.final_status = self.entry.validation_status();
    }

    /// Record a healing attempt
    pub fn record_healing_attempt(&mut self) {
        self.healing_attempts += 1;
    }

    /// Get the inner entry
    pub fn into_entry(self) -> T {
        self.entry
    }

    /// Get a reference to the entry
    pub fn as_entry(&self) -> &T {
        &self.entry
    }

    /// Get a mutable reference to the entry
    pub fn as_entry_mut(&mut self) -> &mut T {
        &mut self.entry
    }
}

/// A validation result with detailed information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// If validation failed, the error message
    pub error: Option<String>,
    /// What status should the entry have
    pub suggested_status: ValidationStatus,
}

impl ValidationResult {
    /// Validation passed
    pub fn passed() -> Self {
        Self {
            passed: true,
            error: None,
            suggested_status: ValidationStatus::Valid,
        }
    }

    /// Validation failed with this error
    pub fn failed(error: String, suggested_status: ValidationStatus) -> Self {
        Self {
            passed: false,
            error: Some(error),
            suggested_status,
        }
    }

    /// Validation failed, suggest Degraded status
    pub fn degraded(error: String) -> Self {
        Self::failed(error, ValidationStatus::Degraded)
    }
}

/// Helper to validate a batch of entries
pub struct BatchValidator<T: SelfHealingEntry> {
    entries: Vec<T>,
    results: Vec<ValidationResult>,
}

impl<T: SelfHealingEntry> BatchValidator<T> {
    /// Create a new batch validator
    pub fn new(entries: Vec<T>) -> Self {
        Self {
            entries,
            results: Vec::new(),
        }
    }

    /// Validate all entries
    pub fn validate_all(&mut self) -> Vec<ValidationResult> {
        self.results = self.entries
            .iter()
            .map(|entry| {
                match entry.validate() {
                    Ok(_) => ValidationResult::passed(),
                    Err(e) => ValidationResult::degraded(e),
                }
            })
            .collect();

        self.results.clone()
    }

    /// Get validation results
    pub fn results(&self) -> &[ValidationResult] {
        &self.results
    }

    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.results.is_empty() {
            return 1.0;
        }

        let passed = self.results.iter().filter(|r| r.passed).count();
        passed as f32 / self.results.len() as f32
    }

    /// Get entries that failed validation
    pub fn failed_entries(&self) -> Vec<(usize, &T, &ValidationResult)> {
        self.entries
            .iter()
            .zip(self.results.iter())
            .enumerate()
            .filter(|(_, (_, result))| !result.passed)
            .map(|(idx, (entry, result))| (idx, entry, result))
            .collect()
    }

    /// Consume and get entries
    pub fn into_entries(self) -> Vec<T> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock entry for testing
    #[derive(Clone, Serialize, Deserialize)]
    struct MockEntry {
        id: String,
        schema_version: u32,
        validation_status: ValidationStatus,
    }

    impl SelfHealingEntry for MockEntry {
        fn schema_version(&self) -> u32 {
            self.schema_version
        }

        fn validation_status(&self) -> ValidationStatus {
            self.validation_status
        }

        fn set_validation_status(&mut self, status: ValidationStatus) {
            self.validation_status = status;
        }

        fn entry_id(&self) -> String {
            self.id.clone()
        }

        fn validate(&self) -> Result<(), String> {
            if self.id.is_empty() {
                Err("ID is required".to_string())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_healed_entry_wrapping() {
        let entry = MockEntry {
            id: "test-1".to_string(),
            schema_version: 2,
            validation_status: ValidationStatus::Valid,
        };

        let mut healed = HealedEntry::new(entry);
        healed.record_healing_attempt();
        healed.mark_healed();

        assert_eq!(healed.healing_attempts, 1);
        assert!(healed.healed_at.is_some());
    }

    #[test]
    fn test_batch_validator() {
        let entries = vec![
            MockEntry {
                id: "valid".to_string(),
                schema_version: 2,
                validation_status: ValidationStatus::Valid,
            },
            MockEntry {
                id: "".to_string(),
                schema_version: 2,
                validation_status: ValidationStatus::Degraded,
            },
        ];

        let mut validator = BatchValidator::new(entries);
        let results = validator.validate_all();

        assert_eq!(results.len(), 2);
        assert!(results[0].passed);
        assert!(!results[1].passed);
        assert!(validator.success_rate() < 1.0);
    }

    #[test]
    fn test_validation_result() {
        let passed = ValidationResult::passed();
        assert!(passed.passed);

        let failed = ValidationResult::degraded("Test error".to_string());
        assert!(!failed.passed);
        assert_eq!(failed.suggested_status, ValidationStatus::Degraded);
    }
}
