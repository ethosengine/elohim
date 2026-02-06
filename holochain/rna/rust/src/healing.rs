//! Self-healing DNA core types and validation framework
//!
//! # Living DNA Pattern
//!
//! Instead of external migration (export → transform → import),
//! the DNA heals itself continuously:
//!
//! 1. **Entry Versioning**: Every entry knows its schema version
//! 2. **Validation Status**: Valid, Migrated, Degraded, or Healing
//! 3. **Lazy Healing**: On read, missing/broken data triggers healing
//! 4. **Background Healing**: Continuous async repair tasks
//! 5. **Bridge Fallback**: Can always reach v1 for source data
//!
//! # Usage
//!
//! ```rust,ignore
//! use hc_rna::{ValidationStatus, SelfHealingEntry};
//!
//! // Every entry includes validation status
//! let content = Content {
//!     id: "content-123".to_string(),
//!     title: "My Content".to_string(),
//!     validation_status: ValidationStatus::Valid,
//!     schema_version: 2,
//!     ..Default::default()
//! };
//!
//! // On read, check validation
//! match content.validate() {
//!     Ok(_) => println!("Content is healthy"),
//!     Err(e) => println!("Content is degraded: {}", e),
//! }
//! ```

use hdk::prelude::*;
use std::collections::HashMap;

/// The health status of an entry as it moves through the system
///
/// # States
///
/// - **Valid**: Fully validated against current schema, references are good
/// - **Migrated**: Successfully migrated from v1, has been validated
/// - **Degraded**: Missing references or validation failed, but still accessible
/// - **Healing**: Currently being repaired by background tasks
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Entry is valid and fully consistent
    Valid,
    /// Entry was migrated from v1, validated and in good state
    Migrated,
    /// Entry has issues (missing refs, validation failed) but is still accessible
    Degraded,
    /// Entry is currently being healed by a background task
    Healing,
}

impl std::fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid => write!(f, "Valid"),
            Self::Migrated => write!(f, "Migrated"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Healing => write!(f, "Healing"),
        }
    }
}

impl ValidationStatus {
    /// Whether this status indicates the entry is usable
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Valid | Self::Migrated | Self::Degraded | Self::Healing)
    }

    /// Whether this status indicates the entry needs healing
    pub fn needs_healing(&self) -> bool {
        matches!(self, Self::Degraded | Self::Healing)
    }
}

/// A validation rule that can be applied to an entry
///
/// Implement this to define custom validation logic for your entry types.
///
/// # Example
///
/// ```rust,ignore
/// struct ContentValidator;
///
/// impl ValidationRule for ContentValidator {
///     fn validate(&self, data: &serde_json::Value) -> Result<(), String> {
///         // Check required fields
///         if !data.get("title").is_some() {
///             return Err("Missing title".to_string());
///         }
///
///         // Check reference integrity
///         if let Some(parent_id) = data.get("parent_id") {
///             if !entry_exists(parent_id.as_str()?)? {
///                 return Err(format!("Parent {} not found", parent_id));
///             }
///         }
///
///         Ok(())
///     }
/// }
/// ```
pub trait ValidationRule {
    /// Validate the entry data
    ///
    /// Return Ok(()) if valid, Err(message) if validation failed
    fn validate(&self, data: &serde_json::Value) -> Result<(), String>;
}

/// A simple validation rule that always passes
pub struct AcceptAllValidator;

impl ValidationRule for AcceptAllValidator {
    fn validate(&self, _: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }
}

/// Signals emitted during the healing process
///
/// Apps can subscribe to these to show healing progress in the UI
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HealingSignal {
    /// A new degraded entry was discovered
    DegradedEntryFound {
        entry_id: String,
        entry_type: String,
        reason: String,
    },
    /// Healing started for an entry
    HealingStarted {
        entry_id: String,
        attempt: u32,
    },
    /// An entry was successfully healed
    HealingSucceeded {
        entry_id: String,
        entry_type: String,
        was_migrated_from_v1: bool,
    },
    /// Healing failed for an entry, will retry
    HealingRetrying {
        entry_id: String,
        reason: String,
        next_attempt_in_seconds: u32,
    },
    /// Healing permanently failed for an entry
    HealingFailed {
        entry_id: String,
        entry_type: String,
        final_error: String,
    },
    /// A batch of entries was healed
    HealingBatchComplete {
        entry_type: String,
        total_found: u32,
        healed: u32,
        failed: u32,
    },
    /// System has fully healed (no more degraded entries)
    SystemFullyHealed {
        time_taken_seconds: u64,
        total_entries_healed: u32,
    },
}

impl HealingSignal {
    /// Get a human-readable description of this signal
    pub fn description(&self) -> String {
        match self {
            Self::DegradedEntryFound { entry_id, reason, .. } => {
                format!("Entry {} is degraded: {}", entry_id, reason)
            }
            Self::HealingStarted { entry_id, attempt } => {
                format!("Healing entry {} (attempt {})", entry_id, attempt)
            }
            Self::HealingSucceeded { entry_id, entry_type, was_migrated_from_v1 } => {
                if *was_migrated_from_v1 {
                    format!("{} {} migrated and healed", entry_type, entry_id)
                } else {
                    format!("{} {} repaired", entry_type, entry_id)
                }
            }
            Self::HealingRetrying { entry_id, reason, .. } => {
                format!("Retrying {} due to: {}", entry_id, reason)
            }
            Self::HealingFailed { entry_id, final_error, .. } => {
                format!("Failed to heal {}: {}", entry_id, final_error)
            }
            Self::HealingBatchComplete { entry_type, healed, .. } => {
                format!("Healed {} {} entries", healed, entry_type)
            }
            Self::SystemFullyHealed { total_entries_healed, time_taken_seconds } => {
                format!(
                    "System fully healed: {} entries in {} seconds",
                    total_entries_healed, time_taken_seconds
                )
            }
        }
    }
}

/// Metadata about when and how an entry was healed
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealingMetadata {
    /// When was this entry healed (timestamp)
    pub healed_at: Option<u64>,
    /// How many times has healing been attempted
    pub healing_attempts: u32,
    /// Last error encountered during healing
    pub last_healing_error: Option<String>,
    /// Was this entry migrated from v1?
    pub migrated_from_v1: bool,
}

impl Default for HealingMetadata {
    fn default() -> Self {
        Self {
            healed_at: None,
            healing_attempts: 0,
            last_healing_error: None,
            migrated_from_v1: false,
        }
    }
}

/// Result of a healing operation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HealingResult {
    /// No v1 bridge available, this is a fresh DNA
    NoV1Bridge,
    /// v1 has no data, nothing to heal
    NoV1Data,
    /// Data was successfully healed from v1
    DataHealed {
        entry_id: String,
        entry_type: String,
    },
    /// Entry already exists and is valid
    AlreadyValid {
        entry_id: String,
    },
    /// Healing needed but not done yet (will be done in background)
    HealingNeeded {
        count: u32,
    },
    /// Healing failed, entry remains degraded
    HealingFailed {
        entry_id: String,
        reason: String,
    },
}

/// Report from a healing operation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealingReport {
    /// When healing started
    pub started_at: u64,
    /// When healing completed (None if still running)
    pub completed_at: Option<u64>,
    /// Per-entry-type healing counts
    pub entry_healing_counts: HashMap<String, HealingCounts>,
    /// Signals emitted during healing
    pub signals: Vec<HealingSignal>,
    /// Any errors encountered
    pub errors: Vec<String>,
}

/// Healing counts for a single entry type
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HealingCounts {
    /// Total degraded entries found
    pub found: u32,
    /// Successfully healed
    pub healed: u32,
    /// Failed to heal
    pub failed: u32,
    /// Healing in progress
    pub in_progress: u32,
}

impl HealingReport {
    /// Create a new healing report
    pub fn new() -> Self {
        Self {
            started_at: sys_time().ok().map(|t| t.as_millis() as u64).unwrap_or(0),
            completed_at: None,
            entry_healing_counts: HashMap::new(),
            signals: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Record that an entry type was found to be degraded
    pub fn record_found(&mut self, entry_type: &str) {
        let counts = self.entry_healing_counts.entry(entry_type.to_string()).or_default();
        counts.found += 1;
    }

    /// Record that an entry was successfully healed
    pub fn record_healed(&mut self, entry_type: &str) {
        let counts = self.entry_healing_counts.entry(entry_type.to_string()).or_default();
        counts.healed += 1;
    }

    /// Record that healing failed for an entry
    pub fn record_failed(&mut self, entry_type: &str) {
        let counts = self.entry_healing_counts.entry(entry_type.to_string()).or_default();
        counts.failed += 1;
    }

    /// Add a signal
    pub fn emit_signal(&mut self, signal: HealingSignal) {
        self.signals.push(signal);
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Mark healing as complete
    pub fn complete(&mut self) {
        self.completed_at = Some(sys_time().ok().map(|t| t.as_millis() as u64).unwrap_or(0));
    }

    /// Get total entries that needed healing
    pub fn total_found(&self) -> u32 {
        self.entry_healing_counts.values().map(|c| c.found).sum()
    }

    /// Get total entries healed
    pub fn total_healed(&self) -> u32 {
        self.entry_healing_counts.values().map(|c| c.healed).sum()
    }

    /// Get total entries that failed to heal
    pub fn total_failed(&self) -> u32 {
        self.entry_healing_counts.values().map(|c| c.failed).sum()
    }

    /// Whether healing was successful (no failures)
    pub fn is_success(&self) -> bool {
        self.total_failed() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_status_is_usable() {
        assert!(ValidationStatus::Valid.is_usable());
        assert!(ValidationStatus::Migrated.is_usable());
        assert!(ValidationStatus::Degraded.is_usable());
        assert!(ValidationStatus::Healing.is_usable());
    }

    #[test]
    fn test_validation_status_needs_healing() {
        assert!(!ValidationStatus::Valid.needs_healing());
        assert!(!ValidationStatus::Migrated.needs_healing());
        assert!(ValidationStatus::Degraded.needs_healing());
        assert!(ValidationStatus::Healing.needs_healing());
    }

    #[test]
    fn test_healing_report() {
        let mut report = HealingReport::new();

        report.record_found("Content");
        report.record_found("Content");
        report.record_healed("Content");
        report.record_failed("Content");

        assert_eq!(report.total_found(), 2);
        assert_eq!(report.total_healed(), 1);
        assert_eq!(report.total_failed(), 1);
        assert!(!report.is_success());
    }

    #[test]
    fn test_healing_signal_description() {
        let signal = HealingSignal::DegradedEntryFound {
            entry_id: "entry-1".to_string(),
            entry_type: "Content".to_string(),
            reason: "Missing reference".to_string(),
        };

        assert!(signal.description().contains("degraded"));
    }
}
